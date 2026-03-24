use serde::Serialize;
use tauri::{AppHandle, Emitter};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

#[derive(Debug, Clone, Serialize)]
pub struct ProgressEvent {
    pub stage: String,
    pub zip_name: String,
    pub zip_index: usize,
    pub zip_total: usize,
    pub files_processed: u64,
    pub files_total: u64,
    pub bytes_processed: u64,
    pub bytes_total: u64,
    pub elapsed_secs: f64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StageEvent {
    pub stage: String,
    pub zip_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogLine {
    pub level: String,
    pub message: String,
}

pub struct ProgressTracker {
    app_handle: AppHandle,
    pub paused: AtomicBool,
    pub cancelled: AtomicBool,
    start_time: Mutex<Option<Instant>>,
    pub files_processed: AtomicU64,
    pub bytes_processed: AtomicU64,
    last_emit_ms: AtomicU64,
}

impl ProgressTracker {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            paused: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
            start_time: Mutex::new(None),
            files_processed: AtomicU64::new(0),
            bytes_processed: AtomicU64::new(0),
            last_emit_ms: AtomicU64::new(0),
        }
    }

    pub fn start(&self) {
        *self.start_time.lock().unwrap() = Some(Instant::now());
        self.files_processed.store(0, Ordering::Relaxed);
        self.bytes_processed.store(0, Ordering::Relaxed);
    }

    fn elapsed_secs(&self) -> f64 {
        self.start_time
            .lock()
            .unwrap()
            .map(|s| s.elapsed().as_secs_f64())
            .unwrap_or(0.0)
    }

    fn elapsed_ms(&self) -> u64 {
        self.start_time
            .lock()
            .unwrap()
            .map(|s| s.elapsed().as_millis() as u64)
            .unwrap_or(0)
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Throttled progress emit — at most every 100ms to avoid flooding the UI.
    pub fn emit_progress(
        &self,
        stage: &str,
        zip_name: &str,
        zip_index: usize,
        zip_total: usize,
        files_processed: u64,
        files_total: u64,
        bytes_processed: u64,
        bytes_total: u64,
        message: &str,
        force: bool,
    ) {
        let now_ms = self.elapsed_ms();
        let last = self.last_emit_ms.load(Ordering::Relaxed);

        // Throttle: only emit every 100ms unless forced or it's the last item
        if !force && files_processed < files_total && now_ms.saturating_sub(last) < 100 {
            return;
        }
        self.last_emit_ms.store(now_ms, Ordering::Relaxed);

        let _ = self.app_handle.emit(
            "progress",
            ProgressEvent {
                stage: stage.to_string(),
                zip_name: zip_name.to_string(),
                zip_index,
                zip_total,
                files_processed,
                files_total,
                bytes_processed,
                bytes_total,
                elapsed_secs: self.elapsed_secs(),
                message: message.to_string(),
            },
        );
    }

    pub fn emit_stage(&self, stage: &str, zip_name: &str) {
        let _ = self.app_handle.emit(
            "stage_changed",
            StageEvent {
                stage: stage.to_string(),
                zip_name: zip_name.to_string(),
            },
        );
    }

    pub fn emit_log(&self, level: &str, message: &str) {
        let _ = self.app_handle.emit(
            "log_line",
            LogLine {
                level: level.to_string(),
                message: message.to_string(),
            },
        );
    }

    pub fn emit_zip_status_changed(&self) {
        let _ = self.app_handle.emit("zip_status_changed", ());
    }

    pub fn emit_complete(&self) {
        let _ = self.app_handle.emit("complete", ());
    }

    pub fn emit_error(&self, message: &str) {
        let _ = self.app_handle.emit("error", message.to_string());
    }

    pub fn wait_if_paused(&self) {
        while self.is_paused() && !self.is_cancelled() {
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    }
}
