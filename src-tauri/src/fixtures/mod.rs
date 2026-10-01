//! Synthetic media and project bundles for tests and the media parity check.

/// 16-bit PCM WAV from interleaved sample frames. Frame count is `samples.len() / channels`.
pub fn generate_pcm16_wav(sample_rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
    assert!(channels > 0);
    assert_eq!(samples.len() % channels as usize, 0);
    let data_bytes = (samples.len() * 2) as u32;
    let riff_chunk_size = 36 + data_bytes;
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&riff_chunk_size.to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * channels as u32 * 2;
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&(channels * 2).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_bytes.to_le_bytes());
    for sample in samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    out
}

/// Minimal on-disk project bundle (manifest, journal and media folders) for
/// tests that need a real project to open with `ProjectReader`.
#[cfg(test)]
pub struct TestProject {
    root: std::path::PathBuf,
    manifest: crate::project::ProjectManifest,
    next_seq: u64,
}

#[cfg(test)]
impl TestProject {
    pub fn create(parent: &std::path::Path, name: &str) -> Self {
        let root = parent.join(format!("{name}.aero"));
        for dir in ["telemetry", "media/screen", "media/webcam", "media/system", "media/mic", "cache"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        let manifest = crate::project::ProjectManifest::new(format!("session-{name}"), name.into());
        let project = Self {
            root,
            manifest,
            next_seq: 0,
        };
        project.save_manifest();
        project
    }

    pub fn root_path(&self) -> &std::path::Path {
        &self.root
    }

    pub fn manifest_mut(&mut self) -> &mut crate::project::ProjectManifest {
        &mut self.manifest
    }

    pub fn save_manifest(&self) {
        self.manifest
            .save_with_backup(self.root.join("manifest.json"))
            .unwrap();
    }

    /// Appends `record` to `journal.jsonl`, assigning the next sequence number.
    pub fn append_journal(&mut self, mut record: crate::project::JournalRecord) {
        use std::io::Write;
        record.set_seq(self.next_seq);
        self.next_seq += 1;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join("journal.jsonl"))
            .unwrap();
        writeln!(file, "{}", serde_json::to_string(&record).unwrap()).unwrap();
    }
}
