use eframe::egui;
use std::fs;
use std::path::PathBuf;

use super::MarkdownEditorApp;

impl MarkdownEditorApp {
    pub fn open_file(&mut self, path: PathBuf) {
        if let Ok(content) = fs::read_to_string(&path) {
            self.text = content;
            self.file_path = Some(path);
        }
    }

    pub fn save_file(&mut self) {
        if let Some(path) = &self.file_path.clone() {
            if let Err(e) = fs::write(path, &self.text) {
                eprintln!("Failed to save file: {}", e);
            }
        } else {
            eprintln!("No file path set; use Save As.");
        }
    }

    pub fn save_as(&mut self, path: PathBuf) {
        if let Err(e) = fs::write(&path, &self.text) {
            eprintln!("Failed to save file: {}", e);
        } else {
            self.file_path = Some(path);
        }
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
