//! Char-index ↔ byte-index conversions shared by the toolbar, find/replace and outline.
//!
//! egui addresses text by *character* index (`CCursor`), while `String` slicing needs
//! *byte* offsets. Mixing the two silently corrupts non-ASCII text, which matters here
//! because the editor is routinely used with Japanese.

/// Byte offset of the `char_idx`-th character (clamped to `text.len()`).
pub fn char_to_byte(text: &str, char_idx: usize) -> usize {
    text.char_indices()
        .nth(char_idx)
        .map_or(text.len(), |(b, _)| b)
}

/// Number of characters before `byte_idx` (clamped to the end of `text`).
pub fn byte_to_char(text: &str, byte_idx: usize) -> usize {
    text[..byte_idx.min(text.len())].chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_multibyte_text() {
        let s = "あiうeお";
        for (i, _) in s.char_indices() {
            assert_eq!(char_to_byte(s, byte_to_char(s, i)), i);
        }
    }

    #[test]
    fn clamps_past_the_end() {
        let s = "日本語";
        assert_eq!(char_to_byte(s, 99), s.len());
        assert_eq!(byte_to_char(s, 99), 3);
    }
}
