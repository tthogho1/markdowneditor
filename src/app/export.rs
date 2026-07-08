use knit_md_docx::ConvertOptions;
use std::sync::mpsc;

use super::MarkdownEditorApp;

impl MarkdownEditorApp {
    pub fn export_docx(&mut self) {
        let text = self.text.clone();
        let default_name = self
            .file_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| format!("{}.docx", s.to_string_lossy()))
            .unwrap_or_else(|| "output.docx".to_string());
        let page = self.settings.docx_page.to_page_setup();

        let (tx, rx) = mpsc::channel();
        self.export_rx = Some(rx);

        std::thread::spawn(move || {
            let path = rfd::FileDialog::new()
                .add_filter("Word Document", &["docx"])
                .set_file_name(&default_name)
                .save_file();

            if let Some(path) = path {
                let mut opts = ConvertOptions::default();
                opts.page = page;
                let result = knit_md_docx::write_file_with(&text, &opts, &path)
                    .map(|_| path)
                    .map_err(|e| e.to_string());
                tx.send(result).ok();
            }
        });
    }

    pub fn export_pptx(&mut self) {
        let text = self.text.clone();
        let default_name = self
            .file_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| format!("{}.pptx", s.to_string_lossy()))
            .unwrap_or_else(|| "output.pptx".to_string());
        let title = self
            .file_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Presentation".to_string());

        let (tx, rx) = mpsc::channel();
        self.export_rx = Some(rx);

        std::thread::spawn(move || {
            let path = rfd::FileDialog::new()
                .add_filter("PowerPoint", &["pptx"])
                .set_file_name(&default_name)
                .save_file();

            if let Some(path) = path {
                let result = crate::pptx_export::convert(&text, &path, &title)
                    .map(|_| path)
                    .map_err(|e| e.to_string());
                tx.send(result).ok();
            }
        });
    }
}
