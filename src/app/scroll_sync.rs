//! Proportional scroll synchronisation between the editor and preview panes.
//!
//! `egui_commonmark` renders the preview without any anchors back to source offsets, so
//! exact section-to-section sync isn't available. Instead both panes are kept at the same
//! *fraction* of their scrollable height, driven by whichever pane the pointer is over.
//! Offsets are applied on the following frame (a pane's content height is only known
//! after it has been laid out), which is imperceptible at frame rate.

#[derive(Default, Clone, Copy)]
struct Pane {
    offset: f32,
    /// Scrollable range: content height minus viewport height.
    max: f32,
    last_offset: f32,
}

impl Pane {
    fn record(&mut self, offset: f32, content_h: f32, view_h: f32) {
        self.offset = offset;
        self.max = (content_h - view_h).max(0.0);
    }

    fn moved(&self) -> bool {
        (self.offset - self.last_offset).abs() > 0.5
    }

    fn fraction(&self) -> f32 {
        if self.max <= 0.0 {
            0.0
        } else {
            (self.offset / self.max).clamp(0.0, 1.0)
        }
    }
}

pub struct ScrollSync {
    pub enabled: bool,
    editor: Pane,
    preview: Pane,
    /// Offsets to force on the next frame.
    pub apply_editor: Option<f32>,
    pub apply_preview: Option<f32>,
}

impl Default for ScrollSync {
    fn default() -> Self {
        Self {
            enabled: true,
            editor: Pane::default(),
            preview: Pane::default(),
            apply_editor: None,
            apply_preview: None,
        }
    }
}

impl ScrollSync {
    pub fn record_editor(&mut self, offset: f32, content_h: f32, view_h: f32) {
        self.editor.record(offset, content_h, view_h);
    }

    pub fn record_preview(&mut self, offset: f32, content_h: f32, view_h: f32) {
        self.preview.record(offset, content_h, view_h);
    }

    /// Decide which pane leads this frame and queue the follower's offset.
    ///
    /// Only the pane under the pointer can lead, which keeps the pane we just scrolled
    /// programmatically from scrolling the other one back.
    pub fn sync(&mut self, pointer_in_editor: bool, pointer_in_preview: bool) {
        self.apply_editor = None;
        self.apply_preview = None;

        if self.enabled {
            if pointer_in_editor && self.editor.moved() {
                self.apply_preview = Some(self.editor.fraction() * self.preview.max);
            } else if pointer_in_preview && self.preview.moved() {
                self.apply_editor = Some(self.preview.fraction() * self.editor.max);
            }
        }

        self.editor.last_offset = self.editor.offset;
        self.preview.last_offset = self.preview.offset;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sync_with(pointer_in_editor: bool) -> ScrollSync {
        let mut s = ScrollSync::default();
        // Frame 1: both panes at rest.
        s.record_editor(0.0, 2000.0, 500.0); // max 1500
        s.record_preview(0.0, 1000.0, 500.0); // max 500
        s.sync(pointer_in_editor, !pointer_in_editor);
        // Frame 2: the editor jumps to 50%.
        s.record_editor(750.0, 2000.0, 500.0);
        s.record_preview(0.0, 1000.0, 500.0);
        s.sync(pointer_in_editor, !pointer_in_editor);
        s
    }

    #[test]
    fn leading_pane_scrolls_the_follower_to_the_same_fraction() {
        assert_eq!(sync_with(true).apply_preview, Some(250.0)); // 50% of 500
    }

    #[test]
    fn a_pane_without_the_pointer_does_not_lead() {
        assert_eq!(sync_with(false).apply_preview, None);
    }

    #[test]
    fn disabled_sync_queues_nothing() {
        let mut s = ScrollSync {
            enabled: false,
            ..Default::default()
        };
        s.record_editor(0.0, 2000.0, 500.0);
        s.sync(true, false);
        s.record_editor(750.0, 2000.0, 500.0);
        s.sync(true, false);
        assert_eq!(s.apply_preview, None);
    }

    #[test]
    fn unscrollable_pane_reports_zero_fraction() {
        let mut p = Pane::default();
        p.record(0.0, 100.0, 500.0);
        assert_eq!(p.max, 0.0);
        assert_eq!(p.fraction(), 0.0);
    }
}
