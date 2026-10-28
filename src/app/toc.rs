//! Table of contents: a nested Markdown link list built from the document's headings,
//! plus the matching heading anchors for HTML export and preview link hooks.
//!
//! Unlike `outline.rs` this goes through `pulldown-cmark`, so setext headings and inline
//! markup (`## **Bold** title` → "Bold title") are handled the same way the renderers see
//! them, and slugs line up with the `id`s written to exported HTML.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::collections::HashMap;

use super::text_util::byte_to_char;

pub struct TocEntry {
    pub level: u8,
    pub text: String,
    /// Anchor without the leading `#`.
    pub slug: String,
    /// Character index of the start of the heading in the source.
    pub char_start: usize,
}

/// GitHub-style slug: lowercase, spaces → `-`, punctuation dropped, non-ASCII letters kept.
pub fn slugify(text: &str) -> String {
    text.trim()
        .to_lowercase()
        .chars()
        .filter_map(|c| match c {
            ' ' => Some('-'),
            '-' | '_' => Some(c),
            c if c.is_alphanumeric() => Some(c),
            _ => None,
        })
        .collect()
}

/// Hands out unique slugs, suffixing repeats with `-1`, `-2`, … like GitHub.
#[derive(Default)]
struct Slugger(HashMap<String, usize>);

impl Slugger {
    fn unique(&mut self, base: String) -> String {
        let n = self.0.entry(base.clone()).or_insert(0);
        let slug = if *n == 0 { base } else { format!("{base}-{n}") };
        *n += 1;
        slug
    }
}

/// All headings in document order, with unique slugs. An explicit `{#id}` wins over the
/// generated slug.
pub fn entries(markdown: &str) -> Vec<TocEntry> {
    let mut out = Vec::new();
    let mut slugger = Slugger::default();
    // (level, explicit id, byte start, collected text)
    let mut current: Option<(u8, Option<String>, usize, String)> = None;

    for (event, range) in Parser::new_ext(markdown, Options::all()).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, id, .. }) => {
                current = Some((level as u8, id.map(|s| s.to_string()), range.start, String::new()));
            }
            Event::Text(t) | Event::Code(t) => {
                if let Some((_, _, _, text)) = &mut current {
                    text.push_str(&t);
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, id, start, text)) = current.take() {
                    let text = text.trim().to_string();
                    let slug = match id {
                        Some(id) => id,
                        None => slugger.unique(slugify(&text)),
                    };
                    out.push(TocEntry {
                        level,
                        text,
                        slug,
                        char_start: byte_to_char(markdown, start),
                    });
                }
            }
            _ => {}
        }
    }
    out
}

/// Render the entries as a nested bullet list of `[text](#slug)` links.
pub fn render(entries: &[TocEntry]) -> String {
    let Some(min) = entries.iter().map(|e| e.level).min() else {
        return String::new();
    };
    let mut out = String::new();
    let mut prev_depth: Option<usize> = None;
    for e in entries {
        // Never nest more than one level deeper than the previous item, or the list
        // would turn into an indented code block (e.g. a document starting at H3).
        let want = usize::from(e.level - min);
        let depth = prev_depth.map_or(0, |p| want.min(p + 1));
        prev_depth = Some(depth);

        let label = e.text.replace('\\', "\\\\").replace('[', "\\[").replace(']', "\\]");
        out.push_str(&"  ".repeat(depth));
        out.push_str(&format!("- [{label}](#{})\n", e.slug));
    }
    out
}

/// Fill in `id` on every heading that lacks one, so the TOC links resolve in exported HTML.
/// Ids come from `entries`, so they always match the generated TOC.
pub fn with_heading_ids<'a>(markdown: &'a str, events: Vec<Event<'a>>) -> Vec<Event<'a>> {
    let mut slugs = entries(markdown).into_iter().map(|e| e.slug);
    events
        .into_iter()
        .map(|event| match event {
            Event::Start(Tag::Heading { level, id, classes, attrs }) => {
                let slug = slugs.next();
                let id = id.or_else(|| slug.map(Into::into));
                Event::Start(Tag::Heading { level, id, classes, attrs })
            }
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_matches_github_style() {
        assert_eq!(slugify("Hello, World!"), "hello-world");
        assert_eq!(slugify("snake_case and-dash"), "snake_case-and-dash");
        assert_eq!(slugify("日本語の見出し"), "日本語の見出し");
    }

    #[test]
    fn duplicate_headings_get_suffixes() {
        let es = entries("# Intro\n## Intro\n## Intro\n");
        let slugs: Vec<_> = es.iter().map(|e| e.slug.as_str()).collect();
        assert_eq!(slugs, ["intro", "intro-1", "intro-2"]);
    }

    #[test]
    fn strips_inline_markup_and_finds_setext() {
        let es = entries("Title\n=====\n\n## **Bold** `code`\n");
        assert_eq!((es[0].level, es[0].text.as_str()), (1, "Title"));
        assert_eq!(es[1].text, "Bold code");
        assert_eq!(es[1].slug, "bold-code");
    }

    #[test]
    fn ignores_hashes_in_code_and_honours_explicit_ids() {
        let es = entries("```\n# no\n```\n# Yes {#custom}\n");
        assert_eq!(es.len(), 1);
        assert_eq!(es[0].slug, "custom");
    }

    #[test]
    fn char_start_counts_characters_not_bytes() {
        let md = "日本語\n\n# 見出し\n";
        assert_eq!(entries(md)[0].char_start, 5);
    }

    #[test]
    fn render_nests_and_clamps_depth() {
        let toc = render(&entries("### Deep first\n# Top\n### Skips a level\n## Mid\n"));
        assert_eq!(
            toc,
            "- [Deep first](#deep-first)\n\
             - [Top](#top)\n  \
               - [Skips a level](#skips-a-level)\n  \
               - [Mid](#mid)\n"
        );
    }

    #[test]
    fn render_escapes_brackets() {
        assert_eq!(render(&entries("# a [b]\n")), "- [a \\[b\\]](#a-b)\n");
    }

    #[test]
    fn html_export_gets_matching_ids() {
        let md = "# One\n## One\n";
        let events = Parser::new_ext(md, Options::all()).collect();
        let mut html = String::new();
        pulldown_cmark::html::push_html(&mut html, with_heading_ids(md, events).into_iter());
        assert!(html.contains("<h1 id=\"one\">"));
        assert!(html.contains("<h2 id=\"one-1\">"));
    }
}
