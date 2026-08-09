use knit_md_docx::ConvertOptions;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ppt_rs::{create_pptx_with_content, SlideContent};
use std::path::Path;
use std::sync::mpsc;

use super::MarkdownEditorApp;

// ── PPTX conversion ───────────────────────────────────────────────────────────

fn pptx_write(markdown: &str, output: &Path, title: &str) -> Result<(), String> {
    let slides = pptx_parse(markdown);
    let bytes =
        create_pptx_with_content(title, slides).map_err(|e| e.to_string())?;
    std::fs::write(output, bytes).map_err(|e| e.to_string())
}

fn pptx_parse(markdown: &str) -> Vec<SlideContent> {
    let mut slides: Vec<SlideContent> = Vec::new();
    let mut current: Option<SlideContent> = None;
    let mut buf = String::new();
    let mut in_list_item = false;
    let mut ordered = false;

    for event in Parser::new_ext(markdown, Options::all()) {
        match event {
            Event::Start(Tag::Heading { level: HeadingLevel::H1 | HeadingLevel::H2, .. }) => {
                if let Some(slide) = current.take() {
                    slides.push(slide);
                }
                buf.clear();
            }
            Event::End(TagEnd::Heading(HeadingLevel::H1 | HeadingLevel::H2)) => {
                let title = std::mem::take(&mut buf);
                current = Some(SlideContent::new(title.trim()));
            }
            Event::Start(Tag::Heading { .. }) => buf.clear(),
            Event::End(TagEnd::Heading(_)) => {
                let text = std::mem::take(&mut buf).trim().to_string();
                if !text.is_empty() {
                    let slide = current.take().unwrap_or_else(|| SlideContent::new(""));
                    current = Some(slide.add_bullet(&format!("▶ {text}")));
                }
            }
            Event::Start(Tag::List(Some(_))) => ordered = true,
            Event::Start(Tag::List(None)) => ordered = false,
            Event::Start(Tag::Item) => {
                buf.clear();
                in_list_item = true;
            }
            Event::End(TagEnd::Item) => {
                let text = std::mem::take(&mut buf).trim().to_string();
                in_list_item = false;
                if !text.is_empty() {
                    let slide =
                        current.take().unwrap_or_else(|| SlideContent::new("Slide"));
                    current = Some(if ordered {
                        slide.add_numbered(&text)
                    } else {
                        slide.add_bullet(&text)
                    });
                }
            }
            Event::End(TagEnd::Paragraph) if !in_list_item => {
                let text = std::mem::take(&mut buf).trim().to_string();
                if !text.is_empty() {
                    if let Some(slide) = current.take() {
                        current = Some(slide.add_bullet(&text));
                    }
                }
            }
            Event::Start(Tag::CodeBlock(_)) => buf.clear(),
            Event::End(TagEnd::CodeBlock) => {
                let text = std::mem::take(&mut buf).trim().to_string();
                if !text.is_empty() {
                    if let Some(slide) = current.take() {
                        current = Some(slide.add_bullet(&format!("📋 {text}")));
                    }
                }
            }
            Event::Text(t) => buf.push_str(&t),
            Event::Code(t) => buf.push_str(&t),
            Event::SoftBreak => buf.push(' '),
            Event::HardBreak => buf.push('\n'),
            _ => {}
        }
    }

    if let Some(slide) = current {
        slides.push(slide);
    }
    if slides.is_empty() {
        slides.push(SlideContent::new("Presentation").add_bullet(markdown.trim()));
    }
    slides
}

// ── Plain-text conversion ─────────────────────────────────────────────────────

fn markdown_to_plain_text(markdown: &str) -> String {
    let parser = Parser::new_ext(markdown, Options::all());
    let mut out = String::new();
    let mut list_stack: Vec<Option<u64>> = Vec::new();
    let mut item_index: Vec<u64> = Vec::new();

    for event in parser {
        match event {
            Event::Text(t) => out.push_str(&t),
            Event::Code(t) => out.push_str(&t),
            Event::SoftBreak => out.push(' '),
            Event::HardBreak => out.push('\n'),
            Event::Rule => out.push_str("\n---\n\n"),
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => out.push_str("\n\n"),
            Event::Start(Tag::Heading { .. }) => {}
            Event::End(TagEnd::Heading(_)) => out.push_str("\n\n"),
            Event::Start(Tag::BlockQuote(_)) => out.push_str("  "),
            Event::End(TagEnd::BlockQuote(_)) => out.push('\n'),
            Event::Start(Tag::CodeBlock(_)) => out.push('\n'),
            Event::End(TagEnd::CodeBlock) => out.push('\n'),
            Event::Start(Tag::List(start)) => {
                list_stack.push(start);
                item_index.push(start.unwrap_or(1));
            }
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
                item_index.pop();
                out.push('\n');
            }
            Event::Start(Tag::Item) => {
                let depth = list_stack.len().saturating_sub(1);
                let indent = "  ".repeat(depth);
                if let Some(last_idx) = item_index.last_mut() {
                    if list_stack.last().and_then(|s| *s).is_some() {
                        out.push_str(&format!("{indent}{}. ", last_idx));
                        *last_idx += 1;
                    } else {
                        out.push_str(&format!("{indent}• "));
                    }
                }
            }
            Event::End(TagEnd::Item) => out.push('\n'),
            Event::Start(Tag::Emphasis) | Event::End(TagEnd::Emphasis) => {}
            Event::Start(Tag::Strong) | Event::End(TagEnd::Strong) => {}
            Event::Start(Tag::Strikethrough) | Event::End(TagEnd::Strikethrough) => {}
            Event::Start(Tag::Link { .. }) | Event::End(TagEnd::Link) => {}
            Event::Start(Tag::Image { .. }) | Event::End(TagEnd::Image) => {}
            _ => {}
        }
    }

    format!("{}\n", out.trim_end())
}

// ── Export methods ────────────────────────────────────────────────────────────

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

    pub fn export_html(&mut self) {
        let markdown = self.text.clone();
        let default_name = self
            .file_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| format!("{}.html", s.to_string_lossy()))
            .unwrap_or_else(|| "output.html".to_string());

        let (tx, rx) = mpsc::channel();
        self.export_rx = Some(rx);
        std::thread::spawn(move || {
            let parser =
                pulldown_cmark::Parser::new_ext(&markdown, pulldown_cmark::Options::all());
            let mut body = String::new();
            pulldown_cmark::html::push_html(&mut body, parser);
            let html = format!(
                "<!DOCTYPE html>\n\
                 <html lang=\"ja\">\n\
                 <head>\n\
                 <meta charset=\"utf-8\">\n\
                 <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
                 <style>\n\
                 body{{font-family:sans-serif;max-width:800px;margin:40px auto;padding:0 20px;line-height:1.6}}\n\
                 pre{{background:#f6f8fa;padding:12px;border-radius:6px;overflow-x:auto}}\n\
                 code{{background:#f6f8fa;padding:2px 4px;border-radius:3px}}\n\
                 blockquote{{border-left:4px solid #ddd;margin:0;padding-left:16px;color:#555}}\n\
                 </style>\n\
                 </head>\n\
                 <body>\n\
                 {body}\n\
                 </body>\n\
                 </html>"
            );
            let path = rfd::FileDialog::new()
                .add_filter("HTML File", &["html", "htm"])
                .set_file_name(&default_name)
                .save_file();
            if let Some(path) = path {
                let result = std::fs::write(&path, html)
                    .map(|_| path)
                    .map_err(|e| e.to_string());
                tx.send(result).ok();
            }
        });
    }

    pub fn export_text(&mut self) {
        let plain = markdown_to_plain_text(&self.text);
        let default_name = self
            .file_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| format!("{}.txt", s.to_string_lossy()))
            .unwrap_or_else(|| "output.txt".to_string());

        let (tx, rx) = mpsc::channel();
        self.export_rx = Some(rx);
        std::thread::spawn(move || {
            let path = rfd::FileDialog::new()
                .add_filter("Text File", &["txt"])
                .set_file_name(&default_name)
                .save_file();
            if let Some(path) = path {
                let result = std::fs::write(&path, plain)
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
                let result = pptx_write(&text, &path, &title)
                    .map(|_| path)
                    .map_err(|e| e.to_string());
                tx.send(result).ok();
            }
        });
    }
}
