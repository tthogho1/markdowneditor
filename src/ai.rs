use std::sync::mpsc::{self, Receiver};

const LANGS: &[&str] = &["English", "日本語", "中文", "Français", "Español", "Deutsch"];

pub struct AiPanel {
    pub visible: bool,
    prompt: String,
    translate_lang: usize, // index into LANGS
    status: Option<String>,
    result_rx: Option<Receiver<Result<String, String>>>,
}

impl AiPanel {
    pub fn new() -> Self {
        Self {
            visible: false,
            prompt: String::new(),
            translate_lang: 0, // English
            status: None,
            result_rx: None,
        }
    }

    /// Show the AI panel window. Overwrites `markdown` with the API response on success.
    pub fn show(
        &mut self,
        ctx: &eframe::egui::Context,
        markdown: &mut String,
        api_key: &str,
    ) {
        // Poll result from background thread
        if let Some(rx) = &self.result_rx {
            if let Ok(result) = rx.try_recv() {
                match result {
                    Ok(text) => {
                        *markdown = text;
                        self.status = Some("完了".to_string());
                    }
                    Err(e) => {
                        self.status = Some(format!("Error: {}", e));
                    }
                }
                self.result_rx = None;
            }
        }

        if !self.visible {
            return;
        }

        let sending = self.result_rx.is_some();
        let mut quick_prompt: Option<String> = None;
        let mut do_send = false;

        eframe::egui::Window::new("AI Assistant")
            .open(&mut self.visible)
            .resizable(true)
            .default_width(480.0)
            .show(ctx, |ui| {
                // ── Quick actions ────────────────────────────────────────
                ui.label("Quick Actions");
                ui.add_space(4.0);

                ui.add_enabled_ui(!sending && !api_key.is_empty(), |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("📝 要約").clicked() {
                            quick_prompt = Some(
                                "以下のMarkdownを日本語で要約してください。\
                                重要なポイントを箇条書きでまとめ、Markdown形式で回答してください。"
                                    .to_string(),
                            );
                        }

                        // Translation button + language selector
                        if ui.button("🌐 翻訳").clicked() {
                            let lang = LANGS[self.translate_lang];
                            quick_prompt = Some(format!(
                                "以下のMarkdownを{lang}に翻訳してください。\
                                Markdownの書式（見出し・リスト・コードブロック等）は維持してください。"
                            ));
                        }
                        eframe::egui::ComboBox::from_id_salt("lang_select")
                            .selected_text(LANGS[self.translate_lang])
                            .width(90.0)
                            .show_ui(ui, |ui| {
                                for (i, &lang) in LANGS.iter().enumerate() {
                                    ui.selectable_value(&mut self.translate_lang, i, lang);
                                }
                            });

                        if ui.button("✏️ 校正").clicked() {
                            quick_prompt = Some(
                                "以下のMarkdownを校正してください。\
                                誤字脱字の修正・文体の統一・読みやすさの改善を行い、\
                                Markdownの書式は維持したまま修正済みの全文を返してください。"
                                    .to_string(),
                            );
                        }
                    });
                });

                ui.separator();

                // ── Free-form prompt ─────────────────────────────────────
                ui.label("Prompt");
                ui.add(
                    eframe::egui::TextEdit::multiline(&mut self.prompt)
                        .desired_width(f32::INFINITY)
                        .desired_rows(4)
                        .hint_text("自由にプロンプトを入力…"),
                );
                ui.add_space(4.0);

                ui.add_enabled_ui(!sending && !api_key.is_empty(), |ui| {
                    if ui.button("送信").clicked() {
                        do_send = true;
                    }
                });

                // ── Status area ──────────────────────────────────────────
                if api_key.is_empty() {
                    ui.separator();
                    ui.colored_label(
                        eframe::egui::Color32::RED,
                        "Settings で OpenAI API キーを設定してください",
                    );
                }
                if sending {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("送信中…");
                    });
                }
                if let Some(status) = &self.status {
                    ui.separator();
                    let color = if status.starts_with("Error") {
                        eframe::egui::Color32::RED
                    } else {
                        eframe::egui::Color32::GREEN
                    };
                    ui.colored_label(color, status);
                }
            });

        if let Some(prompt) = quick_prompt {
            self.send(markdown.clone(), prompt, api_key.to_string());
        } else if do_send {
            self.send(markdown.clone(), self.prompt.clone(), api_key.to_string());
        }
    }

    fn send(&mut self, markdown: String, prompt: String, api_key: String) {
        let (tx, rx) = mpsc::channel();
        self.result_rx = Some(rx);
        self.status = None;

        std::thread::spawn(move || {
            let result = call_openai(&api_key, &prompt, &markdown);
            tx.send(result).ok();
        });
    }
}

fn call_openai(api_key: &str, prompt: &str, markdown: &str) -> Result<String, String> {
    let body = serde_json::json!({
        "model": "gpt-4o",
        "messages": [{
            "role": "user",
            "content": format!("{}\n\n---\n\n{}", prompt, markdown)
        }]
    });

    let response = ureq::post("https://api.openai.com/v1/chat/completions")
        .set("Authorization", &format!("Bearer {}", api_key))
        .set("Content-Type", "application/json")
        .send_json(body)
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = response.into_json().map_err(|e| e.to_string())?;

    json["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Unexpected response format".to_string())
}
