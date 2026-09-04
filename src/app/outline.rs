//! Document outline: ATX headings parsed straight from the buffer.
//!
//! Parsing is line-based rather than going through `pulldown-cmark` because the panel
//! needs the *character offset* of each heading in the source text so clicking one can
//! move the editor cursor there.

use eframe::egui;

pub struct Heading {
    pub level: u8,
    pub text: String,
    /// Character index of the start of the heading's line.
    pub char_start: usize,
}

/// Extract ATX headings (`# …` through `###### …`), skipping fenced code blocks.
pub fn parse(text: &str) -> Vec<Heading> {
    let mut out = Vec::new();
    let mut char_pos = 0usize;
    let mut fence: Option<char> = None;

    for line in text.split_inclusive('\n') {
        let line_start = char_pos;
        char_pos += line.chars().count();

        let trimmed = line.trim_end_matches(['\n', '\r']).trim_start();

        // Fenced code blocks: ``` or ~~~ toggles, and only a matching marker closes.
        let fence_char = if trimmed.starts_with("```") {
            Some('`')
        } else if trimmed.starts_with("~~~") {
            Some('~')
        } else {
            None
        };
        if let Some(c) = fence_char {
            match fence {
                None => fence = Some(c),
                Some(open) if open == c => fence = None,
                Some(_) => {}
            }
            continue;
        }
        if fence.is_some() || !trimmed.starts_with('#') {
            continue;
        }

        let level = trimmed.chars().take_while(|&c| c == '#').count();
        if level > 6 {
            continue;
        }
        let rest = &trimmed[level..];
        // ATX headings need whitespace after the hashes ("#hashtag" is not a heading).
        if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
            continue;
        }
        let title = rest.trim().trim_end_matches('#').trim();
        out.push(Heading {
            level: level as u8,
            text: if title.is_empty() {
                "(untitled)".to_string()
            } else {
                title.to_string()
            },
            char_start: line_start,
        });
    }

    out
}

/// Index of the heading the cursor currently sits in, if any.
pub fn active_index(headings: &[Heading], cursor: usize) -> Option<usize> {
    headings
        .iter()
        .rposition(|h| h.char_start <= cursor)
}

/// Render the outline list. Returns the character offset to jump to, if one was clicked.
pub fn show(ui: &mut egui::Ui, headings: &[Heading], cursor: usize) -> Option<usize> {
    if headings.is_empty() {
        ui.weak("見出しがありません");
        return None;
    }

    let active = active_index(headings, cursor);
    let mut jump = None;
    for (i, h) in headings.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.add_space(f32::from(h.level - 1) * 10.0);
            let label = if h.level <= 2 {
                egui::RichText::new(&h.text).strong()
            } else {
                egui::RichText::new(&h.text)
            };
            if ui
                .selectable_label(active == Some(i), label)
                .on_hover_text(format!("H{}", h.level))
                .clicked()
            {
                jump = Some(h.char_start);
            }
        });
    }
    jump
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_levels_and_offsets() {
        let md = "# One\n\ntext\n## Two\n";
        let hs = parse(md);
        assert_eq!(hs.len(), 2);
        assert_eq!((hs[0].level, hs[0].text.as_str(), hs[0].char_start), (1, "One", 0));
        assert_eq!((hs[1].level, hs[1].text.as_str()), (2, "Two"));
        assert_eq!(&md[hs[1].char_start..hs[1].char_start + 6], "## Two");
    }

    #[test]
    fn skips_hashes_inside_fenced_code() {
        let hs = parse("# Real\n\n```\n# not a heading\n```\n\n## Also real\n");
        let titles: Vec<_> = hs.iter().map(|h| h.text.as_str()).collect();
        assert_eq!(titles, ["Real", "Also real"]);
    }

    #[test]
    fn tilde_fence_is_not_closed_by_backticks() {
        let hs = parse("~~~\n# hidden\n```\n# still hidden\n~~~\n# visible\n");
        let titles: Vec<_> = hs.iter().map(|h| h.text.as_str()).collect();
        assert_eq!(titles, ["visible"]);
    }

    #[test]
    fn requires_space_after_hashes() {
        assert!(parse("#hashtag\n").is_empty());
        assert_eq!(parse("####### seven\n").len(), 0);
    }

    #[test]
    fn char_offsets_are_correct_after_multibyte_lines() {
        let md = "日本語のテキスト\n# 見出し\n";
        let hs = parse(md);
        assert_eq!(hs[0].char_start, 9); // 8 chars + newline
        assert_eq!(hs[0].text, "見出し");
    }

    #[test]
    fn strips_closing_hashes() {
        assert_eq!(parse("## Title ##\n")[0].text, "Title");
    }

    #[test]
    fn active_index_tracks_the_cursor() {
        let hs = parse("# A\n\n# B\n");
        assert_eq!(active_index(&hs, 0), Some(0));
        assert_eq!(active_index(&hs, 4), Some(0));
        assert_eq!(active_index(&hs, 6), Some(1));
    }
}
