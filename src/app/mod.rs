mod export;
mod file_ops;
mod toolbar;

use crate::ai::AiPanel;
use crate::settings::Settings;
use eframe::egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
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
        }
    }
}

impl eframe::App for MarkdownEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.settings.apply_theme(ctx);
        self.handle_dropped_files(ctx);
        self.preview_hovering_files(ctx);

        // Poll file open result
        if let Some(rx) = &self.open_rx {
            if let Ok(result) = rx.try_recv() {
                if let Some((path, content)) = result {
                    self.text = content;
                    self.settings.push_recent(path.clone());
                    self.settings.save();
                    self.file_path = Some(path);
                }
                self.open_rx = None;
            }
        }

        // Poll save-as result
        if let Some(rx) = &self.save_as_rx {
            if let Ok(result) = rx.try_recv() {
                match result {
                    Ok(path) => {
                        self.settings.push_recent(path.clone());
                        self.settings.save();
                        self.file_path = Some(path);
                        self.export_status = Some("Saved".to_string());
                    }
                    Err(e) => self.export_status = Some(format!("Save failed: {e}")),
                }
                self.save_as_rx = None;
            }
        }

        // Poll export result from background thread
        if let Some(rx) = &self.export_rx {
            if let Ok(result) = rx.try_recv() {
                self.export_status = Some(match result {
                    Ok(path) => format!("Exported: {}", path.display()),
                    Err(e) => format!("Export failed: {}", e),
                });
                self.export_rx = None;
            }
        }

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.new_file();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Open…").clicked() {
                        self.open_file_dialog();
                        ui.close_menu();
                    }
                    if ui.button("Save").clicked() {
                        self.save_file();
                        ui.close_menu();
                    }
                    if ui.button("Save As…").clicked() {
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
                                    self.open_file(path.clone());
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

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(action) = toolbar::show(ui) {
                    self.pending_format = Some(action);
                }
            });
        });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let file_label = self
                    .file_path
                    .as_deref()
                    .and_then(|p| p.to_str())
                    .unwrap_or("(untitled)");
                ui.label(file_label);

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

        egui::SidePanel::left("editor_panel")
            .resizable(true)
            .default_width(600.0)
            .show(ctx, |ui| {
                ui.label("Markdown");
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt("editor_scroll")
                    .show(ui, |ui| {
                        // Apply pending toolbar format before rendering so the galley
                        // is built from the already-modified text.
                        let new_cursor = self.pending_format.take().map(|action| {
                            let (s, e) = self.editor_cursor;
                            toolbar::apply_format(&mut self.text, &action, s, e)
                        });

                        let font_id = egui::FontId::monospace(self.settings.editor_font_size);
                        let output = egui::TextEdit::multiline(&mut self.text)
                            .desired_width(f32::INFINITY)
                            .desired_rows(40)
                            .font(font_id)
                            .show(ui);

                        // Keep cursor position up to date for the next toolbar click.
                        if let Some(cr) = &output.cursor_range {
                            self.editor_cursor =
                                (cr.primary.ccursor.index, cr.secondary.ccursor.index);
                        }

                        // After format application: set cursor and restore focus.
                        if let Some(char_idx) = new_cursor {
                            use egui::text::{CCursor, CCursorRange};
                            use egui::widgets::text_edit::TextEditState;

                            let ccursor = CCursor::new(char_idx);
                            let range = CCursorRange::one(ccursor);
                            let mut state =
                                TextEditState::load(ctx, output.response.id)
                                    .unwrap_or_default();
                            state.cursor.set_char_range(Some(range));
                            state.store(ctx, output.response.id);
                            output.response.request_focus();
                            self.editor_cursor = (char_idx, char_idx);
                        }
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Preview");
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("preview_scroll")
                .show(ui, |ui| {
                    CommonMarkViewer::new()
                        .show(ui, &mut self.md_cache, &self.text);
                });
        });

        self.ai_panel.show(ctx, &mut self.text, &self.settings.openai_api_key.clone());

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
    }
}
