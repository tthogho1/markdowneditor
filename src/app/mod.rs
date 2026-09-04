mod export;
mod file_ops;
mod find;
mod outline;
mod scroll_sync;
mod shortcuts;
mod text_util;
mod toolbar;

use crate::ai::AiPanel;
use crate::settings::Settings;
use eframe::egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use file_ops::PendingAction;
use scroll_sync::ScrollSync;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

const HELP_MD: &str = include_str!("../help.md");

pub struct MarkdownEditorApp {
    pub text: String,
    pub file_path: Option<PathBuf>,
    pub md_cache: CommonMarkCache,
    pub help_cache: CommonMarkCache,
    pub show_help: bool,
    pub show_settings: bool,
    pub settings: Settings,
    pub ai_panel: AiPanel,
    pub export_rx: Option<Receiver<Result<PathBuf, String>>>,
    pub export_status: Option<String>,
    pub open_rx: Option<Receiver<Option<(PathBuf, String)>>>,
    pub save_as_rx: Option<Receiver<Result<PathBuf, String>>>,
    // Toolbar
    pending_format: Option<toolbar::FormatAction>,
    editor_cursor: (usize, usize), // (primary, secondary) char indices
    /// Character range to select and scroll to in the editor next frame.
    pending_select: Option<(usize, usize)>,
    // Unsaved-changes tracking
    pub dirty: bool,
    /// Bumped on every edit; lets caches (find matches, outline) know the buffer changed.
    text_rev: u64,
    confirm: Option<PendingAction>,
    after_save: Option<PendingAction>,
    allow_close: bool,
    last_title: String,
    // Find & replace / outline / scroll sync
    find: find::FindReplace,
    show_outline: bool,
    scroll_sync: ScrollSync,
}

impl MarkdownEditorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::fonts::setup(&cc.egui_ctx);
        Self {
            text: String::from(
                "# Hello from Markdown Editor\n\nStart typing **Markdown** here.\n\n- Live preview\n- File open/save\n- Fast native Rust app\n",
            ),
            file_path: None,
            md_cache: CommonMarkCache::default(),
            help_cache: CommonMarkCache::default(),
            show_help: false,
            show_settings: false,
            settings: Settings::load(),
            ai_panel: AiPanel::new(),
            export_rx: None,
            export_status: None,
            open_rx: None,
            save_as_rx: None,
            pending_format: None,
            editor_cursor: (0, 0),
            pending_select: None,
            dirty: false,
            text_rev: 0,
            confirm: None,
            after_save: None,
            allow_close: false,
            last_title: String::new(),
            find: find::FindReplace::default(),
            show_outline: false,
            scroll_sync: ScrollSync::default(),
        }
    }

    /// Record that the buffer changed and now differs from what's on disk.
    fn mark_edited(&mut self) {
        self.dirty = true;
        self.text_rev = self.text_rev.wrapping_add(1);
    }

    /// Record that the buffer now matches what's on disk (saved, opened or reset).
    fn mark_saved(&mut self) {
        self.dirty = false;
        self.text_rev = self.text_rev.wrapping_add(1);
    }

    /// Window title, with a dot marking unsaved changes.
    fn title(&self) -> String {
        let name = self
            .file_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "untitled".to_string());
        format!(
            "{}{name} — Markdown Editor (Rust + egui)",
            if self.dirty { "● " } else { "" }
        )
    }

    /// Poll the background file/export threads.
    fn poll_background(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.open_rx {
            if let Ok(result) = rx.try_recv() {
                if let Some((path, content)) = result {
                    self.text = content;
                    self.settings.push_recent(path.clone());
                    self.settings.save();
                    self.file_path = Some(path);
                    self.mark_saved();
                }
                self.open_rx = None;
            }
        }

        if let Some(rx) = &self.save_as_rx {
            if let Ok(result) = rx.try_recv() {
                self.save_as_rx = None;
                match result {
                    Ok(path) => {
                        self.settings.push_recent(path.clone());
                        self.settings.save();
                        self.file_path = Some(path);
                        self.export_status = Some("保存しました".to_string());
                        self.mark_saved();
                        // A save the user asked for on the way to New/Open/Quit.
                        self.resume_after_save(ctx);
                    }
                    Err(e) => {
                        // A cancelled dialog also cancels whatever it was blocking.
                        self.after_save = None;
                        if e != "cancelled" {
                            self.export_status = Some(format!("Save failed: {e}"));
                        }
                    }
                }
            }
        }

        if let Some(rx) = &self.export_rx {
            if let Ok(result) = rx.try_recv() {
                self.export_status = Some(match result {
                    Ok(path) => format!("Exported: {}", path.display()),
                    Err(e) => format!("Export failed: {e}"),
                });
                self.export_rx = None;
            }
        }
    }

    /// Route the shortcuts consumed this frame to the actions they trigger.
    fn handle_shortcuts(&mut self, ctx: &egui::Context, hits: &shortcuts::Hits) {
        if hits.new_file {
            self.request(ctx, PendingAction::New);
        }
        if hits.open {
            self.request(ctx, PendingAction::Open);
        }
        if hits.save {
            self.save_file();
        }
        if hits.save_as {
            self.save_as_dialog();
        }
        if hits.find {
            self.find.open();
        }
        if hits.escape && self.find.visible {
            self.find.close();
        }
        if hits.find_next || hits.find_prev {
            self.find.visible = true;
            self.pending_select =
                self.find
                    .step_external(&self.text, self.text_rev, hits.find_next);
        }
        if hits.toggle_outline {
            self.show_outline = !self.show_outline;
        }
        if hits.bold {
            self.pending_format = Some(toolbar::FormatAction::Bold);
        }
        if hits.italic {
            self.pending_format = Some(toolbar::FormatAction::Italic);
        }
        if hits.link {
            self.pending_format = Some(toolbar::FormatAction::Link);
        }
    }

    /// Ask before closing the window when there are unsaved changes.
    fn handle_close_request(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.viewport().close_requested()) {
            return;
        }
        if self.dirty && !self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm = Some(PendingAction::Quit);
        }
    }

    fn menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if menu_item(ui, "New", &shortcuts::hint(false, "N")) {
                        self.request(ctx, PendingAction::New);
                        ui.close_menu();
                    }
                    ui.separator();
                    if menu_item(ui, "Open…", &shortcuts::hint(false, "O")) {
                        self.request(ctx, PendingAction::Open);
                        ui.close_menu();
                    }
                    if menu_item(ui, "Save", &shortcuts::hint(false, "S")) {
                        self.save_file();
                        ui.close_menu();
                    }
                    if menu_item(ui, "Save As…", &shortcuts::hint(true, "S")) {
                        self.save_as_dialog();
                        ui.close_menu();
                    }
                    ui.separator();

                    // Recent files submenu
                    let recents = self.settings.recent_files.clone();
                    ui.menu_button("Recent Files", |ui| {
                        if recents.is_empty() {
                            ui.label("(none)");
                        } else {
                            for path in &recents {
                                let label = path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| path.to_string_lossy().to_string());
                                if ui.button(&label).on_hover_text(path.to_string_lossy()).clicked() {
                                    self.request(ctx, PendingAction::OpenPath(path.clone()));
                                    ui.close_menu();
                                }
                            }
                            ui.separator();
                            if ui.button("Clear").clicked() {
                                self.settings.recent_files.clear();
                                self.settings.save();
                                ui.close_menu();
                            }
                        }
                    });

                    ui.separator();
                    let busy = self.export_rx.is_some();
                    ui.add_enabled_ui(!busy, |ui| {
                        if ui.button("Export HTML…").clicked() {
                            self.export_html();
                            ui.close_menu();
                        }
                        if ui.button("Export Text…").clicked() {
                            self.export_text();
                            ui.close_menu();
                        }
                        if ui.button("Export DOCX…").clicked() {
                            self.export_docx();
                            ui.close_menu();
                        }
                        if ui.button("Export PPTX…").clicked() {
                            self.export_pptx();
                            ui.close_menu();
                        }
                    });
                });

                ui.menu_button("Edit", |ui| {
                    if menu_item(ui, "Find & Replace…", &shortcuts::hint(false, "F")) {
                        self.find.open();
                        ui.close_menu();
                    }
                });

                ui.menu_button("View", |ui| {
                    let outline = menu_item_toggle(
                        ui,
                        "Outline",
                        &shortcuts::hint(true, "O"),
                        self.show_outline,
                    );
                    if outline {
                        self.show_outline = !self.show_outline;
                        ui.close_menu();
                    }
                    if menu_item_toggle(ui, "Sync scroll", "", self.scroll_sync.enabled) {
                        self.scroll_sync.enabled = !self.scroll_sync.enabled;
                        ui.close_menu();
                    }
                });

                ui.separator();
                if ui.button("AI").clicked() {
                    self.ai_panel.visible = true;
                }
                ui.separator();
                if ui.button("Settings").clicked() {
                    self.show_settings = true;
                }
                ui.separator();
                if ui.button("Help").clicked() {
                    self.show_help = true;
                }
            });
        });
    }

    fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let file_label = self
                    .file_path
                    .as_deref()
                    .and_then(|p| p.to_str())
                    .unwrap_or("(untitled)");
                ui.label(file_label);
                if self.dirty {
                    ui.label(egui::RichText::new("●").color(egui::Color32::from_rgb(220, 150, 0)))
                        .on_hover_text("未保存の変更があります");
                }

                if let Some(status) = &self.export_status {
                    ui.separator();
                    ui.label(status);
                }
                if self.export_rx.is_some() || self.open_rx.is_some() || self.save_as_rx.is_some() {
                    ui.separator();
                    ui.spinner();
                }

                // Right-aligned character / line counts
                let chars = self.text.chars().count();
                let lines = self.text.lines().count().max(1);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("{chars} 文字  {lines} 行"));
                });
            });
        });
    }

    /// The editor pane. Returns its rect so scroll sync can tell where the pointer is.
    fn editor_panel(&mut self, ctx: &egui::Context) -> egui::Rect {
        egui::SidePanel::left("editor_panel")
            .resizable(true)
            .default_width(600.0)
            .show(ctx, |ui| {
                ui.label("Markdown");
                ui.separator();

                let mut scroll = egui::ScrollArea::vertical().id_salt("editor_scroll");
                // A jump to a match/heading owns the scroll position this frame.
                if let (Some(offset), None) = (self.scroll_sync.apply_editor, self.pending_select) {
                    scroll = scroll.vertical_scroll_offset(offset);
                }

                let out = scroll.show(ui, |ui| {
                    // Apply pending toolbar format before rendering so the galley
                    // is built from the already-modified text.
                    if let Some(action) = self.pending_format.take() {
                        let (s, e) = self.editor_cursor;
                        let caret = toolbar::apply_format(&mut self.text, &action, s, e);
                        self.pending_select = Some((caret, caret));
                        self.mark_edited();
                    }

                    let font_id = egui::FontId::monospace(self.settings.editor_font_size);
                    let output = egui::TextEdit::multiline(&mut self.text)
                        .desired_width(f32::INFINITY)
                        .desired_rows(40)
                        .font(font_id)
                        .show(ui);

                    if output.response.changed() {
                        self.mark_edited();
                    }

                    // Keep cursor position up to date for the next toolbar click.
                    if let Some(cr) = &output.cursor_range {
                        self.editor_cursor =
                            (cr.primary.ccursor.index, cr.secondary.ccursor.index);
                    }

                    // Move the caret/selection to a requested range and scroll it into view.
                    if let Some((from, to)) = self.pending_select.take() {
                        use egui::text::{CCursor, CCursorRange};
                        use egui::widgets::text_edit::TextEditState;

                        let range = CCursorRange::two(CCursor::new(from), CCursor::new(to));
                        let mut state =
                            TextEditState::load(ctx, output.response.id).unwrap_or_default();
                        state.cursor.set_char_range(Some(range));
                        state.store(ctx, output.response.id);
                        output.response.request_focus();
                        self.editor_cursor = (to, from);

                        let rect = output
                            .galley
                            .pos_from_ccursor(CCursor::new(from))
                            .translate(output.galley_pos.to_vec2());
                        ui.scroll_to_rect(rect, Some(egui::Align::Center));
                    }
                });

                self.scroll_sync.record_editor(
                    out.state.offset.y,
                    out.content_size.y,
                    out.inner_rect.height(),
                );
            })
            .response
            .rect
    }

    /// The preview pane. Returns its rect so scroll sync can tell where the pointer is.
    fn preview_panel(&mut self, ctx: &egui::Context) -> egui::Rect {
        egui::CentralPanel::default()
            .show(ctx, |ui| {
                ui.label("Preview");
                ui.separator();

                let mut scroll = egui::ScrollArea::vertical().id_salt("preview_scroll");
                if let Some(offset) = self.scroll_sync.apply_preview {
                    scroll = scroll.vertical_scroll_offset(offset);
                }

                let out = scroll.show(ui, |ui| {
                    CommonMarkViewer::new().show(ui, &mut self.md_cache, &self.text);
                });

                self.scroll_sync.record_preview(
                    out.state.offset.y,
                    out.content_size.y,
                    out.inner_rect.height(),
                );
            })
            .response
            .rect
    }
}

impl eframe::App for MarkdownEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.settings.apply_theme(ctx);

        // Shortcuts are consumed before any widget is built, so the editor never sees them.
        // While the unsaved-changes prompt is up, it owns the keyboard.
        let hits = shortcuts::consume(ctx);
        if self.confirm.is_none() {
            self.handle_shortcuts(ctx, &hits);
        }

        self.handle_dropped_files(ctx);
        self.preview_hovering_files(ctx);
        self.poll_background(ctx);
        self.handle_close_request(ctx);

        let title = self.title();
        if title != self.last_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }

        self.menu_bar(ctx);

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(action) = toolbar::show(ui) {
                    self.pending_format = Some(action);
                }
            });
        });

        if self.find.visible {
            egui::TopBottomPanel::top("find_bar").show(ctx, |ui| {
                let out = self.find.show(ui, &mut self.text, self.text_rev);
                if out.text_changed {
                    self.mark_edited();
                }
                if let Some(range) = out.select {
                    self.pending_select = Some(range);
                }
            });
        }

        self.status_bar(ctx);

        if self.show_outline {
            egui::SidePanel::left("outline_panel")
                .resizable(true)
                .default_width(220.0)
                .show(ctx, |ui| {
                    ui.label("Outline");
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .id_salt("outline_scroll")
                        .show(ui, |ui| {
                            let headings = outline::parse(&self.text);
                            if let Some(char_start) =
                                outline::show(ui, &headings, self.editor_cursor.0)
                            {
                                self.pending_select = Some((char_start, char_start));
                            }
                        });
                });
        }

        let editor_rect = self.editor_panel(ctx);
        let preview_rect = self.preview_panel(ctx);

        // Decide which pane leads the scroll sync, for the next frame.
        let pointer = ctx.pointer_latest_pos();
        self.scroll_sync.sync(
            pointer.is_some_and(|p| editor_rect.contains(p)),
            pointer.is_some_and(|p| preview_rect.contains(p)),
        );

        let api_key = self.settings.openai_api_key.clone();
        if self.ai_panel.show(ctx, &mut self.text, &api_key) {
            self.mark_edited();
        }

        let settings_was_open = self.show_settings;
        if self.show_settings {
            self.settings.show_window(ctx, &mut self.show_settings);
        }
        // Save when the settings window is closed
        if settings_was_open && !self.show_settings {
            self.settings.save();
        }

        if self.show_help {
            egui::Window::new("Markdown Help")
                .open(&mut self.show_help)
                .resizable(true)
                .default_width(500.0)
                .default_height(600.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("help_scroll")
                        .show(ui, |ui| {
                            CommonMarkViewer::new()
                                .show(ui, &mut self.help_cache, HELP_MD);
                        });
                });
        }

        self.show_confirm_dialog(ctx);
    }
}

/// A menu entry with a right-aligned shortcut hint.
fn menu_item(ui: &mut egui::Ui, label: &str, hint: &str) -> bool {
    ui.add(egui::Button::new(label).shortcut_text(hint)).clicked()
}

/// A menu entry with a checkmark and a shortcut hint.
fn menu_item_toggle(ui: &mut egui::Ui, label: &str, hint: &str, on: bool) -> bool {
    let mark = if on { "✔ " } else { "   " };
    ui.add(egui::Button::new(format!("{mark}{label}")).shortcut_text(hint))
        .clicked()
}
