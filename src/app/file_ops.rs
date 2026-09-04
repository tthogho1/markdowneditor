use eframe::egui;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;

use super::MarkdownEditorApp;

/// An action that discards the current buffer, so it has to wait for the
/// "unsaved changes" prompt when the document is dirty.
#[derive(Clone, PartialEq)]
pub enum PendingAction {
    New,
    Open,
    OpenPath(PathBuf),
    Quit,
}

impl MarkdownEditorApp {
    /// Run `action`, first prompting to save if the buffer has unsaved changes.
    pub fn request(&mut self, ctx: &egui::Context, action: PendingAction) {
        if self.dirty {
            self.confirm = Some(action);
        } else {
            self.perform(ctx, action);
        }
    }

    fn perform(&mut self, ctx: &egui::Context, action: PendingAction) {
        match action {
            PendingAction::New => self.new_file(),
            PendingAction::Open => self.open_file_dialog(),
            PendingAction::OpenPath(path) => self.open_file(path),
            PendingAction::Quit => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    /// The "unsaved changes" prompt. Shown while `self.confirm` is set.
    pub fn show_confirm_dialog(&mut self, ctx: &egui::Context) {
        let Some(action) = self.confirm.clone() else {
            return;
        };

        let mut choice = None;
        egui::Window::new("未保存の変更")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("変更が保存されていません。どうしますか？");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("保存して続行").clicked() {
                        choice = Some(Choice::Save);
                    }
                    if ui.button("破棄して続行").clicked() {
                        choice = Some(Choice::Discard);
                    }
                    if ui.button("キャンセル").clicked() {
                        choice = Some(Choice::Cancel);
                    }
                });
            });

        match choice {
            None => {}
            Some(Choice::Cancel) => self.confirm = None,
            Some(Choice::Discard) => {
                self.confirm = None;
                self.dirty = false;
                self.perform(ctx, action);
            }
            Some(Choice::Save) => {
                self.confirm = None;
                if self.file_path.is_some() {
                    self.save_file();
                    // Only continue if the write actually succeeded.
                    if !self.dirty {
                        self.perform(ctx, action);
                    }
                } else {
                    // No path yet: run Save As and resume once its result arrives.
                    self.after_save = Some(action);
                    self.save_as_dialog();
                }
            }
        }
    }

    /// Resume a pending action after an async Save As finished.
    pub fn resume_after_save(&mut self, ctx: &egui::Context) {
        if let Some(action) = self.after_save.take() {
            self.perform(ctx, action);
        }
    }

    pub fn new_file(&mut self) {
        self.text = String::new();
        self.file_path = None;
        self.export_status = None;
        self.mark_saved();
    }

    pub fn open_file(&mut self, path: PathBuf) {
        match fs::read_to_string(&path) {
            Ok(content) => {
                self.text = content;
                self.settings.push_recent(path.clone());
                self.settings.save();
                self.file_path = Some(path);
                self.mark_saved();
            }
            Err(e) => self.export_status = Some(format!("Open failed: {e}")),
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
            match fs::write(path, &self.text) {
                Ok(()) => {
                    self.dirty = false;
                    self.export_status = Some("保存しました".to_string());
                }
                Err(e) => self.export_status = Some(format!("Save failed: {e}")),
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
            let result = match path {
                Some(path) => fs::write(&path, &text)
                    .map(|_| path)
                    .map_err(|e| e.to_string()),
                None => Err(String::from("cancelled")),
            };
            tx.send(result).ok();
        });
    }

    /// Open the first file dropped onto the window, guarding unsaved changes.
    pub fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if let Some(path) = dropped.into_iter().find_map(|f| f.path) {
            self.request(ctx, PendingAction::OpenPath(path));
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

enum Choice {
    Save,
    Discard,
    Cancel,
}
