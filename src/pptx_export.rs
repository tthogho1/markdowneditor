use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ppt_rs::{create_pptx_with_content, SlideContent};
use std::path::Path;

/// Convert Markdown text to a .pptx file at `output`.
/// H1/H2 headings become slide titles; list items and paragraphs become bullets.
pub fn convert(markdown: &str, output: &Path, presentation_title: &str) -> Result<(), String> {
    let slides = parse(markdown);
    let bytes = create_pptx_with_content(presentation_title, slides)
        .map_err(|e| e.to_string())?;
    std::fs::write(output, bytes).map_err(|e| e.to_string())
}

fn parse(markdown: &str) -> Vec<SlideContent> {
    let mut slides: Vec<SlideContent> = Vec::new();
    let mut current: Option<SlideContent> = None;
    let mut buf = String::new();
    let mut in_list_item = false;
    let mut ordered = false;

    for event in Parser::new_ext(markdown, Options::all()) {
        match event {
            // H1 / H2 start → flush current slide, clear buffer
            Event::Start(Tag::Heading { level: HeadingLevel::H1 | HeadingLevel::H2, .. }) => {
                if let Some(slide) = current.take() {
                    slides.push(slide);
                }
                buf.clear();
            }
            // H1 / H2 end → start a new slide with collected title
            Event::End(TagEnd::Heading(HeadingLevel::H1 | HeadingLevel::H2)) => {
                let title = std::mem::take(&mut buf);
                current = Some(SlideContent::new(title.trim()));
            }

            // H3+ start → clear buffer
            Event::Start(Tag::Heading { .. }) => buf.clear(),
            // H3+ end → add as bold-prefixed bullet in current slide
            Event::End(TagEnd::Heading(_)) => {
                let text = std::mem::take(&mut buf);
                let text = text.trim().to_string();
                if !text.is_empty() {
                    let slide = current.take().unwrap_or_else(|| SlideContent::new(""));
                    current = Some(slide.add_bullet(&format!("▶ {text}")));
                }
            }

            // Lists
            Event::Start(Tag::List(Some(_))) => ordered = true,
            Event::Start(Tag::List(None)) => ordered = false,
            Event::Start(Tag::Item) => {
                buf.clear();
                in_list_item = true;
            }
            Event::End(TagEnd::Item) => {
                let text = std::mem::take(&mut buf);
                let text = text.trim().to_string();
                in_list_item = false;
                if !text.is_empty() {
                    let slide = current.take().unwrap_or_else(|| SlideContent::new("Slide"));
                    current = Some(if ordered {
                        slide.add_numbered(&text)
                    } else {
                        slide.add_bullet(&text)
                    });
                }
            }

            // Paragraphs outside list items → bullet
            Event::End(TagEnd::Paragraph) if !in_list_item => {
                let text = std::mem::take(&mut buf);
                let text = text.trim().to_string();
                if !text.is_empty() {
                    if let Some(slide) = current.take() {
                        current = Some(slide.add_bullet(&text));
                    }
                }
            }

            // Code blocks
            Event::Start(Tag::CodeBlock(_)) => buf.clear(),
            Event::End(TagEnd::CodeBlock) => {
                let text = std::mem::take(&mut buf);
                let text = text.trim().to_string();
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
