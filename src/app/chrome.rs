use super::{ConsoliderApp, theme, worker::Job};
use consolider::consolidate::{Phase, Request};
use eframe::egui::{self, RichText};

impl ConsoliderApp {
    pub(super) fn header(&self, context: &egui::Context) {
        egui::TopBottomPanel::top("header")
            .frame(
                egui::Frame::new()
                    .fill(theme::BACKGROUND)
                    .inner_margin(egui::Margin::symmetric(24, 20)),
            )
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    theme::union_icon(ui);
                    ui.add_space(4.0);
                    ui.label(RichText::new("Consolider").size(23.0).strong());
                });
            });
    }

    pub(super) fn footer(&mut self, context: &egui::Context) {
        egui::TopBottomPanel::bottom("footer")
            .frame(
                egui::Frame::new()
                    .fill(theme::BACKGROUND)
                    .inner_margin(egui::Margin::symmetric(24, 18)),
            )
            .show(context, |ui| {
                self.copy_progress(ui);

                // Errors can span multiple lines; routine status stays in the toolbar.
                let has_error =
                    self.notice_color == theme::ERROR || self.notice_color == theme::WARNING;
                if has_error && !self.notice.is_empty() {
                    ui.label(RichText::new(&self.notice).small().color(self.notice_color));
                    ui.add_space(4.0);
                }
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        self.actions(ui, context);
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            if self.worker.is_some() {
                                ui.spinner();
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(self.worker_status())
                                            .small()
                                            .color(theme::MUTED),
                                    )
                                    .truncate(),
                                );
                            } else if !has_error && !self.notice.is_empty() {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&self.notice)
                                            .small()
                                            .color(self.notice_color),
                                    )
                                    .truncate(),
                                );
                            }
                        });
                    });
                });
            });
    }

    fn copy_progress(&self, ui: &mut egui::Ui) {
        if self.worker.is_some()
            && let Some(progress) = &self.progress
            && progress.phase == Phase::Copying
            && let Some(plan) = &self.plan
        {
            let fraction = if plan.total_bytes() == 0 {
                progress.files as f32 / plan.files.len().max(1) as f32
            } else {
                progress.bytes as f32 / plan.total_bytes() as f32
            };
            ui.add(
                egui::ProgressBar::new(fraction.min(1.0))
                    .fill(theme::ACCENT)
                    .desired_height(3.0),
            );
            ui.add_space(6.0);
        }
    }

    fn worker_status(&self) -> String {
        if self.cancelling {
            return "Cancelling…".into();
        }
        let Some(progress) = &self.progress else {
            return self.notice.clone();
        };
        let phase = match progress.phase {
            Phase::Scanning => "Scanning",
            Phase::Verifying => "Verifying",
            Phase::Copying => "Copying",
        };
        format!(
            "{phase} · {} files · {}",
            progress.files,
            theme::bytes(progress.bytes)
        )
    }

    fn cancel_button(&mut self, ui: &mut egui::Ui) {
        let label = if self.cancelling {
            "Cancelling…"
        } else {
            "Cancel"
        };
        if ui
            .add_enabled(
                self.worker.is_some() && !self.cancelling,
                theme::button(label),
            )
            .clicked()
        {
            self.cancel();
        }
    }

    fn actions(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        self.cancel_button(ui);
        let busy = self.worker.is_some();
        if ui
            .add_enabled(
                !busy && self.plan.is_some() && !self.copied,
                theme::primary_button("Copy"),
            )
            .clicked()
            && let Some(plan) = self.plan.clone()
        {
            self.start(Job::Copy(plan), context);
        }
        let destination = self.destination();
        if ui
            .add_enabled(
                !busy && !self.sources.is_empty() && destination.is_some(),
                theme::button("Analyze"),
            )
            .clicked()
            && let Some(destination) = destination
        {
            self.plan = None;
            self.copied = false;
            self.start(
                Job::Analyze(Request {
                    sources: self.sources.clone(),
                    destination,
                }),
                context,
            );
        }
    }
}
