mod export;
mod file_ops;

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
            settings: Settings::default(),
            ai_panel: AiPanel::new(),
            export_rx: None,
            export_status: None,
        }
    }
}

impl eframe::App for MarkdownEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.settings.apply_theme(ctx);
        self.handle_dropped_files(ctx);
        self.preview_hovering_files(ctx);

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
                    if ui.button("Open…").clicked() {
                        self.open_file(PathBuf::from("example.md"));
                        ui.close_menu();
                    }
                    if ui.button("Save").clicked() {
                        self.save_file();
                        ui.close_menu();
                    }
                    if ui.button("Save As… (stub)").clicked() {
                        self.save_as(PathBuf::from("output.md"));
                        ui.close_menu();
                    }
                    ui.separator();
                    let exporting = self.export_rx.is_some();
                    ui.add_enabled_ui(!exporting, |ui| {
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
                if self.export_rx.is_some() {
                    ui.separator();
                    ui.label("Exporting…");
                }
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
                        let font_id = egui::FontId::monospace(self.settings.editor_font_size);
                        ui.add(
                            egui::TextEdit::multiline(&mut self.text)
                                .desired_width(f32::INFINITY)
                                .desired_rows(40)
                                .font(font_id),
                        );
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

        if self.show_settings {
            self.settings.show_window(ctx, &mut self.show_settings);
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
