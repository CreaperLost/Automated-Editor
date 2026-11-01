//! Exports, one at a time, and the list of what went out (DLV-2).
//!
//! One export runs at a time, as always: asking for another while one runs is refused unless
//! the caller queues it, and then it waits its turn. One worker thread runs the queue in order.
//! Every export that ends is recorded in the project (`exports.json`): what it was (the video or
//! a short, with its settings, so it can be made again), the edit revision it was made from and
//! where it went, so the list can say which outputs are older than the edit.
use super::{run_job, status_from, CapturedExport, EncoderGate, ExportSettings, ExportState};
use super::{ExportFailure, ExportStatus};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

pub const FILE: &str = "exports.json";
/// The newest outputs a project keeps a record of.
const MAX_RECORDS: usize = 100;

/// What an export makes, to make it again.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    /// A short or clip's id; none for the video.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_id: Option<String>,
    pub settings: ExportSettings,
    /// A file beside the video instead (subtitles, transcript, sound).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<super::files::FileKind>,
}

/// One export in the list: waiting, running or done.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Delivery {
    pub job_id: String,
    /// "Video", or the short's title.
    pub label: String,
    pub request: Request,
    pub state: ExportState,
    /// The edit revision it was made from.
    pub revision: u64,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub progress: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<String>,
    /// When it ended, local time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    /// What it was made from ([`fingerprint`]): the same edit makes the same output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
}

/// What an export of `document` would show and play, whatever its revision number: two
/// documents with the same fingerprint export the same video.
pub fn fingerprint(document: &crate::project::revision::EditDocument) -> String {
    use sha2::{Digest, Sha256};
    let mut document = document.clone();
    document.revision = 0;
    let text = serde_json::to_string(&document).unwrap_or_default();
    format!("{:x}", Sha256::digest(text.as_bytes()))[..16].to_string()
}

struct Job {
    delivery: Delivery,
    root: PathBuf,
    cancel: Arc<AtomicBool>,
    status: Arc<Mutex<ExportStatus>>,
    /// Where it writes, kept while it waits or runs so no other export takes the name.
    dest: Option<PathBuf>,
    /// Waiting its turn; taken when it starts.
    captured: Option<CapturedExport>,
}

#[derive(Default)]
struct Shared {
    jobs: Vec<Job>,
    worker_running: bool,
}

/// The session's exports: the one running, the ones waiting, and the ones that ended.
pub struct ExportOwner {
    shared: Arc<Mutex<Shared>>,
    worker: Option<JoinHandle<()>>,
}

impl ExportOwner {
    pub fn new() -> Self {
        Self {
            shared: Default::default(),
            worker: None,
        }
    }

    /// An export runs or waits.
    pub fn busy(&mut self) -> bool {
        self.shared
            .lock()
            .jobs
            .iter()
            .any(|job| active(&job.status))
    }

    /// Starts `captured`, or queues it behind the one running when `queue`; without `queue` a
    /// busy owner refuses it, as before there was a queue.
    pub fn start(
        &mut self,
        captured: CapturedExport,
        label: &str,
        request: Request,
        queue: bool,
        gate: Arc<EncoderGate>,
    ) -> ExportStatus {
        if self.busy() && !queue {
            let status = status_from(
                &captured.job_id,
                &captured.document,
                &captured.settings,
                ExportState::Failed,
                Some(ExportFailure::EncoderBusy {
                    message: "An export job is already running".into(),
                }),
            );
            self.install_failed(status.clone());
            return status;
        }
        let status = status_from(
            &captured.job_id,
            &captured.document,
            &captured.settings,
            ExportState::Queued,
            None,
        );
        let delivery = Delivery {
            job_id: captured.job_id.clone(),
            label: label.to_string(),
            request,
            state: ExportState::Queued,
            revision: captured.document.revision,
            width: captured.settings.width,
            height: captured.settings.height,
            progress: 0.0,
            output_path: None,
            failure: None,
            finished_at: None,
            fingerprint: Some(fingerprint(&captured.document)),
        };
        let job = Job {
            delivery,
            root: captured.root.clone(),
            cancel: Arc::new(AtomicBool::new(false)),
            status: Arc::new(Mutex::new(status.clone())),
            dest: Some(captured.dest.clone()),
            captured: Some(captured),
        };
        let spawn = {
            let mut shared = self.shared.lock();
            shared.jobs.push(job);
            let spawn = !shared.worker_running;
            shared.worker_running = true;
            spawn
        };
        if spawn {
            if let Some(old) = self.worker.take() {
                let _ = old.join();
            }
            let shared = Arc::clone(&self.shared);
            self.worker = Some(std::thread::spawn(move || work(&shared, &gate)));
        }
        status
    }

    /// The export with `job_id`, or with none the one running (else the last one asked for).
    pub fn status_of(&mut self, job_id: Option<&str>) -> Option<ExportStatus> {
        let shared = self.shared.lock();
        let job = match job_id {
            Some(id) => shared.jobs.iter().find(|j| j.delivery.job_id == id),
            None => shared
                .jobs
                .iter()
                .find(|j| j.status.lock().state == ExportState::Running)
                .or_else(|| shared.jobs.last()),
        };
        job.map(|j| j.status.lock().clone())
    }

    /// The export running, else the last one asked for, else idle.
    pub fn status(&mut self) -> ExportStatus {
        self.status_of(None).unwrap_or_else(ExportStatus::idle)
    }

    /// Stops `job_id`: one waiting leaves the queue at once; the one running stops at its next
    /// frame.
    pub fn cancel(&mut self, job_id: &str) -> Result<ExportStatus, String> {
        let mut shared = self.shared.lock();
        let job = shared
            .jobs
            .iter_mut()
            .find(|j| j.delivery.job_id == job_id)
            .ok_or("Stale export job id")?;
        job.cancel.store(true, Ordering::SeqCst);
        if job.captured.take().is_some() {
            let mut status = job.status.lock();
            status.state = ExportState::Cancelled;
            status.failure = Some(ExportFailure::Cancelled {
                message: "Taken out of the queue".into(),
            });
            job.delivery.state = ExportState::Cancelled;
            return Ok(status.clone());
        }
        let mut status = job.status.lock();
        if matches!(status.state, ExportState::Queued | ExportState::Running) {
            status.diagnostics = vec!["Cancellation requested; waiting for cleanup".into()];
        }
        Ok(status.clone())
    }

    /// Keeps a refused export's status, so asking for it by id answers why.
    pub fn install_failed(&mut self, status: ExportStatus) {
        let delivery = Delivery {
            job_id: status.job_id.clone(),
            label: String::new(),
            request: Request {
                short_id: None,
                settings: ExportSettings::default(),
                file: None,
            },
            state: status.state,
            revision: status.captured_revision,
            width: status.width,
            height: status.height,
            progress: 0.0,
            output_path: None,
            failure: status.failure.as_ref().map(|f| f.message().to_string()),
            finished_at: None,
            fingerprint: None,
        };
        self.shared.lock().jobs.push(Job {
            delivery,
            root: PathBuf::new(),
            cancel: Arc::new(AtomicBool::new(true)),
            status: Arc::new(Mutex::new(status)),
            dest: None,
            captured: None,
        });
    }

    /// The exports of the project at `root` this session that are waiting or running, in turn.
    pub fn active_for(&mut self, root: &Path) -> Vec<Delivery> {
        let shared = self.shared.lock();
        shared
            .jobs
            .iter()
            .filter(|j| j.root == root && active(&j.status))
            .map(|j| delivery_now(j))
            .collect()
    }

    /// Where the exports of `root` that wait or run will write (their files aren't there
    /// yet), so a new one picks another name.
    pub fn waiting_destinations(&mut self, root: &Path) -> Vec<PathBuf> {
        let shared = self.shared.lock();
        shared
            .jobs
            .iter()
            .filter(|j| j.root == root && active(&j.status))
            .filter_map(|j| j.dest.clone())
            .collect()
    }
}

fn active(status: &Mutex<ExportStatus>) -> bool {
    matches!(
        status.lock().state,
        ExportState::Queued | ExportState::Running
    )
}

/// A job's line as it stands now.
fn delivery_now(job: &Job) -> Delivery {
    let status = job.status.lock();
    let mut delivery = job.delivery.clone();
    delivery.state = status.state;
    delivery.progress = match (status.progress_denominator, status.state) {
        (_, ExportState::Completed) => 1.0,
        (0, _) => 0.0,
        (total, _) => status.progress_numerator as f64 / total as f64,
    };
    delivery.output_path = status.output_path.clone();
    delivery.failure = status.failure.as_ref().map(|f| f.message().to_string());
    delivery
}

/// The worker: runs the waiting exports in the order they were asked for, records each that
/// ends, and stops when none is left.
fn work(shared: &Mutex<Shared>, gate: &EncoderGate) {
    loop {
        let next = {
            let mut shared = shared.lock();
            let found = shared
                .jobs
                .iter_mut()
                .find(|j| j.captured.is_some())
                .map(|j| {
                    (
                        j.captured.take(),
                        Arc::clone(&j.cancel),
                        Arc::clone(&j.status),
                        j.delivery.job_id.clone(),
                    )
                });
            if found.is_none() {
                shared.worker_running = false;
            }
            found
        };
        let Some((Some(captured), cancel, status, id)) = next else {
            return;
        };
        run_job(captured, cancel, Arc::clone(&status), gate);
        let ended = {
            let shared = shared.lock();
            shared
                .jobs
                .iter()
                .find(|j| j.delivery.job_id == id)
                .map(|j| (j.root.clone(), delivery_now(j)))
        };
        if let Some((root, mut delivery)) = ended {
            delivery.finished_at = Some(chrono::Local::now().format("%Y-%m-%d %H:%M").to_string());
            let _ = record(&root, delivery);
        }
    }
}

impl Default for ExportOwner {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ExportOwner {
    fn drop(&mut self) {
        for job in self.shared.lock().jobs.iter_mut() {
            job.cancel.store(true, Ordering::SeqCst);
            job.captured = None;
        }
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }
}

/// The outputs recorded in the project at `root`, newest first.
pub fn records(root: &Path) -> Vec<Delivery> {
    std::fs::read_to_string(root.join(FILE))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write(root: &Path, records: &[Delivery]) -> Result<(), String> {
    let text = serde_json::to_string_pretty(records).map_err(|e| e.to_string())?;
    let path = root.join(FILE);
    let temp = root.join(format!("{FILE}.tmp"));
    std::fs::write(&temp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&temp, &path).map_err(|e| e.to_string())
}

/// Adds an output that ended to the project's record, newest first.
pub fn record(root: &Path, delivery: Delivery) -> Result<(), String> {
    let mut all = records(root);
    all.retain(|d| d.job_id != delivery.job_id);
    all.insert(0, delivery);
    all.truncate(MAX_RECORDS);
    write(root, &all)
}

/// Takes `job_id` out of the project's record (its file stays).
pub fn forget(root: &Path, job_id: &str) -> Result<(), String> {
    let mut all = records(root);
    all.retain(|d| d.job_id != job_id);
    write(root, &all)
}

/// A line of the list as the editor shows it.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryView {
    #[serde(flatten)]
    pub delivery: Delivery,
    /// Made from an earlier revision than the edit's now.
    pub older: bool,
    /// Its file is gone.
    pub missing: bool,
}

/// The list for the project at `root` whose edit is at `revision`: what waits or runs, in
/// turn, then what ended, newest first. An output is older than the edit when `now` (what its
/// request would be made from now: `None` when that is gone, a short removed say) has another
/// fingerprint; one recorded without a fingerprint goes by the revision.
pub fn list(
    owner: &mut ExportOwner,
    root: &Path,
    revision: u64,
    now: &dyn Fn(&Request) -> Option<String>,
) -> Vec<DeliveryView> {
    let active = owner.active_for(root);
    let done: Vec<Delivery> = records(root)
        .into_iter()
        .filter(|d| !active.iter().any(|a| a.job_id == d.job_id))
        .collect();
    active
        .into_iter()
        .chain(done)
        .map(|delivery| {
            let finished = delivery.state == ExportState::Completed;
            let older = match &delivery.fingerprint {
                Some(made) => now(&delivery.request).as_ref() != Some(made),
                None => delivery.revision < revision,
            };
            DeliveryView {
                older: finished && older,
                missing: finished
                    && delivery
                        .output_path
                        .as_deref()
                        .is_none_or(|p| !Path::new(p).exists()),
                delivery,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn done(id: &str, revision: u64, path: Option<&Path>) -> Delivery {
        Delivery {
            job_id: id.into(),
            label: "Video".into(),
            request: Request {
                short_id: None,
                settings: ExportSettings::default(),
                file: None,
            },
            state: ExportState::Completed,
            revision,
            width: 1920,
            height: 1080,
            progress: 1.0,
            output_path: path.map(|p| p.to_string_lossy().into_owned()),
            failure: None,
            finished_at: None,
            fingerprint: None,
        }
    }

    #[test]
    fn one_export_runs_at_a_time_and_another_waits_only_when_queued() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        // Its file is there already, so an export of it ends at once (a collision).
        let dest = root.join("taken.mp4");
        std::fs::write(&dest, b"mp4").unwrap();
        let captured = |id: &str| CapturedExport {
            job_id: id.into(),
            root: root.to_path_buf(),
            document: Default::default(),
            dest: dest.clone(),
            temp: root.join(format!(".{id}.part")),
            settings: ExportSettings::default(),
            lease: Arc::new(std::fs::File::open(&dest).unwrap()),
        };
        let request = || Request {
            short_id: None,
            settings: ExportSettings::default(),
            file: None,
        };
        let gate = Arc::new(EncoderGate::new());
        let mut owner = ExportOwner::new();
        // An export running.
        let mut running = ExportStatus::idle();
        running.job_id = "a".into();
        running.state = ExportState::Running;
        owner.shared.lock().jobs.push(Job {
            delivery: done("a", 1, None),
            root: root.to_path_buf(),
            cancel: Default::default(),
            status: Arc::new(Mutex::new(running)),
            dest: Some(root.join("running.mp4")),
            captured: None,
        });
        assert!(owner.busy());

        let refused = owner.start(captured("b"), "Video", request(), false, Arc::clone(&gate));
        assert_eq!(refused.state, ExportState::Failed);
        assert!(matches!(
            refused.failure,
            Some(ExportFailure::EncoderBusy { .. })
        ));
        assert_eq!(
            owner.status_of(Some("b")).unwrap().state,
            ExportState::Failed
        );

        let queued = owner.start(
            captured("c"),
            "My short",
            request(),
            true,
            Arc::clone(&gate),
        );
        assert_eq!(queued.state, ExportState::Queued);
        // It runs (ending at once) and goes into the project's record.
        let started = std::time::Instant::now();
        while !records(root).iter().any(|d| d.job_id == "c") {
            assert!(
                started.elapsed().as_secs() < 10,
                "the queued export never ran"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let listed = list(&mut owner, root, 1, &|_| None);
        assert_eq!(listed[0].delivery.job_id, "a", "the running one first");
        assert_eq!(listed[1].delivery.label, "My short");
        assert_eq!(
            owner.waiting_destinations(root),
            [root.join("running.mp4")],
            "the one running keeps its name"
        );
        assert_eq!(listed[1].delivery.state, ExportState::Failed);
        assert!(listed[1]
            .delivery
            .failure
            .as_deref()
            .unwrap()
            .contains("exists"));
    }

    #[test]
    fn outputs_are_recorded_newest_first_and_marked_older_than_the_edit() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("out.mp4");
        std::fs::write(&file, b"mp4").unwrap();
        record(dir.path(), done("a", 3, Some(&file))).unwrap();
        record(dir.path(), done("b", 5, None)).unwrap();
        let mut owner = ExportOwner::new();
        let listed = list(&mut owner, dir.path(), 5, &|_| None);
        assert_eq!(
            listed
                .iter()
                .map(|v| v.delivery.job_id.as_str())
                .collect::<Vec<_>>(),
            ["b", "a"]
        );
        assert!(
            listed[1].older && !listed[1].missing,
            "made at 3, the edit is at 5"
        );
        assert!(!listed[0].older && listed[0].missing, "no file");
        // Made from what the edit still makes: not older, whatever the revision says.
        let mut edit = crate::project::revision::EditDocument::default();
        let made = fingerprint(&edit);
        edit.revision = 9;
        assert_eq!(
            fingerprint(&edit),
            made,
            "the revision number alone changes nothing"
        );
        record(
            dir.path(),
            Delivery {
                fingerprint: Some(made.clone()),
                ..done("c", 3, Some(&file))
            },
        )
        .unwrap();
        let same = list(&mut owner, dir.path(), 9, &|_| Some(made.clone()));
        assert!(!same[0].older);
        let moved = list(&mut owner, dir.path(), 9, &|_| Some("another".into()));
        assert!(moved[0].older);
        forget(dir.path(), "a").unwrap();
        assert_eq!(records(dir.path()).len(), 2, "b and c are left");
    }
}
