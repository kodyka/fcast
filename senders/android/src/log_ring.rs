use std::sync::Mutex;
use std::sync::Arc;
use tracing::{Subscriber, Event};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;
use slint::{ComponentHandle, ModelRc, VecModel};
use std::rc::Rc;

use crate::{MainWindow, Bridge, LogEntry, LogLevel};

const LOG_RING_CAP: usize = 1024;

#[derive(Clone)]
pub struct LogRing {
    entries: Arc<Mutex<std::collections::VecDeque<LogEntry>>>,
    ui_handle: slint::Weak<MainWindow>,
}

impl LogRing {
    pub fn new(ui_handle: slint::Weak<MainWindow>) -> Self {
        Self {
            entries: Arc::new(Mutex::new(
                std::collections::VecDeque::with_capacity(LOG_RING_CAP),
            )),
            ui_handle,
        }
    }

    pub fn clear(&self) {
        // `clear` is user-initiated from the Slint UI thread (the "Clear"
        // button in `debug_log_page.slint`). It is not re-entrant with
        // tracing, so a blocking `lock()` is safe and guarantees the user's
        // click actually empties the ring — unlike `try_lock()`, which would
        // silently no-op under contention with a concurrent `on_event`. The
        // guard is dropped before `push_to_ui` so the subsequent snapshot
        // re-lock cannot deadlock against ourselves. Poisoning is treated as
        // a best-effort: clear the inner queue even if a previous panic
        // poisoned the mutex.
        match self.entries.lock() {
            Ok(mut q) => q.clear(),
            Err(poisoned) => poisoned.into_inner().clear(),
        }
        self.push_to_ui();
    }

    fn push_to_ui(&self) {
        let snap: Vec<LogEntry> = if let Ok(q) = self.entries.try_lock() {
            q.iter().cloned().collect()
        } else {
            return;
        };
        let _ = self.ui_handle.upgrade_in_event_loop(move |ui| {
            let model: ModelRc<LogEntry> = Rc::new(VecModel::from(snap)).into();
            ui.global::<Bridge>().set_log_entries(model);
        });
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
        }
        self.push_to_ui();
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
