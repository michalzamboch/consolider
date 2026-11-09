use eframe::egui::{self, Color32, FontFamily, FontId, Stroke, TextStyle};

pub const BACKGROUND: Color32 = Color32::from_rgb(16, 17, 21);
pub const SURFACE: Color32 = Color32::from_rgb(24, 26, 32);
pub const INSET: Color32 = Color32::from_rgb(30, 32, 39);
pub const BORDER: Color32 = Color32::from_rgb(44, 47, 57);
pub const MUTED: Color32 = Color32::from_rgb(146, 149, 163);
pub const ACCENT: Color32 = Color32::from_rgb(190, 173, 255);
pub const WARNING: Color32 = Color32::from_rgb(246, 193, 111);
pub const ERROR: Color32 = Color32::from_rgb(255, 148, 148);

pub fn apply(context: &egui::Context) {
    context.set_theme(egui::Theme::Dark);
    let mut style = (*context.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = BACKGROUND;
    style.visuals.window_fill = SURFACE;
    style.visuals.extreme_bg_color = INSET;
    style.visuals.faint_bg_color = INSET;
    style.visuals.selection.bg_fill = Color32::from_rgb(72, 60, 109);
    style.visuals.widgets.inactive.bg_fill = INSET;
    style.visuals.widgets.inactive.weak_bg_fill = INSET;
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(45, 47, 59);
    style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(45, 47, 59);
    style.visuals.widgets.active.bg_fill = Color32::from_rgb(59, 52, 80);
    style.visuals.widgets.active.weak_bg_fill = Color32::from_rgb(59, 52, 80);
    style.visuals.override_text_color = Some(Color32::from_rgb(237, 237, 243));
    style.visuals.widgets.inactive.corner_radius = 8.into();
    style.visuals.widgets.hovered.corner_radius = 8.into();
    style.visuals.widgets.active.corner_radius = 8.into();
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    style.spacing.item_spacing = egui::vec2(12.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 9.0);
    style
        .text_styles
        .insert(TextStyle::Body, FontId::new(14.0, FontFamily::Proportional));
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(14.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(19.0, FontFamily::Proportional),
    );
    context.set_style_of(egui::Theme::Dark, style);
}

pub fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(16)
        .inner_margin(20)
}

pub fn button(label: &'static str) -> egui::Button<'static> {
    egui::Button::new(label)
        .corner_radius(8)
        .min_size(egui::vec2(96.0, 38.0))
}

pub fn primary_button(label: &'static str) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(label).color(BACKGROUND).strong())
        .corner_radius(8)
        .fill(ACCENT)
        .stroke(Stroke::NONE)
        .min_size(egui::vec2(112.0, 38.0))
}

pub fn union_icon(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::hover());
    let first = egui::Rect::from_min_size(rect.min + egui::vec2(1.0, 3.0), egui::vec2(19.0, 22.0));
    let second = first.translate(egui::vec2(9.0, 3.0));
    ui.painter()
        .rect_filled(first, 5, ACCENT.gamma_multiply(0.25));
    ui.painter()
        .rect_filled(second, 5, ACCENT.gamma_multiply(0.9));
}

pub fn folder_icon(ui: &mut egui::Ui, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let tab = egui::Rect::from_min_size(
        rect.min + egui::vec2(size * 0.06, size * 0.2),
        egui::vec2(size * 0.44, size * 0.24),
    );
    let body = egui::Rect::from_min_max(
        rect.min + egui::vec2(size * 0.06, size * 0.34),
        rect.max - egui::vec2(size * 0.06, size * 0.14),
    );
    ui.painter().rect_filled(tab, 3, color.gamma_multiply(0.35));
    ui.painter().rect_filled(body, 3, color.gamma_multiply(0.2));
    ui.painter().rect_stroke(
        body,
        3,
        Stroke::new(1.0_f32, color.gamma_multiply(0.7)),
        egui::StrokeKind::Inside,
    );
}

pub fn empty_state(ui: &mut egui::Ui, height: f32, label: &str, folder: bool) {
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), height),
        egui::Layout::top_down(egui::Align::Center),
        |ui| {
            let content_height = if folder { 80.0 } else { 20.0 };
            ui.add_space(((height - content_height) * 0.5).max(0.0));
            if folder {
                folder_icon(ui, 46.0, MUTED);
            }
            ui.label(egui::RichText::new(label).size(13.0).color(MUTED));
        },
    );
}

pub fn bytes(value: u64) -> String {
    let units = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut size = value as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < units.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{value} B")
    } else {
        format!("{size:.1} {}", units[unit])
    }
}
