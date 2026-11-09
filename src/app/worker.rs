use consolider::consolidate::{self, Plan, Progress, Request, Result};
use eframe::egui;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, TryRecvError},
    },
    thread,
    time::{Duration, Instant},
};

pub enum Job {
    Analyze(Request),
    Copy(Arc<Plan>),
}
pub enum Outcome {
    Analyzed(Arc<Plan>),
    Copied(Arc<Plan>),
}
pub enum Event {
    Progress(Progress),
    Finished(Result<Outcome>),
}

pub struct Worker {
    receiver: Receiver<Event>,
    cancel: Arc<AtomicBool>,
}

impl Worker {
    pub fn start(job: Job, context: egui::Context) -> Self {
        let (sender, receiver) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        thread::spawn(move || {
            let mut last_update = Instant::now() - Duration::from_secs(1);
            let report = |progress| {
                if last_update.elapsed() >= Duration::from_millis(60) {
                    let _ = sender.send(Event::Progress(progress));
                    context.request_repaint();
                    last_update = Instant::now();
                }
            };
            let result = match job {
                Job::Analyze(request) => consolidate::analyze(&request, &worker_cancel, report)
                    .map(|plan| Outcome::Analyzed(Arc::new(plan))),
                Job::Copy(plan) => consolidate::execute(&plan, &worker_cancel, report)
                    .map(|()| Outcome::Copied(plan)),
            };
            let _ = sender.send(Event::Finished(result));
            context.request_repaint();
        });
        Self { receiver, cancel }
    }

    pub fn poll(&self) -> Option<Event> {
        match self.receiver.try_recv() {
            Ok(event) => Some(event),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Event::Finished(Err(
                consolidate::Error::Invalid("The background worker stopped unexpectedly.".into()),
            ))),
        }
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel();
    }
}
