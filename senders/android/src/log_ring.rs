use std::sync::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{Subscriber, Event};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;
use slint::{ComponentHandle, ModelRc, VecModel};
use std::rc::Rc;

use crate::{MainWindow, Bridge, LogEntry, LogLevel};

const LOG_RING_CAP: usize = 1024;
/// Minimum interval between pushes from the ring buffer to the Slint UI.
///
/// `on_event` is invoked synchronously from every tracing call (including the
/// firehose of GStreamer `Fixme`-level events forwarded by
/// `tracing_gstreamer::integrate_events`). Pushing on every event would clone
/// the full ring (up to `LOG_RING_CAP` entries) and queue a closure on the
/// Slint event loop thousands of times per second during active pipelines,
/// starving the UI thread. Instead we mark a `dirty` flag on every event and
/// let a single background task drain it at this cadence.
const PUSH_INTERVAL: std::time::Duration = std::time::Duration::from_millis(200);

#[derive(Clone)]
pub struct LogRing {
    entries: Arc<Mutex<std::collections::VecDeque<LogEntry>>>,
    dirty: Arc<AtomicBool>,
    ui_handle: slint::Weak<MainWindow>,
}

impl LogRing {
    pub fn new(ui_handle: slint::Weak<MainWindow>) -> Self {
        let this = Self {
            entries: Arc::new(Mutex::new(
                std::collections::VecDeque::with_capacity(LOG_RING_CAP),
            )),
            dirty: Arc::new(AtomicBool::new(false)),
            ui_handle,
        };
        this.spawn_pusher();
        this
    }

    /// Spawn the single background task that drains the `dirty` flag and
    /// pushes a snapshot of the ring buffer to the Slint UI at most once per
    /// `PUSH_INTERVAL`. Must be called from inside a tokio runtime context
    /// (see `_runtime_guard` in `android_main`).
    fn spawn_pusher(&self) {
        let entries = self.entries.clone();
        let dirty = self.dirty.clone();
        let ui_handle = self.ui_handle.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(PUSH_INTERVAL);
            loop {
                tick.tick().await;
                if !dirty.swap(false, Ordering::AcqRel) {
                    continue;
                }
                let snap: Vec<LogEntry> = match entries.lock() {
                    Ok(q) => q.iter().cloned().collect(),
                    Err(poisoned) => poisoned.into_inner().iter().cloned().collect(),
                };
                let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                    let model: ModelRc<LogEntry> = Rc::new(VecModel::from(snap)).into();
                    ui.global::<Bridge>().set_log_entries(model);
                });
            }
        });
    }

    pub fn clear(&self) {
        // `clear` is user-initiated from the Slint UI thread (the "Clear"
        // button in `debug_log_page.slint`). It is not re-entrant with
        // tracing, so a blocking `lock()` is safe and guarantees the user's
        // click actually empties the ring — unlike `try_lock()`, which would
        // silently no-op under contention with a concurrent `on_event`.
        // Poisoning is treated as a best-effort: clear the inner queue even
        // if a previous panic poisoned the mutex. The dirty flag is set so
        // the background pusher (see `spawn_pusher`) emits the cleared
        // snapshot to the UI on the next tick (≤ `PUSH_INTERVAL` later).
        match self.entries.lock() {
            Ok(mut q) => q.clear(),
            Err(poisoned) => poisoned.into_inner().clear(),
        }
        self.dirty.store(true, Ordering::Release);
    }
}

impl<S: Subscriber> Layer<S> for LogRing {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        if metadata.target().starts_with("fcastsender::log_ring") { return; }

        let mut visitor = LogEventVisitor::default();
        event.record(&mut visitor);

        let entry = LogEntry {
            level: match *metadata.level() {
                tracing::Level::TRACE => LogLevel::Trace,
                tracing::Level::DEBUG => LogLevel::Debug,
                tracing::Level::INFO  => LogLevel::Info,
                tracing::Level::WARN  => LogLevel::Warning,
                tracing::Level::ERROR => LogLevel::Error,
            },
            timestamp: chrono::Local::now().format("%H:%M:%S%.3f").to_string().into(),
            target: metadata.target().into(),
            message: visitor.message.into(),
        };

        if let Ok(mut q) = self.entries.try_lock() {
            if q.len() == LOG_RING_CAP {
                q.pop_front();
            }
            q.push_back(entry);
            self.dirty.store(true, Ordering::Release);
        }
        // Note: no synchronous UI push here — the background pusher spawned
        // in `LogRing::new` drains the dirty flag at `PUSH_INTERVAL` cadence.
        // This keeps `on_event` O(1) amortized even under GStreamer `Fixme`
        // event floods.
    }
}

#[derive(Default)]
struct LogEventVisitor { message: String }

impl tracing::field::Visit for LogEventVisitor {
    fn record_str(&mut self, f: &tracing::field::Field, v: &str) {
        if f.name() == "message" { self.message = v.to_owned(); }
    }
    fn record_debug(&mut self, f: &tracing::field::Field, v: &dyn std::fmt::Debug) {
        if f.name() == "message" { self.message = format!("{:?}", v); }
    }
}
