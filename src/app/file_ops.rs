use eframe::egui;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;

use super::MarkdownEditorApp;

impl MarkdownEditorApp {
    pub fn new_file(&mut self) {
        self.text = String::new();
        self.file_path = None;
        self.export_status = None;
    }

    pub fn open_file(&mut self, path: PathBuf) {
        if let Ok(content) = fs::read_to_string(&path) {
            self.text = content;
            self.settings.push_recent(path.clone());
            self.settings.save();
            self.file_path = Some(path);
        }
    }

    /// Show a native open dialog on a background thread.
    pub fn open_file_dialog(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.open_rx = Some(rx);
        std::thread::spawn(move || {
            let result = rfd::FileDialog::new()
                .add_filter("Markdown", &["md", "markdown"])
                .add_filter("Text", &["txt"])
                .add_filter("All files", &["*"])
                .pick_file()
                .and_then(|path| {
                    fs::read_to_string(&path).ok().map(|text| (path, text))
                });
            tx.send(result).ok();
        });
    }

    pub fn save_file(&mut self) {
        if let Some(path) = &self.file_path.clone() {
            if let Err(e) = fs::write(path, &self.text) {
                self.export_status = Some(format!("Save failed: {e}"));
            }
        } else {
            self.save_as_dialog();
        }
    }

    /// Show a native save dialog on a background thread.
    pub fn save_as_dialog(&mut self) {
        let text = self.text.clone();
        let default_name = self
            .file_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "untitled.md".to_string());

        let (tx, rx) = mpsc::channel();
        self.save_as_rx = Some(rx);
        std::thread::spawn(move || {
            let path = rfd::FileDialog::new()
                .add_filter("Markdown", &["md", "markdown"])
                .set_file_name(&default_name)
                .save_file();
            if let Some(path) = path {
                let result = fs::write(&path, &text)
                    .map(|_| path)
                    .map_err(|e| e.to_string());
                tx.send(result).ok();
            }
        });
    }

    /// Open the first file dropped onto the window.
    pub fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if let Some(path) = dropped.into_iter().find_map(|f| f.path) {
            self.open_file(path);
        }
    }

    /// Dim the window and show a hint while files are being dragged over it.
    pub fn preview_hovering_files(&self, ctx: &egui::Context) {
        use egui::{Align2, Color32, Id, LayerId, Order, TextStyle};

        let hovered = ctx.input(|i| i.raw.hovered_files.clone());
        if hovered.is_empty() {
            return;
        }

        let text = if hovered.len() == 1 {
            match hovered[0].path.as_ref() {
                Some(p) => format!("Drop to open:\n{}", p.display()),
                None => "Drop to open".to_owned(),
            }
        } else {
            "Drop a single file to open".to_owned()
        };

        let painter =
            ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));
        let screen_rect = ctx.screen_rect();
        painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(160));
        painter.text(
            screen_rect.center(),
            Align2::CENTER_CENTER,
            text,
            TextStyle::Heading.resolve(&ctx.style()),
            Color32::WHITE,
        );
    }
}
