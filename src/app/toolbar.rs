use eframe::egui::{self, RichText};

#[derive(Clone)]
pub enum FormatAction {
    Bold,
    Italic,
    Strikethrough,
    InlineCode,
    CodeBlock,
    Heading(u8),
    Link,
    BulletList,
    NumberedList,
    Quote,
    HorizontalRule,
}

/// Render the toolbar and return the triggered action, if any.
pub fn show(ui: &mut egui::Ui) -> Option<FormatAction> {
    let mut action = None;
    ui.spacing_mut().item_spacing.x = 2.0;

    // Inline formatting
    if btn(ui, RichText::new("B").strong(), "Bold (**text**)") {
        action = Some(FormatAction::Bold);
    }
    if btn(ui, RichText::new("I").italics(), "Italic (*text*)") {
        action = Some(FormatAction::Italic);
    }
    if btn(ui, RichText::new("S").strikethrough(), "Strikethrough (~~text~~)") {
        action = Some(FormatAction::Strikethrough);
    }

    ui.separator();

    // Headings
    for level in 1u8..=3 {
        if btn(ui, format!("H{level}"), format!("Heading {level}")) {
            action = Some(FormatAction::Heading(level));
        }
    }

    ui.separator();

    // Code
    if btn(ui, RichText::new("`c`").monospace(), "Inline code") {
        action = Some(FormatAction::InlineCode);
    }
    if btn(ui, RichText::new("```").monospace(), "Code block") {
        action = Some(FormatAction::CodeBlock);
    }

    ui.separator();

    // Link
    if btn(ui, "🔗", "Link ([text](url))") {
        action = Some(FormatAction::Link);
    }

    ui.separator();

    // Lists / block
    if btn(ui, "• List", "Bullet list") {
        action = Some(FormatAction::BulletList);
    }
    if btn(ui, "1. List", "Numbered list") {
        action = Some(FormatAction::NumberedList);
    }
    if btn(ui, "❝ Quote", "Blockquote (> text)") {
        action = Some(FormatAction::Quote);
    }

    ui.separator();

    if btn(ui, "─── HR", "Horizontal rule (---)") {
        action = Some(FormatAction::HorizontalRule);
    }

    action
}

fn btn(ui: &mut egui::Ui, label: impl Into<egui::WidgetText>, tip: impl Into<String>) -> bool {
    ui.small_button(label).on_hover_text(tip.into()).clicked()
}

/// Apply a format action to `text` at the given cursor/selection range (char indices).
/// Returns the new cursor position as a char index.
pub fn apply_format(
    text: &mut String,
    action: &FormatAction,
    sel_start: usize,
    sel_end: usize,
) -> usize {
    let lo_char = sel_start.min(sel_end);
    let hi_char = sel_start.max(sel_end);
    let lo = char_to_byte(text, lo_char);
    let hi = char_to_byte(text, hi_char);

    let new_byte = match action {
        FormatAction::Bold => wrap(text, lo, hi, "**", "**"),
        FormatAction::Italic => wrap(text, lo, hi, "*", "*"),
        FormatAction::Strikethrough => wrap(text, lo, hi, "~~", "~~"),
        FormatAction::InlineCode => wrap(text, lo, hi, "`", "`"),
        FormatAction::CodeBlock => {
            let inner = if lo < hi {
                text[lo..hi].to_string()
            } else {
                "code here".to_string()
            };
            let repl = format!("```\n{inner}\n```");
            let end = lo + repl.len();
            text.replace_range(lo..hi, &repl);
            end
        }
        FormatAction::Heading(level) => {
            let prefix = format!("{} ", "#".repeat(*level as usize));
            let line_byte = line_start(text, lo);
            text.insert_str(line_byte, &prefix);
            line_byte + prefix.len()
        }
        FormatAction::Link => {
            let label = if lo < hi {
                text[lo..hi].to_string()
            } else {
                "link text".to_string()
            };
            let repl = format!("[{label}](url)");
            let end = lo + repl.len();
            text.replace_range(lo..hi, &repl);
            end
        }
        FormatAction::BulletList => {
            let lb = line_start(text, lo);
            text.insert_str(lb, "- ");
            lb + 2
        }
        FormatAction::NumberedList => {
            let lb = line_start(text, lo);
            text.insert_str(lb, "1. ");
            lb + 3
        }
        FormatAction::Quote => {
            let lb = line_start(text, lo);
            text.insert_str(lb, "> ");
            lb + 2
        }
        FormatAction::HorizontalRule => {
            let insert = "\n---\n";
            text.insert_str(hi, insert);
            hi + insert.len()
        }
    };

    byte_to_char(text, new_byte)
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn wrap(text: &mut String, lo: usize, hi: usize, pre: &str, suf: &str) -> usize {
    let inner = if lo < hi {
        text[lo..hi].to_string()
    } else {
        "text".to_string()
    };
    let repl = format!("{pre}{inner}{suf}");
    let end = lo + repl.len();
    text.replace_range(lo..hi, &repl);
    end
}

fn line_start(text: &str, byte_pos: usize) -> usize {
    text[..byte_pos].rfind('\n').map_or(0, |i| i + 1)
}

fn char_to_byte(text: &str, char_idx: usize) -> usize {
    text.char_indices()
        .nth(char_idx)
        .map_or(text.len(), |(b, _)| b)
}

fn byte_to_char(text: &str, byte_idx: usize) -> usize {
    text[..byte_idx.min(text.len())].chars().count()
}
