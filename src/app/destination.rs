use super::{ConsoliderApp, theme};
use consolider::consolidate::PlannedFile;
use eframe::egui::{self, RichText};

impl ConsoliderApp {
    pub(super) fn destination_panel(&mut self, ui: &mut egui::Ui) {
        let panel_height = (ui.available_height() - 40.0).max(0.0);
        theme::card().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.set_min_height(panel_height);
            ui.heading("Destination");
            ui.add_space(20.0);
            ui.label(RichText::new("Location").small().color(theme::MUTED));
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(theme::button("Browse")).clicked()
                        && let Some(path) = rfd::FileDialog::new()
                            .set_title("Choose destination location")
                            .pick_folder()
                    {
                        self.destination_parent = path.display().to_string();
                        self.picked_parent = Some(path);
                        self.invalidate();
                    }
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut self.destination_parent)
                                .hint_text("Choose a folder")
                                .margin(egui::vec2(10.0, 10.0))
                                .desired_width(ui.available_width()),
                        )
                        .changed()
                    {
                        self.picked_parent = None;
                        self.invalidate();
                    }
                });
            });
            ui.add_space(4.0);
            ui.label(RichText::new("Folder name").small().color(theme::MUTED));
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.destination_name)
                    .margin(egui::vec2(10.0, 10.0))
                    .desired_width(f32::INFINITY),
            );
            if response.changed() {
                self.invalidate();
            }
            if let Some(path) = self.destination() {
                response.on_hover_text(path.display().to_string());
            }
            ui.add_space(20.0);
            ui.separator();
            ui.add_space(12.0);
            self.preview(ui);
        });
    }

    fn preview(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Preview").strong());
            if let Some(plan) = &self.plan {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "{} files · {}",
                            plan.files.len(),
                            theme::bytes(plan.total_bytes())
                        ))
                        .small()
                        .color(theme::MUTED),
                    );
                });
            }
        });

        let Some(plan) = &self.plan else {
            theme::empty_state(
                ui,
                ui.available_height().max(60.0),
                "Analyze to preview",
                false,
            );
            return;
        };
        ui.add_space(4.0);
        let mut details = Vec::new();
        if plan.duplicates > 0 {
            details.push(format!("{} duplicates skipped", plan.duplicates));
        }
        if plan.renamed_files() > 0 {
            details.push(format!("{} versions kept", plan.renamed_files()));
        }
        if !details.is_empty() {
            ui.label(RichText::new(details.join(" · ")).small().color(theme::MUTED))
                .on_hover_text("Identical paths and contents are combined. Different versions are kept with a conflict suffix.");
        }
        if !plan.warnings.is_empty() {
            egui::CollapsingHeader::new(
                RichText::new(format!("{} skipped items", plan.warnings.len()))
                    .small()
                    .color(theme::WARNING),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("warnings")
                    .max_height(100.0)
                    .show(ui, |ui| {
                        for warning in &plan.warnings {
                            ui.label(RichText::new(warning).small().color(theme::MUTED));
                        }
                    });
            });
        }
        if plan.files.is_empty() {
            theme::empty_state(ui, ui.available_height().max(60.0), "No files", false);
            return;
        }
        egui::ScrollArea::vertical()
            .id_salt("preview")
            .max_height(ui.available_height().max(40.0))
            .show_rows(ui, 28.0, plan.files.len(), |ui, range| {
                for index in range {
                    file_row(ui, &plan.files[index]);
                }
            });
    }
}

fn file_row(ui: &mut egui::Ui, file: &PlannedFile) {
    ui.horizontal(|ui| {
        let color = if file.renamed() {
            theme::WARNING
        } else {
            theme::MUTED
        };
        let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 18.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 2.5, color);
        ui.add(
            egui::Label::new(
                RichText::new(file.output_path.display().to_string())
                    .size(13.0)
                    .color(color),
            )
            .truncate(),
        )
        .on_hover_text(format!(
            "Source: {}\nOutput: {}",
            file.source.display(),
            file.output_path.display()
        ));
    });
}
