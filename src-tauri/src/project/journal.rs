//! Record format of the recorder's append-only `journal.jsonl`. The editor
//! only reads it (see `reader.rs`).
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JournalRecord {
    SegmentCommitted {
        seq: u64,
        track_id: String,
        relative_path: String,
        start_us: u64,
        end_us: u64,
        size_bytes: u64,
        is_keyframe_start: bool,
        /// Native media-clock timescale (e.g. 90_000 for H.264) parsed from
        /// the segment's `mdhd` box. Zero when the segment is not fMP4.
        #[serde(default)]
        media_timescale: u32,
        /// Media-clock value at which the segment starts. Signed because
        /// `elst` edit lists can carry a non-zero media_time offset. Zero
        /// when the segment is not fMP4.
        #[serde(default)]
        media_start_value: i64,
        /// Host-clock anchor (epoch microseconds) at which the segment's
        /// media start_value was published. Lets the editor re-establish
        /// the same media → host mapping on replay.
        #[serde(default)]
        host_anchor_us: i64,
    },
    Discontinuity {
        seq: u64,
        track_id: String,
        t_us: u64,
        reason: String,
    },
    PauseStarted {
        seq: u64,
        t_us: u64,
    },
    PauseEnded {
        seq: u64,
        start_us: u64,
        end_us: u64,
    },
    UnindexedSegmentRecovered {
        seq: u64,
        track_id: String,
        relative_path: String,
        start_us: u64,
        end_us: u64,
        size_bytes: u64,
        /// Same timescale metadata as `SegmentCommitted` when known.
        #[serde(default)]
        media_timescale: u32,
        #[serde(default)]
        media_start_value: i64,
        #[serde(default)]
        host_anchor_us: i64,
    },
    Checkpoint {
        seq: u64,
        t_us: u64,
        total_duration_us: u64,
    },
    RuntimeError {
        seq: u64,
        track_id: String,
        error_code: i32,
        message: String,
        t_us: u64,
        recoverable: bool,
    },
}

impl JournalRecord {
    pub fn seq(&self) -> u64 {
        match self {
            JournalRecord::SegmentCommitted { seq, .. } => *seq,
            JournalRecord::Discontinuity { seq, .. } => *seq,
            JournalRecord::PauseStarted { seq, .. } => *seq,
            JournalRecord::PauseEnded { seq, .. } => *seq,
            JournalRecord::UnindexedSegmentRecovered { seq, .. } => *seq,
            JournalRecord::Checkpoint { seq, .. } => *seq,
            JournalRecord::RuntimeError { seq, .. } => *seq,
        }
    }

    #[cfg(test)]
    pub fn set_seq(&mut self, new_seq: u64) {
        match self {
            JournalRecord::SegmentCommitted { seq, .. } => *seq = new_seq,
            JournalRecord::Discontinuity { seq, .. } => *seq = new_seq,
            JournalRecord::PauseStarted { seq, .. } => *seq = new_seq,
            JournalRecord::PauseEnded { seq, .. } => *seq = new_seq,
            JournalRecord::UnindexedSegmentRecovered { seq, .. } => *seq = new_seq,
            JournalRecord::Checkpoint { seq, .. } => *seq = new_seq,
            JournalRecord::RuntimeError { seq, .. } => *seq = new_seq,
        }
    }
}
