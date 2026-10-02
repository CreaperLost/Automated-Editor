//! Bounded, version-aware telemetry reader.
//!
//! V1 events use a top-level `kind`. V2 disk records use a tagged `payload.kind`,
//! snake_case fields, and omit the FFI-only `record` wrapper. Gap events may omit
//! coordinates and geometry. A truncated final JSONL line is recovered; corrupt
//! interior lines and unknown versions are reported rather than rewritten.
use super::event::{GeometryRecord, TelemetryEvent, TelemetryKind};
use super::native::{MouseEvent, MouseGeometry, MousePayload};
use crate::project::reader::{open_regular, safe_path};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

/// Long recordings with a high-rate mouse reach hundreds of MB of JSONL.
pub const EVENTS_LIMIT: u64 = 1_073_741_824;
pub const GEOMETRY_LIMIT: u64 = 1_048_576;
pub const LINE_LIMIT: u64 = 65_536;
pub const RECORD_LIMIT: usize = 5_000_000;
/// Mouse moves closer together than this are thinned to one sample; zoom and the auto
/// webcam need positions and pauses, not a 1000 Hz trace.
pub const MOVE_THIN_US: u64 = 8_000;
pub const DIAGNOSTIC_LIMIT: usize = 256;
const MAX_SAFE_TIME: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, PartialEq)]
pub struct TelemetryStream {
    pub events: Vec<CanonicalEvent>,
    pub geometries: BTreeMap<String, CanonicalGeometry>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CanonicalEvent {
    pub version: u32,
    pub seq: u64,
    pub t_us: u64,
    pub geometry_id: Option<String>,
    pub kind: CanonicalKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CanonicalKind {
    Move {
        norm_x: f64,
        norm_y: f64,
        inside_source: bool,
    },
    ButtonDown {
        button: Option<u32>,
        norm_x: f64,
        norm_y: f64,
        inside_source: bool,
    },
    ButtonUp {
        button: Option<u32>,
        norm_x: f64,
        norm_y: f64,
        inside_source: bool,
    },
    /// V1 derived click with unknown button provenance. Never invent a matching
    /// down/up pair from this record.
    Click {
        norm_x: f64,
        norm_y: f64,
        inside_source: bool,
    },
    Scroll {
        norm_x: Option<f64>,
        norm_y: Option<f64>,
        inside_source: Option<bool>,
    },
    Gap {
        reason: String,
        start_us: u64,
        end_us: u64,
        dropped_events: u64,
    },
    /// A record that is not mouse activity (cursor shape changes, unknown kinds). Kept so
    /// sequence numbers stay contiguous.
    Note,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CanonicalGeometry {
    pub geometry_id: String,
    pub t_us: u64,
    pub version: u32,
    pub supported: bool,
    pub unsupported_reason: Option<String>,
    pub sampling_interval_us: u64,
    pub source_id: Option<String>,
}

#[derive(Deserialize)]
struct LinePeek {
    version: Option<u32>,
    record: Option<String>,
    kind: Option<serde_json::Value>,
    payload: Option<serde_json::Value>,
    geometry_id: Option<String>,
    coordinate_space: Option<String>,
}

pub fn read_telemetry(root: &Path) -> Result<TelemetryStream, String> {
    let mut stream = TelemetryStream {
        events: Vec::new(),
        geometries: BTreeMap::new(),
        diagnostics: Vec::new(),
    };
    read_geometry_file(root, &mut stream)?;
    read_events_file(root, &mut stream)?;
    Ok(stream)
}

fn push_diag(stream: &mut TelemetryStream, message: String) {
    if stream.diagnostics.len() < DIAGNOSTIC_LIMIT {
        stream.diagnostics.push(message);
    }
}

fn insert_uncertainty_gap(stream: &mut TelemetryStream, reason: &str) {
    let (start_us, seq) = match stream.events.last() {
        Some(previous) => (previous.t_us, previous.seq.saturating_add(1)),
        None => (0, 0),
    };
    if matches!(
        stream.events.last().map(|event| &event.kind),
        Some(CanonicalKind::Gap { reason: last, .. }) if last == reason
    ) {
        return;
    }
    stream.events.push(CanonicalEvent {
        version: 2,
        seq,
        t_us: start_us,
        geometry_id: None,
        kind: CanonicalKind::Gap {
            reason: reason.into(),
            start_us,
            end_us: start_us,
            dropped_events: 1,
        },
    });
}

fn read_geometry_file(root: &Path, stream: &mut TelemetryStream) -> Result<(), String> {
    let path = match safe_path(root, "telemetry/geometry.jsonl") {
        Ok(path) => path,
        Err(error) => {
            push_diag(stream, format!("Telemetry geometry path rejected: {error}"));
            return Ok(());
        }
    };
    if !path.exists() {
        push_diag(stream, "No telemetry geometry file".into());
        return Ok(());
    }
    parse_jsonl(&path, GEOMETRY_LIMIT, stream, parse_geometry_line)
}

fn read_events_file(root: &Path, stream: &mut TelemetryStream) -> Result<(), String> {
    let path = match safe_path(root, "telemetry/events.jsonl") {
        Ok(path) => path,
        Err(error) => {
            push_diag(stream, format!("Telemetry events path rejected: {error}"));
            return Ok(());
        }
    };
    if !path.exists() {
        push_diag(stream, "No telemetry events file".into());
        return Ok(());
    }
    parse_jsonl(&path, EVENTS_LIMIT, stream, parse_event_line)
}

fn parse_jsonl(
    path: &Path,
    limit: u64,
    stream: &mut TelemetryStream,
    mut on_line: impl FnMut(&[u8], usize, &mut TelemetryStream) -> Result<(), String>,
) -> Result<(), String> {
    let file = open_regular(path)?;
    let meta = file.metadata().map_err(|e| e.to_string())?;
    if !meta.is_file() {
        return Err("Telemetry path is not a file".into());
    }
    let mut reader = BufReader::new(file.take(limit + 1));
    let mut total = 0u64;
    for line_number in 0..=RECORD_LIMIT {
        let mut line = Vec::new();
        let count = reader
            .by_ref()
            .take(LINE_LIMIT + 1)
            .read_until(b'\n', &mut line)
            .map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if count as u64 > LINE_LIMIT {
            return Err("Telemetry line exceeds the size limit".into());
        }
        if total > limit || line_number == RECORD_LIMIT {
            // Keep what was read rather than losing the whole recording's telemetry.
            push_diag(
                stream,
                format!("Telemetry is very large; only the first {line_number} records were read"),
            );
            break;
        }
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let had_newline = line.ends_with(b"\n");
        match on_line(&line, line_number + 1, stream) {
            Ok(()) => {}
            Err(error) if !had_newline && is_truncated_json(&error) => {
                push_diag(
                    stream,
                    "Incomplete final telemetry line ignored; source was not repaired".into(),
                );
                break;
            }
            Err(error) => {
                push_diag(
                    stream,
                    format!("Corrupt telemetry line {}: {error}", line_number + 1),
                );
                if had_newline {
                    insert_uncertainty_gap(stream, "corrupt_record");
                }
            }
        }
    }
    Ok(())
}

fn is_truncated_json(error: &str) -> bool {
    error.contains("EOF") || error.contains("eof") || error.contains("unterminated")
}

fn parse_geometry_line(
    line: &[u8],
    line_number: usize,
    stream: &mut TelemetryStream,
) -> Result<(), String> {
    let value: serde_json::Value = serde_json::from_slice(line).map_err(|e| e.to_string())?;
    let peek: LinePeek = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    if peek.record.as_deref() == Some("flush") {
        return Ok(());
    }
    if peek.record.as_deref() == Some("event") {
        push_diag(
            stream,
            format!("Event record in geometry.jsonl ignored at line {line_number}"),
        );
        return Ok(());
    }
    let geometry = if peek.version == Some(2)
        || peek.coordinate_space.is_some()
        || peek.record.as_deref() == Some("geometry")
    {
        if peek.version.is_some() && peek.version != Some(2) {
            push_diag(
                stream,
                format!(
                    "Unknown geometry version {} at line {line_number}",
                    peek.version.unwrap()
                ),
            );
            return Ok(());
        }
        let parsed: MouseGeometry = serde_json::from_value(value).map_err(|e| e.to_string())?;
        geometry_from_v2(parsed)
    } else if peek.geometry_id.is_some() {
        let parsed: GeometryRecord = serde_json::from_value(value).map_err(|e| e.to_string())?;
        geometry_from_v1(parsed)
    } else {
        push_diag(
            stream,
            format!("Unknown geometry record at line {line_number}"),
        );
        return Ok(());
    };
    if geometry.geometry_id.is_empty() || geometry.t_us > MAX_SAFE_TIME {
        push_diag(
            stream,
            format!("Invalid geometry identity at line {line_number}"),
        );
        return Ok(());
    }
    stream
        .geometries
        .insert(geometry.geometry_id.clone(), geometry);
    Ok(())
}

fn geometry_from_v1(record: GeometryRecord) -> CanonicalGeometry {
    let supported = record.physical_width > 0 && record.physical_height > 0;
    CanonicalGeometry {
        geometry_id: record.geometry_id,
        t_us: record.t_us,
        version: 1,
        supported,
        unsupported_reason: (!supported).then(|| "v1 geometry missing physical size".into()),
        sampling_interval_us: 100_000,
        source_id: None,
    }
}

fn geometry_from_v2(record: MouseGeometry) -> CanonicalGeometry {
    let mut unsupported_reason = None;
    // Events carry coordinates the recorder already normalized to the source, so the space
    // name (quartz_global on macOS, a Windows desktop space on Windows) is only a label.
    if record.coordinate_space.trim().is_empty() {
        unsupported_reason = Some("geometry has no coordinate space".into());
    } else if record.source_id.starts_with("application:") {
        unsupported_reason = Some("application capture geometry is unsupported".into());
    } else if record.source_id.starts_with("window:")
        && (record.physical_width.is_none() || record.physical_height.is_none())
    {
        unsupported_reason = Some("window geometry is missing physical transforms".into());
    } else if record.output_width == 0 || record.output_height == 0 {
        unsupported_reason = Some("geometry has zero output size".into());
    }
    CanonicalGeometry {
        geometry_id: record.geometry_id,
        t_us: record.t_us,
        version: 2,
        supported: unsupported_reason.is_none(),
        unsupported_reason,
        sampling_interval_us: record.sampling_interval_us.max(1),
        source_id: Some(record.source_id),
    }
}

fn parse_event_line(
    line: &[u8],
    line_number: usize,
    stream: &mut TelemetryStream,
) -> Result<(), String> {
    let value: serde_json::Value = serde_json::from_slice(line).map_err(|e| e.to_string())?;
    let peek: LinePeek = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    if peek.record.as_deref() == Some("flush") {
        return Ok(());
    }
    if peek.record.as_deref() == Some("geometry") {
        push_diag(
            stream,
            format!("Geometry record in events.jsonl ignored at line {line_number}"),
        );
        insert_uncertainty_gap(stream, "unknown_record");
        return Ok(());
    }
    let version = peek.version.unwrap_or(0);
    let event = if version == 2 || peek.payload.is_some() {
        if version != 0 && version != 2 {
            push_diag(
                stream,
                format!("Unknown telemetry version {version} at line {line_number}"),
            );
            insert_uncertainty_gap(stream, "unknown_version");
            return Ok(());
        }
        let parsed: MouseEvent = serde_json::from_value(value).map_err(|e| e.to_string())?;
        event_from_v2(parsed)?
    } else if version == 1 || peek.kind.is_some() {
        if version != 0 && version != 1 {
            push_diag(
                stream,
                format!("Unknown telemetry version {version} at line {line_number}"),
            );
            insert_uncertainty_gap(stream, "unknown_version");
            return Ok(());
        }
        let parsed: TelemetryEvent = serde_json::from_value(value).map_err(|e| e.to_string())?;
        event_from_v1(parsed)
    } else {
        push_diag(
            stream,
            format!("Unknown telemetry version {version} at line {line_number}"),
        );
        insert_uncertainty_gap(stream, "unknown_version");
        return Ok(());
    };
    if event.t_us > MAX_SAFE_TIME {
        push_diag(
            stream,
            format!("Telemetry timestamp exceeds precision at line {line_number}"),
        );
        insert_uncertainty_gap(stream, "invalid_timestamp");
        return Ok(());
    }
    if let Some(previous) = stream.events.last() {
        let prev_seq = previous.seq;
        let prev_t_us = previous.t_us;
        let prev_is_gap = matches!(previous.kind, CanonicalKind::Gap { .. });
        if event.seq <= prev_seq {
            push_diag(
                stream,
                format!(
                    "Telemetry sequence is not increasing at line {line_number} (seq {})",
                    event.seq
                ),
            );
            insert_uncertainty_gap(stream, "sequence_discontinuity");
        } else if event.seq > prev_seq.saturating_add(1) && !prev_is_gap {
            push_diag(
                stream,
                format!(
                    "Telemetry sequence skipped at line {line_number} (seq {} after {})",
                    event.seq, prev_seq
                ),
            );
            stream.events.push(CanonicalEvent {
                version: 2,
                seq: prev_seq.saturating_add(1),
                t_us: prev_t_us,
                geometry_id: None,
                kind: CanonicalKind::Gap {
                    reason: "sequence_discontinuity".into(),
                    start_us: prev_t_us,
                    end_us: event.t_us,
                    dropped_events: event.seq.saturating_sub(prev_seq).saturating_sub(1),
                },
            });
        }
    }
    push_thinned(stream, event);
    Ok(())
}

/// Appends `event`, replacing the newest move instead when it and the move before it are
/// within [`MOVE_THIN_US`] of `event`. Replacing (not dropping) keeps sequence numbers
/// contiguous, so thinning never looks like a telemetry gap.
fn push_thinned(stream: &mut TelemetryStream, event: CanonicalEvent) {
    let is_move = |e: &CanonicalEvent| matches!(e.kind, CanonicalKind::Move { .. });
    if is_move(&event) {
        if let [.., before, last] = stream.events.as_slice() {
            if is_move(before)
                && is_move(last)
                && last.seq.saturating_add(1) == event.seq
                && last.geometry_id == event.geometry_id
                && event.t_us.saturating_sub(before.t_us) < MOVE_THIN_US
            {
                *stream.events.last_mut().unwrap() = event;
                return;
            }
        }
    }
    stream.events.push(event);
}

fn event_from_v1(event: TelemetryEvent) -> CanonicalEvent {
    let kind = match event.kind {
        TelemetryKind::Move => CanonicalKind::Move {
            norm_x: event.norm_x as f64,
            norm_y: event.norm_y as f64,
            inside_source: event.inside_source,
        },
        TelemetryKind::Down => CanonicalKind::ButtonDown {
            button: None,
            norm_x: event.norm_x as f64,
            norm_y: event.norm_y as f64,
            inside_source: event.inside_source,
        },
        TelemetryKind::Up => CanonicalKind::ButtonUp {
            button: None,
            norm_x: event.norm_x as f64,
            norm_y: event.norm_y as f64,
            inside_source: event.inside_source,
        },
        TelemetryKind::Click => CanonicalKind::Click {
            norm_x: event.norm_x as f64,
            norm_y: event.norm_y as f64,
            inside_source: event.inside_source,
        },
        TelemetryKind::Scroll => CanonicalKind::Scroll {
            norm_x: Some(event.norm_x as f64),
            norm_y: Some(event.norm_y as f64),
            inside_source: Some(event.inside_source),
        },
    };
    CanonicalEvent {
        version: 1,
        seq: event.seq,
        t_us: event.t_us,
        geometry_id: Some(event.geometry_id),
        kind,
    }
}

fn event_from_v2(event: MouseEvent) -> Result<CanonicalEvent, String> {
    let coords = || -> Result<(f64, f64, bool), String> {
        let x = event.norm_x.ok_or("Missing mouse coordinates")?;
        let y = event.norm_y.ok_or("Missing mouse coordinates")?;
        let inside = event.inside_source.ok_or("Missing mouse coordinates")?;
        if !x.is_finite() || !y.is_finite() {
            return Err("Invalid mouse coordinates".into());
        }
        Ok((x, y, inside))
    };
    let kind = match event.payload {
        MousePayload::Move => {
            let (norm_x, norm_y, inside_source) = coords()?;
            CanonicalKind::Move {
                norm_x,
                norm_y,
                inside_source,
            }
        }
        MousePayload::ButtonDown { button } => {
            let (norm_x, norm_y, inside_source) = coords()?;
            CanonicalKind::ButtonDown {
                button: Some(button),
                norm_x,
                norm_y,
                inside_source,
            }
        }
        MousePayload::ButtonUp { button } => {
            let (norm_x, norm_y, inside_source) = coords()?;
            CanonicalKind::ButtonUp {
                button: Some(button),
                norm_x,
                norm_y,
                inside_source,
            }
        }
        MousePayload::Scroll { .. } => CanonicalKind::Scroll {
            norm_x: event.norm_x,
            norm_y: event.norm_y,
            inside_source: event.inside_source,
        },
        MousePayload::Gap {
            reason,
            start_us,
            end_us,
            dropped_events,
        } => {
            if start_us > end_us || end_us != event.t_us {
                return Err("Invalid mouse gap".into());
            }
            CanonicalKind::Gap {
                reason,
                start_us,
                end_us,
                dropped_events,
            }
        }
        MousePayload::CursorChanged { .. } | MousePayload::Unknown => CanonicalKind::Note,
    };
    Ok(CanonicalEvent {
        version: 2,
        seq: event.seq,
        t_us: event.t_us,
        geometry_id: event.geometry_id,
        kind,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_pair(root: &Path, geometry: &str, events: &str) {
        fs::create_dir_all(root.join("telemetry")).unwrap();
        fs::write(root.join("telemetry/geometry.jsonl"), geometry).unwrap();
        fs::write(root.join("telemetry/events.jsonl"), events).unwrap();
    }

    /// Shaped like the Windows recorder's output: a `windows_virtual_screen` geometry and
    /// `cursor_changed` records between the mouse events.
    const WINDOWS_GEOMETRY: &str = r#"{"version":2,"geometry_id":"mouse-g1","t_us":4895,"coordinate_space":"windows_virtual_screen","source_id":"display:1","bounds":{"x":0.0,"y":0.0,"width":1920.0,"height":1080.0},"output_width":1920,"output_height":1080,"sampling_interval_us":100000,"cursor_mode":"replace","physical_width":1920.0,"physical_height":1080.0,"rotation_degrees":0.0,"logical_to_physical_scale_x":1.0,"logical_to_physical_scale_y":1.0}
"#;

    fn windows_events() -> String {
        let mut lines = vec![
            r#"{"version":2,"seq":0,"t_us":4895,"payload":{"kind":"gap","reason":"geometry_changed","start_us":0,"end_us":4895,"dropped_events":0}}"#.to_string(),
            r#"{"version":2,"seq":1,"t_us":4895,"geometry_id":"mouse-g1","payload":{"kind":"cursor_changed","cursor_id":"c1","name":"arrow","hotspot_x":2.0,"hotspot_y":2.0,"width":32.0,"height":32.0}}"#.to_string(),
        ];
        let mut seq = 2;
        // The cursor rests near (0.7, 0.6) for two seconds, then clicks there.
        for i in 0..20u64 {
            lines.push(format!(
                r#"{{"version":2,"seq":{seq},"t_us":{},"geometry_id":"mouse-g1","norm_x":0.7,"norm_y":0.6,"inside_source":true,"payload":{{"kind":"move"}}}}"#,
                500_000 + i * 100_000
            ));
            seq += 1;
        }
        lines.push(format!(
            r#"{{"version":2,"seq":{seq},"t_us":2600000,"geometry_id":"mouse-g1","payload":{{"kind":"cursor_changed","cursor_id":"c1","name":"pointing_hand","hotspot_x":2.0,"hotspot_y":2.0,"width":32.0,"height":32.0}}}}"#
        ));
        seq += 1;
        for (kind, t) in [("button_down", 2_700_000), ("button_up", 2_780_000)] {
            lines.push(format!(
                r#"{{"version":2,"seq":{seq},"t_us":{t},"geometry_id":"mouse-g1","norm_x":0.7,"norm_y":0.6,"inside_source":true,"payload":{{"kind":"{kind}","button":0}}}}"#
            ));
            seq += 1;
        }
        lines.push(format!(
            r#"{{"version":2,"seq":{seq},"t_us":2800000,"payload":{{"kind":"a_future_kind","value":1}}}}"#
        ));
        lines.join("\n") + "\n"
    }

    #[test]
    fn windows_recorder_telemetry_reads_cleanly_and_yields_zooms() {
        let dir = tempdir().unwrap();
        write_pair(dir.path(), WINDOWS_GEOMETRY, &windows_events());
        let stream = read_telemetry(dir.path()).unwrap();
        assert!(stream.diagnostics.is_empty(), "{:?}", stream.diagnostics);
        let geometry = &stream.geometries["mouse-g1"];
        assert!(geometry.supported, "{:?}", geometry.unsupported_reason);
        // Only the recorder's own leading gap; cursor changes and unknown kinds are notes.
        let gaps = stream
            .events
            .iter()
            .filter(|e| matches!(e.kind, CanonicalKind::Gap { .. }))
            .count();
        assert_eq!(gaps, 1);
        assert_eq!(
            stream
                .events
                .iter()
                .filter(|e| e.kind == CanonicalKind::Note)
                .count(),
            3
        );
        let generation =
            crate::zoom::generate_zoom_suggestions(&stream, &crate::zoom::ZoomConfig::default())
                .unwrap();
        assert!(
            !generation.suggestions.is_empty(),
            "{:?}",
            generation.diagnostics
        );
    }

    #[test]
    fn dense_moves_are_thinned_without_breaking_the_sequence() {
        let dir = tempdir().unwrap();
        // One move per millisecond for a second.
        let events: String = (0..1_000u64)
            .map(|i| {
                format!(
                    "{{\"version\":2,\"seq\":{i},\"t_us\":{},\"geometry_id\":\"mouse-g1\",\"norm_x\":{},\"norm_y\":0.5,\"inside_source\":true,\"payload\":{{\"kind\":\"move\"}}}}\n",
                    1_000 + i * 1_000,
                    i as f64 / 1_000.0
                )
            })
            .collect();
        write_pair(dir.path(), WINDOWS_GEOMETRY, &events);
        let stream = read_telemetry(dir.path()).unwrap();
        assert!(stream.diagnostics.is_empty(), "{:?}", stream.diagnostics);
        assert!(
            stream
                .events
                .iter()
                .all(|e| matches!(e.kind, CanonicalKind::Move { .. })),
            "thinning must not insert gaps"
        );
        // About one sample per 8 ms, and the last position survives.
        assert!(
            (100..=160).contains(&stream.events.len()),
            "{}",
            stream.events.len()
        );
        assert_eq!(stream.events.last().unwrap().seq, 999);
        let gaps_between = stream
            .events
            .windows(2)
            .map(|w| w[1].t_us - w[0].t_us)
            .max()
            .unwrap();
        assert!(gaps_between <= MOVE_THIN_US, "{gaps_between}");
    }

    #[test]
    fn reads_v1_kind_and_v2_payload_without_record_wrapper() {
        let dir = tempdir().unwrap();
        write_pair(
            dir.path(),
            r#"{"geometry_id":"g1","t_us":0,"physical_width":1920,"physical_height":1080,"scale_factor":2.0,"crop_x":0,"crop_y":0,"crop_width":1920,"crop_height":1080}
{"version":2,"geometry_id":"g2","t_us":10,"coordinate_space":"quartz_global","source_id":"display:1","bounds":{"x":0,"y":0,"width":1920,"height":1080},"output_width":1920,"output_height":1080,"sampling_interval_us":100000,"cursor_mode":"baked","physical_width":1920,"physical_height":1080}
"#,
            r#"{"version":1,"seq":0,"t_us":1000,"geometry_id":"g1","kind":"click","norm_x":0.4,"norm_y":0.5,"inside_source":true,"visible":true}
{"version":2,"seq":1,"t_us":5000,"geometry_id":"g2","norm_x":-0.25,"norm_y":1.5,"inside_source":false,"payload":{"kind":"move"}}
{"version":2,"seq":2,"t_us":6000,"payload":{"kind":"gap","reason":"queue_overflow","start_us":5500,"end_us":6000,"dropped_events":3}}
"#,
        );
        let stream = read_telemetry(dir.path()).unwrap();
        assert_eq!(stream.geometries.len(), 2);
        assert!(stream.geometries["g1"].supported);
        assert!(stream.geometries["g2"].supported);
        assert_eq!(stream.events.len(), 3);
        assert!(matches!(stream.events[0].kind, CanonicalKind::Click { .. }));
        assert!(matches!(
            stream.events[1].kind,
            CanonicalKind::Move {
                inside_source: false,
                ..
            }
        ));
        if let CanonicalKind::Move { norm_x, .. } = stream.events[1].kind {
            assert_eq!(norm_x, -0.25);
        }
        assert!(matches!(
            stream.events[2].kind,
            CanonicalKind::Gap {
                dropped_events: 3,
                ..
            }
        ));
        assert!(stream.events[2].geometry_id.is_none());
    }

    #[test]
    fn recovers_partial_final_line_and_reports_corrupt_interior() {
        let dir = tempdir().unwrap();
        write_pair(
            dir.path(),
            "{\"geometry_id\":\"g1\",\"t_us\":0,\"physical_width\":100,\"physical_height\":100,\"scale_factor\":1,\"crop_x\":0,\"crop_y\":0,\"crop_width\":100,\"crop_height\":100}\n",
            "{\"version\":1,\"seq\":0,\"t_us\":1,\"geometry_id\":\"g1\",\"kind\":\"move\",\"norm_x\":0.2,\"norm_y\":0.2,\"inside_source\":true,\"visible\":true}\n{not json}\n{\"version\":1,\"seq\":2,\"t_us\":3,\"geometry_id\":\"g1\",\"kind\":\"move\",\"norm_x\":0.3,\"norm_y\":0.3,\"inside_source\":true,\"visible\":true}\n{\"version\":1,\"seq\":3,\"t_us\":4,\"geometry_id\":\"g1\",\"kind\":\"move\",\"norm_x\":0.4",
        );
        let stream = read_telemetry(dir.path()).unwrap();
        assert!(stream
            .events
            .iter()
            .any(|event| matches!(event.kind, CanonicalKind::Gap { ref reason, .. } if reason == "corrupt_record")));
        assert_eq!(
            stream
                .events
                .iter()
                .filter(|event| !matches!(event.kind, CanonicalKind::Gap { .. }))
                .count(),
            2
        );
        assert!(stream
            .diagnostics
            .iter()
            .any(|d| d.contains("Corrupt telemetry line 2")));
        assert!(stream
            .diagnostics
            .iter()
            .any(|d| d.contains("Incomplete final telemetry line")));
    }

    #[test]
    fn reports_unknown_versions_without_inventing_events() {
        let dir = tempdir().unwrap();
        write_pair(
            dir.path(),
            "{\"version\":9,\"geometry_id\":\"gx\",\"t_us\":0,\"coordinate_space\":\"mystery\"}\n",
            "{\"version\":9,\"seq\":0,\"t_us\":1,\"payload\":{\"kind\":\"teleport\"}}\n{\"version\":1,\"seq\":1,\"t_us\":2,\"geometry_id\":\"g1\",\"kind\":\"move\",\"norm_x\":0.1,\"norm_y\":0.1,\"inside_source\":true,\"visible\":true}\n",
        );
        let stream = read_telemetry(dir.path()).unwrap();
        assert_eq!(
            stream
                .events
                .iter()
                .filter(|event| !matches!(event.kind, CanonicalKind::Gap { .. }))
                .count(),
            1
        );
        assert_eq!(
            stream
                .events
                .iter()
                .find(|event| !matches!(event.kind, CanonicalKind::Gap { .. }))
                .unwrap()
                .version,
            1
        );
        assert!(stream
            .events
            .iter()
            .any(|event| matches!(event.kind, CanonicalKind::Gap { ref reason, .. } if reason == "unknown_version")));
        assert!(stream
            .diagnostics
            .iter()
            .any(|d| d.contains("Unknown telemetry version 9")));
        assert!(stream
            .diagnostics
            .iter()
            .any(|d| d.contains("Unknown geometry version 9")));
    }

    #[test]
    fn missing_files_are_empty_not_fabricated() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("telemetry")).unwrap();
        let stream = read_telemetry(dir.path()).unwrap();
        assert!(stream.events.is_empty());
        assert!(stream.geometries.is_empty());
        assert!(stream
            .diagnostics
            .iter()
            .any(|d| d.contains("No telemetry")));
    }

    #[test]
    fn window_geometry_without_physical_transform_is_unsupported() {
        let dir = tempdir().unwrap();
        write_pair(
            dir.path(),
            r#"{"version":2,"geometry_id":"win","t_us":0,"coordinate_space":"quartz_global","source_id":"window:3","bounds":{"x":10,"y":10,"width":800,"height":600},"output_width":800,"output_height":600,"sampling_interval_us":100000,"cursor_mode":"baked"}
"#,
            "",
        );
        let stream = read_telemetry(dir.path()).unwrap();
        let geo = &stream.geometries["win"];
        assert!(!geo.supported);
        assert!(geo
            .unsupported_reason
            .as_deref()
            .unwrap()
            .contains("physical"));
    }

    #[test]
    fn permission_gaps_and_application_geometry_do_not_invent_events() {
        let dir = tempdir().unwrap();
        write_pair(
            dir.path(),
            r#"{"version":2,"geometry_id":"app","t_us":0,"coordinate_space":"quartz_global","source_id":"application:com.example","bounds":{"x":0,"y":0,"width":800,"height":600},"output_width":800,"output_height":600,"sampling_interval_us":100000,"cursor_mode":"baked"}
{"version":2,"geometry_id":"disp","t_us":0,"coordinate_space":"quartz_global","source_id":"display:1","bounds":{"x":-1920,"y":-100,"width":1920,"height":1080},"output_width":1920,"output_height":1080,"sampling_interval_us":100000,"cursor_mode":"baked","physical_width":1920,"physical_height":1080}
"#,
            r#"{"version":2,"seq":0,"t_us":0,"payload":{"kind":"gap","reason":"initial_button_state_unknown","start_us":0,"end_us":0,"dropped_events":0}}
{"version":2,"seq":1,"t_us":1000,"payload":{"kind":"gap","reason":"input_monitoring_unavailable","start_us":1000,"end_us":1000,"dropped_events":0}}
{"version":2,"seq":2,"t_us":2000,"payload":{"kind":"gap","reason":"input_monitoring_revoked","start_us":1000,"end_us":2000,"dropped_events":0}}
{"version":1,"seq":3,"t_us":3000,"geometry_id":"disp","kind":"click","norm_x":0.4,"norm_y":0.5,"inside_source":true,"visible":true}
"#,
        );
        let stream = read_telemetry(dir.path()).unwrap();
        assert!(!stream.geometries["app"].supported);
        assert!(stream.geometries["app"]
            .unsupported_reason
            .as_deref()
            .unwrap()
            .contains("application"));
        assert!(stream.geometries["disp"].supported);
        assert_eq!(stream.geometries["disp"].sampling_interval_us, 100_000);
        assert_eq!(stream.events.len(), 4);
        assert!(matches!(
            &stream.events[1].kind,
            CanonicalKind::Gap { reason, .. } if reason == "input_monitoring_unavailable"
        ));
        assert!(matches!(
            &stream.events[2].kind,
            CanonicalKind::Gap { reason, .. } if reason == "input_monitoring_revoked"
        ));
        assert!(
            matches!(stream.events[3].kind, CanonicalKind::Click { .. }),
            "v1 click must remain a click with unknown button provenance"
        );
    }

    #[test]
    fn geometry_changed_gap_preserves_uncertainty_interval() {
        let dir = tempdir().unwrap();
        write_pair(
            dir.path(),
            r#"{"version":2,"geometry_id":"g1","t_us":1000000,"coordinate_space":"quartz_global","source_id":"display:1","bounds":{"x":0,"y":0,"width":1920,"height":1080},"output_width":1920,"output_height":1080,"sampling_interval_us":100000,"cursor_mode":"baked","physical_width":1920,"physical_height":1080}
"#,
            r#"{"version":2,"seq":0,"t_us":1000000,"payload":{"kind":"gap","reason":"geometry_changed","start_us":900000,"end_us":1000000,"dropped_events":0}}
"#,
        );
        let stream = read_telemetry(dir.path()).unwrap();
        assert_eq!(stream.geometries["g1"].sampling_interval_us, 100_000);
        match &stream.events[0].kind {
            CanonicalKind::Gap {
                reason,
                start_us,
                end_us,
                ..
            } => {
                assert_eq!(reason, "geometry_changed");
                assert_eq!(end_us.saturating_sub(*start_us), 100_000);
            }
            other => panic!("expected geometry_changed gap, got {other:?}"),
        }
    }
}
