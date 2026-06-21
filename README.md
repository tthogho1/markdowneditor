# Markdown Editor

A fast, native Markdown editor built with Rust, [egui](https://github.com/emilk/egui), and [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark). Write Markdown on the left and see a live rendered preview on the right.

## Features

- **Live preview** — the right pane renders your Markdown instantly as you type
- **Rendered output** — preview is displayed as formatted content (headings, bold, italic, lists, code blocks, tables), not raw HTML
- **File operations** — open, save, and save-as support
- **Markdown help** — built-in cheat sheet accessible via the Help button in the menu bar
- **Native performance** — no Electron, no browser engine; runs as a lean native binary

## Screenshots

```
┌─────────────────────────────────────────────────┐
│ File  │  Help                                   │
├───────────────────┬─────────────────────────────┤
│ Markdown          │ Preview                     │
│                   │                             │
│ # Hello           │  Hello                      │
│                   │                             │
│ **bold** text     │  bold text                  │
│                   │                             │
│ - item one        │  • item one                 │
│ - item two        │  • item two                 │
│                   │                             │
├───────────────────┴─────────────────────────────┤
│ (untitled)                                      │
└─────────────────────────────────────────────────┘
```

## Requirements

- [Rust](https://www.rust-lang.org/) 1.76 or later
- macOS, Windows, or Linux (any platform supported by egui/eframe)
- On macOS: Xcode Command Line Tools (`xcode-select --install`)

## Build & Run

```bash
# Clone the repository
git clone <repo-url>
cd markdwoneditor

# Run in development mode
cargo run

# Build an optimized release binary
cargo build --release
# Binary is at: target/release/markdown_editor_rust
```

## Usage

| Action | How |
|--------|-----|
| Open a file | File → Open… |
| Save | File → Save |
| Save to a new path | File → Save As… |
| Show Markdown syntax reference | Click **Help** in the menu bar |

Type Markdown in the left pane. The right pane updates the rendered preview automatically.

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| [eframe](https://crates.io/crates/eframe) | 0.29 | Native window and event loop |
| [egui](https://crates.io/crates/egui) | 0.29 | Immediate-mode GUI framework |
| [egui_commonmark](https://crates.io/crates/egui_commonmark) | 0.18 | Markdown rendering inside egui |
| [pulldown-cmark](https://crates.io/crates/pulldown-cmark) | 0.12 | CommonMark-compliant Markdown parser |

## License

MIT
