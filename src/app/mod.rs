mod chrome;
mod destination;
mod sources;
#[cfg(test)]
mod tests;
mod theme;
mod worker;

use consolider::consolidate::{Error, Plan, Progress};
use eframe::egui::{self, Color32};
use std::{
    path::{Component, Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use worker::{Event, Job, Outcome, Worker};

pub struct ConsoliderApp {
    sources: Vec<PathBuf>,
    destination_parent: String,
    picked_parent: Option<PathBuf>,
    destination_name: String,
    plan: Option<Arc<Plan>>,
    worker: Option<Worker>,
    progress: Option<Progress>,
    notice: String,
    notice_color: Color32,
    copied: bool,
    copying: bool,
    cancelling: bool,
}

impl ConsoliderApp {
    pub fn new(context: &egui::Context) -> Self {
        theme::apply(context);
        Self {
            sources: Vec::new(),
            destination_parent: String::new(),
            picked_parent: None,
            destination_name: "Consolidated".into(),
            plan: None,
            worker: None,
            progress: None,
            notice: String::new(),
            notice_color: theme::MUTED,
            copied: false,
            copying: false,
            cancelling: false,
        }
    }

    fn invalidate(&mut self) {
        self.plan = None;
        self.copied = false;
        self.notice.clear();
        self.notice_color = theme::MUTED;
    }

    fn destination(&self) -> Option<PathBuf> {
        let name = Path::new(&self.destination_name);
        let mut components = name.components();
        if self.destination_parent.trim().is_empty()
            || !matches!(components.next(), Some(Component::Normal(_)))
            || components.next().is_some()
            || (cfg!(windows) && self.destination_name.ends_with(['.', ' ']))
        {
            return None;
        }
        let parent = self
            .picked_parent
            .as_deref()
            .unwrap_or_else(|| Path::new(&self.destination_parent));
        Some(parent.join(name))
    }

    fn add_sources(&mut self, paths: Vec<PathBuf>) {
        let mut changed = false;
        for path in paths {
            if path.is_dir() && !self.sources.contains(&path) {
                self.sources.push(path);
                changed = true;
            }
        }
        if changed {
            self.invalidate();
        }
    }

    fn start(&mut self, job: Job, context: &egui::Context) {
        self.copying = matches!(&job, Job::Copy(_));
        self.progress = None;
        self.cancelling = false;
        self.notice_color = theme::MUTED;
        self.notice = if self.copying {
            "Verifying…"
        } else {
            "Scanning…"
        }
        .into();
        self.worker = Some(Worker::start(job, context.clone()));
    }

    fn poll_worker(&mut self) {
        while let Some(event) = self.worker.as_ref().and_then(Worker::poll) {
            match event {
                Event::Progress(progress) => self.progress = Some(progress),
                Event::Finished(result) => {
                    self.worker = None;
                    self.progress = None;
                    self.cancelling = false;
                    match result {
                        Ok(Outcome::Analyzed(plan)) => {
                            self.notice.clear();
                            self.notice_color = theme::ACCENT;
                            self.plan = Some(plan);
                            self.copied = false;
                        }
                        Ok(Outcome::Copied(plan)) => {
                            self.notice = format!(
                                "Copied {} files · {}",
                                plan.files.len(),
                                theme::bytes(plan.total_bytes())
                            );
                            self.notice_color = theme::ACCENT;
                            self.copied = true;
                        }
                        Err(error) => {
                            let cancelled = matches!(error, Error::Cancelled);
                            self.notice = error.to_string();
                            if self.copying {
                                self.notice.push_str(
                                    " Completed copies remain. Choose a new destination to retry.",
                                );
                            }
                            self.notice_color = if cancelled {
                                theme::WARNING
                            } else {
                                theme::ERROR
                            };
                            self.plan = None;
                        }
                    }
                    break;
                }
            }
        }
    }
}

impl ConsoliderApp {
    fn render(&mut self, context: &egui::Context) {
        self.poll_worker();
        if self.worker.is_none() {
            let paths = context.input(|input| {
                input
                    .raw
                    .dropped_files
                    .iter()
                    .filter_map(|file| file.path.clone())
                    .collect::<Vec<_>>()
            });
            self.add_sources(paths);
        } else {
            context.request_repaint_after(Duration::from_millis(100));
        }
        self.header(context);
        self.footer(context);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::BACKGROUND)
                    .inner_margin(egui::Margin::symmetric(24, 0)),
            )
            .show(context, |ui| {
                ui.add_enabled_ui(self.worker.is_none(), |ui| {
                    ui.columns(2, |columns| {
                        self.source_panel(&mut columns[0]);
                        self.destination_panel(&mut columns[1]);
                    });
                });
            });
    }
}

impl eframe::App for ConsoliderApp {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.render(context);
    }
}
