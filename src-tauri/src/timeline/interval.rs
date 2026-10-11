use serde::{Deserialize, Serialize};

/// Non-destructive half-open retained source interval `[start_us, end_us)`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInterval {
    pub id: String,
    pub start_us: u64,
    pub end_us: u64,
    /// Imported media asset this entry plays; `None` is the recording. Times are within
    /// that asset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<String>,
    /// How long it plays in edited time when that is not its source length (a clip played
    /// faster or slower).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited_len_us: Option<u64>,
}

/// `x * num / den`, exact when `num == den`.
fn scale(x: u64, num: u64, den: u64) -> u64 {
    if num == den {
        x
    } else {
        (x as u128 * num as u128 / den.max(1) as u128) as u64
    }
}

impl SourceInterval {
    pub fn new(id: String, start_us: u64, end_us: u64) -> Self {
        assert!(start_us <= end_us, "start_us must be <= end_us");
        Self {
            id,
            start_us,
            end_us,
            media: None,
            edited_len_us: None,
        }
    }

    pub fn with_media(mut self, media: Option<String>) -> Self {
        self.media = media;
        self
    }

    /// Plays for `len` of edited time (`None`: its source length).
    pub fn with_edited_len(mut self, len: Option<u64>) -> Self {
        self.edited_len_us = len.filter(|&len| len != self.source_duration_us());
        self
    }

    pub fn is_recording(&self) -> bool {
        self.media.is_none()
    }

    /// How long it plays in edited time.
    pub fn duration_us(&self) -> u64 {
        self.edited_len_us
            .unwrap_or_else(|| self.source_duration_us())
    }

    /// How much source time it holds.
    pub fn source_duration_us(&self) -> u64 {
        self.end_us.saturating_sub(self.start_us)
    }

    /// The source time playing `offset` of edited time into it.
    pub fn source_at(&self, offset: u64) -> u64 {
        self.start_us + scale(offset, self.source_duration_us(), self.duration_us())
    }

    /// How far into it (in edited time) source time `source_us` plays.
    pub fn edited_offset(&self, source_us: u64) -> u64 {
        scale(
            source_us.saturating_sub(self.start_us),
            self.duration_us(),
            self.source_duration_us(),
        )
    }

    pub fn contains_source_us(&self, source_us: u64) -> bool {
        source_us >= self.start_us && source_us < self.end_us
    }

    /// Excludes a given cut interval `[cut_start, cut_end)` from this interval.
    /// Returns 0, 1, or 2 resulting sub-intervals.
    pub fn exclude_range(&self, cut_start: u64, cut_end: u64) -> Vec<SourceInterval> {
        if cut_start >= self.end_us || cut_end <= self.start_us {
            // Cut does not overlap this interval at all
            return vec![self.clone()];
        }

        let mut results = Vec::new();
        // A part keeps the speed it played at.
        let edited = |start: u64, end: u64| {
            self.edited_len_us
                .map(|_| self.edited_offset(end) - self.edited_offset(start))
        };

        // Leading portion before the cut
        if cut_start > self.start_us {
            let end = cut_start.min(self.end_us);
            results.push(
                SourceInterval::new(format!("{}-a", self.id), self.start_us, end)
                    .with_media(self.media.clone())
                    .with_edited_len(edited(self.start_us, end)),
            );
        }

        // Trailing portion after the cut
        if cut_end < self.end_us {
            let start = cut_end.max(self.start_us);
            results.push(
                SourceInterval::new(format!("{}-b", self.id), start, self.end_us)
                    .with_media(self.media.clone())
                    .with_edited_len(edited(start, self.end_us)),
            );
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interval_exclusion() {
        let interval = SourceInterval::new("int-1".into(), 1_000_000, 10_000_000);

        // Middle cut: cuts [3s, 5s), leaving [1s, 3s) and [5s, 10s)
        let parts = interval.exclude_range(3_000_000, 5_000_000);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].start_us, 1_000_000);
        assert_eq!(parts[0].end_us, 3_000_000);
        assert_eq!(parts[1].start_us, 5_000_000);
        assert_eq!(parts[1].end_us, 10_000_000);

        // Entire interval cut: cuts [0s, 15s)
        let empty = interval.exclude_range(0, 15_000_000);
        assert!(empty.is_empty());
    }
}
