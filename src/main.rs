mod ai;
mod app;
mod fonts;
mod pptx_export;
mod settings;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Markdown Editor (Rust + egui)")
            .with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Markdown Editor",
        options,
        Box::new(|cc| Ok(Box::new(app::MarkdownEditorApp::new(cc)))),
    )
}
