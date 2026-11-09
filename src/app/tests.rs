use super::ConsoliderApp;
use eframe::egui::{self, Pos2, Rect, Vec2, epaint::Shape};
use std::{
    fs, thread,
    time::{Duration, Instant},
};

fn render(
    app: &mut ConsoliderApp,
    context: &egui::Context,
    size: Vec2,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    context.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            system_theme: Some(egui::Theme::Light),
            events,
            ..Default::default()
        },
        |context| app.render(context),
    )
}

fn text_bounds(output: &egui::FullOutput, prefix: &str) -> Rect {
    output
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            Shape::Text(text) if text.galley.job.text.starts_with(prefix) => {
                Some(Rect::from_min_size(text.pos, text.galley.size()))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("Missing UI text: {prefix}"))
}

fn settled_frame(app: &mut ConsoliderApp, context: &egui::Context, size: Vec2) -> egui::FullOutput {
    render(app, context, size, vec![]);
    render(app, context, size, vec![]);
    render(app, context, size, vec![])
}

fn click(app: &mut ConsoliderApp, context: &egui::Context, position: Pos2, size: Vec2) {
    render(
        app,
        context,
        size,
        vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    render(
        app,
        context,
        size,
        vec![egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
}

fn await_worker(app: &mut ConsoliderApp) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.worker.is_some() {
        assert!(
            Instant::now() < deadline,
            "Worker timed out: {}",
            app.notice
        );
        app.poll_worker();
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn controls_fit_at_minimum_and_default_window_sizes() {
    for size in [egui::vec2(920.0, 680.0), egui::vec2(1120.0, 800.0)] {
        let context = egui::Context::default();
        let mut app = ConsoliderApp::new(&context);
        let output = settled_frame(&mut app, &context, size);
        assert!(
            context.style().visuals.dark_mode,
            "System theme must not reduce text contrast"
        );
        let screen = Rect::from_min_size(Pos2::ZERO, size);
        for label in [
            "Sources",
            "Destination",
            "+ Add folders",
            "Browse",
            "Analyze",
            "Copy",
        ] {
            let bounds = text_bounds(&output, label);
            assert!(
                screen.contains_rect(bounds),
                "{label} is outside {size:?}: {bounds:?}"
            );
        }
        assert!(text_bounds(&output, "Sources").max.x < text_bounds(&output, "Destination").min.x);
    }
}

#[test]
fn ui_analyze_and_copy_actions_complete_through_the_background_worker() {
    let temporary = tempfile::tempdir().unwrap();
    let source = temporary.path().join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("file.txt"), b"copied through the UI").unwrap();
    let context = egui::Context::default();
    let mut app = ConsoliderApp::new(&context);
    app.add_sources(vec![source]);
    app.destination_parent = temporary.path().display().to_string();
    let size = egui::vec2(1120.0, 800.0);
    let output = settled_frame(&mut app, &context, size);
    click(
        &mut app,
        &context,
        text_bounds(&output, "Analyze").center(),
        size,
    );
    await_worker(&mut app);
    assert!(app.plan.is_some(), "{}", app.notice);
    let output = settled_frame(&mut app, &context, size);
    assert!(text_bounds(&output, "Preview").is_positive());
    click(
        &mut app,
        &context,
        text_bounds(&output, "Copy").center(),
        size,
    );
    await_worker(&mut app);
    assert!(app.copied, "{}", app.notice);
    assert_eq!(
        fs::read(temporary.path().join("Consolidated/file.txt")).unwrap(),
        b"copied through the UI"
    );
}

#[test]
fn source_rows_fit_and_can_be_removed_after_preview() {
    use consolider::consolidate::{Request, analyze};
    use std::sync::{Arc, atomic::AtomicBool};

    let temporary = tempfile::tempdir().unwrap();
    let name = "A source folder with a long name that should fit within its panel";
    let source = temporary.path().join(name);
    fs::create_dir(&source).unwrap();
    fs::write(source.join("note.txt"), b"data").unwrap();
    let context = egui::Context::default();
    let mut app = ConsoliderApp::new(&context);
    app.add_sources(vec![source]);
    app.destination_parent = temporary.path().display().to_string();
    app.plan = Some(Arc::new(
        analyze(
            &Request {
                sources: app.sources.clone(),
                destination: app.destination().unwrap(),
            },
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap(),
    ));
    let size = egui::vec2(920.0, 680.0);
    let output = settled_frame(&mut app, &context, size);
    let remove = text_bounds(&output, "×");
    assert!(text_bounds(&output, name).max.x < remove.min.x);
    assert!(remove.max.x < text_bounds(&output, "Destination").min.x);
    click(&mut app, &context, remove.center(), size);
    assert!(app.sources.is_empty());
    assert!(app.plan.is_none());
}
