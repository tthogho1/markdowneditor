//! Find & replace bar for the editor buffer.
//!
//! Matches are stored as *character* ranges so they can be handed straight to egui's
//! `CCursor`; conversion to byte offsets happens only when the text is spliced.

use eframe::egui;

use super::text_util::char_to_byte;

/// What the caller must do after showing the bar.
#[derive(Default)]
pub struct FindOutcome {
    /// Character range to select and scroll into view in the editor.
    pub select: Option<(usize, usize)>,
    /// The buffer was modified (caller must bump its text revision / dirty flag).
    pub text_changed: bool,
}

#[derive(Default)]
pub struct FindReplace {
    pub visible: bool,
    query: String,
    replace_with: String,
    case_sensitive: bool,
    focus_query: bool,
    matches: Vec<(usize, usize)>,
    current: Option<usize>,
    /// (query, case_sensitive, text revision) the cached `matches` were computed from.
    cache_key: Option<(String, bool, u64)>,
}

impl FindReplace {
    /// Open the bar and focus the query field.
    pub fn open(&mut self) {
        self.visible = true;
        self.focus_query = true;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.current = None;
    }

    /// Recompute matches when the query, the case flag or the buffer changed.
    fn refresh(&mut self, text: &str, text_rev: u64) {
        let key = (self.query.clone(), self.case_sensitive, text_rev);
        if self.cache_key.as_ref() == Some(&key) {
            return;
        }
        self.matches = find_matches(text, &self.query, self.case_sensitive);
        if self.current.is_some_and(|i| i >= self.matches.len()) {
            self.current = if self.matches.is_empty() {
                None
            } else {
                Some(self.matches.len() - 1)
            };
        }
        self.cache_key = Some(key);
    }

    /// Advance to the next/previous match, wrapping around.
    fn step(&mut self, forward: bool) -> Option<(usize, usize)> {
        let len = self.matches.len();
        if len == 0 {
            return None;
        }
        let next = match self.current {
            None => {
                if forward {
                    0
                } else {
                    len - 1
                }
            }
            Some(i) if forward => (i + 1) % len,
            Some(i) => (i + len - 1) % len,
        };
        self.current = Some(next);
        Some(self.matches[next])
    }

    /// Jump to the next/previous match from outside the bar (keyboard shortcut).
    pub fn step_external(&mut self, text: &str, text_rev: u64, forward: bool) -> Option<(usize, usize)> {
        self.refresh(text, text_rev);
        self.step(forward)
    }

    fn replace_current(&mut self, text: &mut String) -> bool {
        let Some(i) = self.current else { return false };
        let Some(&(lo, hi)) = self.matches.get(i) else {
            return false;
        };
        let lo_b = char_to_byte(text, lo);
        let hi_b = char_to_byte(text, hi);
        text.replace_range(lo_b..hi_b, &self.replace_with);
        // Keep `current` where it is: after the splice that index refers to the
        // following match, so Replace → Replace walks forward naturally.
        self.cache_key = None;
        true
    }

    fn replace_all(&mut self, text: &mut String) -> usize {
        if self.matches.is_empty() {
            return 0;
        }
        // Splice back-to-front so earlier ranges stay valid.
        for &(lo, hi) in self.matches.iter().rev() {
            let lo_b = char_to_byte(text, lo);
            let hi_b = char_to_byte(text, hi);
            text.replace_range(lo_b..hi_b, &self.replace_with);
        }
        let n = self.matches.len();
        self.current = None;
        self.cache_key = None;
        n
    }

    pub fn show(&mut self, ui: &mut egui::Ui, text: &mut String, text_rev: u64) -> FindOutcome {
        let mut out = FindOutcome::default();
        self.refresh(text, text_rev);

        ui.horizontal_wrapped(|ui| {
            ui.label("検索");
            let field = ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .desired_width(180.0)
                    .hint_text("Find…"),
            );
            if std::mem::take(&mut self.focus_query) {
                field.request_focus();
            }
            // Enter in the query field goes to the next match.
            if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.refresh(text, text_rev);
                out.select = self.step(true);
                field.request_focus();
            }

            let count = self.matches.len();
            let label = match self.current {
                Some(i) if count > 0 => format!("{} / {}", i + 1, count),
                _ if count > 0 => format!("- / {count}"),
                _ if self.query.is_empty() => String::new(),
                _ => "0 件".to_string(),
            };
            ui.add_enabled_ui(count > 0, |ui| {
                if ui.small_button("◀").on_hover_text("前へ (Shift+Cmd/Ctrl+G)").clicked() {
                    out.select = self.step(false);
                }
                if ui.small_button("▶").on_hover_text("次へ (Cmd/Ctrl+G)").clicked() {
                    out.select = self.step(true);
                }
            });
            ui.label(label);

            ui.separator();

            ui.label("置換");
            ui.add(
                egui::TextEdit::singleline(&mut self.replace_with)
                    .desired_width(180.0)
                    .hint_text("Replace with…"),
            );
            ui.add_enabled_ui(count > 0, |ui| {
                if ui.small_button("置換").on_hover_text("Replace the current match").clicked()
                    && self.replace_current(text)
                {
                    out.text_changed = true;
                }
                if ui
                    .small_button("すべて置換")
                    .on_hover_text("Replace every match")
                    .clicked()
                    && self.replace_all(text) > 0
                {
                    out.text_changed = true;
                }
            });

            ui.separator();
            ui.checkbox(&mut self.case_sensitive, "Aa")
                .on_hover_text("Match case");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("✕").on_hover_text("閉じる (Esc)").clicked() {
                    self.close();
                }
            });
        });

        out
    }
}

/// Non-overlapping character ranges of `query` within `text`.
///
/// Case-insensitive comparison folds each `char` individually rather than lowercasing
/// the whole string, so the 1:1 char mapping the returned indices rely on is preserved.
pub fn find_matches(text: &str, query: &str, case_sensitive: bool) -> Vec<(usize, usize)> {
    if query.is_empty() {
        return Vec::new();
    }
    let fold = |c: char| {
        if case_sensitive {
            c
        } else {
            c.to_lowercase().next().unwrap_or(c)
        }
    };
    let hay: Vec<char> = text.chars().map(fold).collect();
    let needle: Vec<char> = query.chars().map(fold).collect();
    if needle.len() > hay.len() {
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut i = 0;
    while i + needle.len() <= hay.len() {
        if hay[i..i + needle.len()] == needle[..] {
            out.push((i, i + needle.len()));
            i += needle.len();
        } else {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_non_overlapping_matches() {
        assert_eq!(find_matches("aaaa", "aa", true), vec![(0, 2), (2, 4)]);
    }

    #[test]
    fn respects_case_sensitivity() {
        assert!(find_matches("Hello", "hello", true).is_empty());
        assert_eq!(find_matches("Hello", "hello", false), vec![(0, 5)]);
    }

    #[test]
    fn returns_char_indices_not_byte_indices() {
        // "日本語" is 9 bytes but 3 chars; the match must start at char 3.
        assert_eq!(find_matches("日本語text", "text", true), vec![(3, 7)]);
    }

    #[test]
    fn empty_query_matches_nothing() {
        assert!(find_matches("anything", "", false).is_empty());
    }

    #[test]
    fn replaces_every_match_back_to_front() {
        let mut fr = FindReplace {
            query: "本".into(),
            replace_with: "BOOK".into(),
            ..Default::default()
        };
        let mut text = String::from("日本語と本");
        fr.refresh(&text, 0);
        assert_eq!(fr.replace_all(&mut text), 2);
        assert_eq!(text, "日BOOK語とBOOK");
    }

    #[test]
    fn replace_current_splices_only_the_selected_match() {
        let mut fr = FindReplace {
            query: "x".into(),
            replace_with: "y".into(),
            ..Default::default()
        };
        let mut text = String::from("axbxc");
        fr.refresh(&text, 0);
        fr.step(true);
        assert!(fr.replace_current(&mut text));
        assert_eq!(text, "aybxc");
    }
}
