use knit_md_docx::PageSetup;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum Theme {
    Light,
    Dark,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum DocxPage {
    A4,
    Letter,
}

impl DocxPage {
    pub fn to_page_setup(&self) -> PageSetup {
        match self {
            DocxPage::A4 => PageSetup::A4,
            DocxPage::Letter => PageSetup::LETTER,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub theme: Theme,
    pub editor_font_size: f32,
    pub docx_page: DocxPage,
    pub openai_api_key: String,
    #[serde(skip)]
    pub show_api_key: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::Light,
            editor_font_size: 14.0,
            docx_page: DocxPage::A4,
            openai_api_key: String::new(),
            show_api_key: false,
        }
    }
}

impl Settings {
    /// Load from config file, falling back to defaults if missing or invalid.
    pub fn load() -> Self {
        Self::try_load().unwrap_or_default()
    }

    /// Save to config file. Silently ignores errors.
    pub fn save(&self) {
        let Some(path) = Self::config_path() else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = toml::to_string(self) {
            let _ = std::fs::write(path, text);
        }
    }

    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("markdown-editor").join("settings.toml"))
    }

    fn try_load() -> Option<Self> {
        let text = std::fs::read_to_string(Self::config_path()?).ok()?;
        toml::from_str(&text).ok()
    }

    pub fn show_window(&mut self, ctx: &eframe::egui::Context, open: &mut bool) {
        eframe::egui::Window::new("Settings")
            .open(open)
            .resizable(false)
            .default_width(320.0)
            .show(ctx, |ui| {
                eframe::egui::Grid::new("settings_grid")
                    .num_columns(2)
                    .spacing([16.0, 12.0])
                    .show(ui, |ui| {
                        ui.label("Theme");
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut self.theme, Theme::Light, "Light");
                            ui.radio_value(&mut self.theme, Theme::Dark, "Dark");
                        });
                        ui.end_row();

                        ui.label("Editor font size");
                        ui.add(
                            eframe::egui::Slider::new(&mut self.editor_font_size, 10.0..=28.0)
                                .suffix(" pt")
                                .step_by(1.0),
                        );
                        ui.end_row();

                        ui.label("DOCX page size");
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut self.docx_page, DocxPage::A4, "A4");
                            ui.radio_value(&mut self.docx_page, DocxPage::Letter, "Letter");
                        });
                        ui.end_row();

                        ui.label("OpenAI API key");
                        ui.horizontal(|ui| {
                            ui.add(
                                eframe::egui::TextEdit::singleline(&mut self.openai_api_key)
                                    .password(!self.show_api_key)
                                    .desired_width(200.0)
                                    .hint_text("sk-…"),
                            );
                            let eye = if self.show_api_key { "🙈" } else { "👁" };
                            if ui.small_button(eye).clicked() {
                                self.show_api_key = !self.show_api_key;
                            }
                        });
                        ui.end_row();
                    });
            });
    }

    pub fn apply_theme(&self, ctx: &eframe::egui::Context) {
        match self.theme {
            Theme::Light => ctx.set_visuals(eframe::egui::Visuals::light()),
            Theme::Dark => ctx.set_visuals(eframe::egui::Visuals::dark()),
        }
    }
}
