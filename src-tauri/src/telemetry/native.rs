//! Version 2 mouse stream records as written by the recorder. V1 clicks remain
//! unchanged; v2 stores authoritative transitions and never invents derived
//! click provenance.
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MousePayload {
    Move,
    ButtonDown {
        button: u32,
    },
    ButtonUp {
        button: u32,
    },
    Scroll {
        delta_x: f64,
        delta_y: f64,
        units: ScrollUnits,
        precise: bool,
        phase: i64,
        momentum_phase: i64,
    },
    Gap {
        reason: String,
        start_us: u64,
        end_us: u64,
        dropped_events: u64,
    },
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScrollUnits {
    Pixels,
    Lines,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MouseEvent {
    pub version: u32,
    pub seq: u64,
    pub t_us: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub norm_x: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub norm_y: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inside_source: Option<bool>,
    pub payload: MousePayload,
    // Visibility and modifiers are unknown in this adapter, not fabricated.
}
#[derive(Debug, Deserialize, Serialize)]
pub struct MouseBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct MouseGeometry {
    pub version: u32,
    pub geometry_id: String,
    pub t_us: u64,
    pub coordinate_space: String,
    pub source_id: String,
    pub bounds: MouseBounds,
    pub output_width: u32,
    pub output_height: u32,
    pub sampling_interval_us: u64,
    pub cursor_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub physical_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub physical_height: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation_degrees: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logical_to_physical_scale_x: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logical_to_physical_scale_y: Option<f64>,
}
