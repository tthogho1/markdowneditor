use std::sync::mpsc::{self, Receiver};

pub struct AiPanel {
    pub visible: bool,
    prompt: String,
    status: Option<String>,
    result_rx: Option<Receiver<Result<String, String>>>,
}

impl AiPanel {
    pub fn new() -> Self {
        Self {
            visible: false,
            prompt: String::new(),
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
                        self.status = None;
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
        let mut do_send = false;

        eframe::egui::Window::new("AI Assistant")
            .open(&mut self.visible)
            .resizable(true)
            .default_width(480.0)
            .show(ctx, |ui| {
                ui.label("Prompt");
                ui.add(
                    eframe::egui::TextEdit::multiline(&mut self.prompt)
                        .desired_width(f32::INFINITY)
                        .desired_rows(4)
                        .hint_text("例: 以下のMarkdownを要約してください"),
                );
                ui.add_space(8.0);

                ui.add_enabled_ui(!sending && !api_key.is_empty(), |ui| {
                    if ui.button("送信").clicked() {
                        do_send = true;
                    }
                });

                if api_key.is_empty() {
                    ui.colored_label(
                        eframe::egui::Color32::RED,
                        "Settings で OpenAI API キーを設定してください",
                    );
                }
                if sending {
                    ui.separator();
                    ui.label("送信中…");
                }
                if let Some(status) = &self.status {
                    ui.separator();
                    ui.colored_label(eframe::egui::Color32::RED, status);
                }
            });

        if do_send {
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
