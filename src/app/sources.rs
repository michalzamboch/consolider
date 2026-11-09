use super::{ConsoliderApp, theme};
use eframe::egui::{self, RichText};
use std::path::Path;

impl ConsoliderApp {
    pub(super) fn source_panel(&mut self, ui: &mut egui::Ui) {
        let panel_height = (ui.available_height() - 40.0).max(0.0);
        let hovering = ui.ctx().input(|input| !input.raw.hovered_files.is_empty());
        let card = if hovering {
            theme::card().stroke(egui::Stroke::new(1.0_f32, theme::ACCENT))
        } else {
            theme::card()
        };
        card.show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.set_min_height(panel_height);
            ui.horizontal(|ui| {
                ui.heading("Sources");
                if !self.sources.is_empty() {
                    ui.label(RichText::new(self.sources.len().to_string()).color(theme::MUTED));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(theme::button("+ Add folders")).clicked()
                        && let Some(paths) = rfd::FileDialog::new()
                            .set_title("Add source folders")
                            .pick_folders()
                    {
                        self.add_sources(paths);
                    }
                });
            });
            ui.add_space(16.0);
            let list_height = ui.available_height().max(80.0);
            if self.sources.is_empty() {
                theme::empty_state(ui, list_height, "Drop folders here", true);
                return;
            }

            let mut remove = None;
            egui::ScrollArea::vertical()
                .id_salt("sources")
                .max_height(list_height)
                .show(ui, |ui| {
                    for (index, path) in self.sources.iter().enumerate() {
                        if source_row(ui, path) {
                            remove = Some(index);
                        }
                        ui.add_space(4.0);
                    }
                });
            if let Some(index) = remove {
                self.sources.remove(index);
                self.invalidate();
            }
        });
    }
}

fn source_row(ui: &mut egui::Ui, path: &Path) -> bool {
    let mut remove = false;
    egui::Frame::new()
        .fill(theme::INSET)
        .corner_radius(10)
        .inner_margin(14)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                theme::folder_icon(ui, 25.0, theme::ACCENT);
                ui.add_space(4.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    remove = ui
                        .add(
                            egui::Button::new(RichText::new("×").size(20.0).color(theme::MUTED))
                                .frame(false)
                                .min_size(egui::vec2(24.0, 28.0)),
                        )
                        .on_hover_text("Remove folder")
                        .clicked();
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), 40.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.spacing_mut().item_spacing.y = 4.0;
                            ui.add(
                                egui::Label::new(
                                    RichText::new(
                                        path.file_name()
                                            .unwrap_or(path.as_os_str())
                                            .to_string_lossy(),
                                    )
                                    .strong(),
                                )
                                .truncate(),
                            );
                            ui.add(
                                egui::Label::new(
                                    RichText::new(path.display().to_string())
                                        .size(12.0)
                                        .color(theme::MUTED),
                                )
                                .truncate(),
                            )
                            .on_hover_text(path.display().to_string());
                        },
                    );
                });
            });
        });
    remove
}
