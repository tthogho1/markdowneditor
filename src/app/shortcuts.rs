//! Keyboard shortcuts.
//!
//! All shortcuts are consumed at the top of `update`, before any widget is built, so the
//! editor `TextEdit` never sees them. `Modifiers::COMMAND` is ⌘ on macOS and Ctrl elsewhere.

use eframe::egui::{Key, KeyboardShortcut, Modifiers};

/// Which shortcuts fired this frame.
#[derive(Default)]
pub struct Hits {
    pub new_file: bool,
    pub open: bool,
    pub save: bool,
    pub save_as: bool,
    pub find: bool,
    pub find_next: bool,
    pub find_prev: bool,
    pub escape: bool,
    pub bold: bool,
    pub italic: bool,
    pub link: bool,
    pub toggle_outline: bool,
}

const COMMAND: Modifiers = Modifiers::COMMAND;
const COMMAND_SHIFT: Modifiers = Modifiers::COMMAND.plus(Modifiers::SHIFT);

pub fn consume(ctx: &eframe::egui::Context) -> Hits {
    let mut hits = Hits::default();
    ctx.input_mut(|i| {
        let mut hit =
            |mods: Modifiers, key: Key| i.consume_shortcut(&KeyboardShortcut::new(mods, key));
        // Shift variants must be checked before their plain counterparts: `consume_shortcut`
        // ignores *extra* Shift, so Cmd+S would otherwise swallow Cmd+Shift+S.
        hits.save_as = hit(COMMAND_SHIFT, Key::S);
        hits.find_prev = hit(COMMAND_SHIFT, Key::G);
        hits.toggle_outline = hit(COMMAND_SHIFT, Key::O);

        hits.new_file = hit(COMMAND, Key::N);
        hits.open = hit(COMMAND, Key::O);
        hits.save = hit(COMMAND, Key::S);
        hits.find = hit(COMMAND, Key::F);
        hits.find_next = hit(COMMAND, Key::G);
        hits.bold = hit(COMMAND, Key::B);
        hits.italic = hit(COMMAND, Key::I);
        hits.link = hit(COMMAND, Key::K);

        hits.escape = i.key_pressed(Key::Escape);
    });
    hits
}

/// Shortcut hint text for a menu item, formatted for the current platform.
pub fn hint(shift: bool, key: &str) -> String {
    if cfg!(target_os = "macos") {
        format!("{}\u{2318}{key}", if shift { "\u{21e7}" } else { "" })
    } else {
        format!("Ctrl+{}{key}", if shift { "Shift+" } else { "" })
    }
}
