use eframe::egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use std::fs;
use std::path::PathBuf;

const HELP_MD: &str = r#"# Markdown Cheat Sheet

## Headings
```
# H1
## H2
### H3
```

## Emphasis
```
**bold**  or  __bold__
*italic*  or  _italic_
~~strikethrough~~
```

## Lists
```
- Unordered item
- Another item
  - Nested item

1. Ordered item
2. Second item
```

## Links & Images
```
[Link text](https://example.com)
![Alt text](image.png)
```

## Code
```
`inline code`

    indented code block (4 spaces)
```

Fenced block:
````
```rust
fn main() {}
```
````

## Blockquote
```
> This is a blockquote
```

## Horizontal Rule
```
---
```

## Table
```
| Column A | Column B |
|----------|----------|
| cell 1   | cell 2   |
```
"#;

struct MarkdownEditorApp {
    text: String,
    file_path: Option<PathBuf>,
    md_cache: CommonMarkCache,
    help_cache: CommonMarkCache,
    show_help: bool,
}

impl MarkdownEditorApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            text: String::from(
                "# Hello from Markdown Editor\n\nStart typing **Markdown** here.\n\n- Live preview\n- File open/save\n- Fast native Rust app\n",
            ),
            file_path: None,
            md_cache: CommonMarkCache::default(),
            help_cache: CommonMarkCache::default(),
            show_help: false,
        }
    }

    fn open_file(&mut self, path: PathBuf) {
        if let Ok(content) = fs::read_to_string(&path) {
            self.text = content;
            self.file_path = Some(path);
        }
    }

    fn save_file(&mut self) {
        if let Some(path) = &self.file_path.clone() {
            if let Err(e) = fs::write(path, &self.text) {
                eprintln!("Failed to save file: {}", e);
            }
        } else {
            eprintln!("No file path set; use Save As.");
        }
    }

    fn save_as(&mut self, path: PathBuf) {
        if let Err(e) = fs::write(&path, &self.text) {
            eprintln!("Failed to save file: {}", e);
        } else {
            self.file_path = Some(path);
        }
    }
}

impl eframe::App for MarkdownEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open…").clicked() {
                        self.open_file(PathBuf::from("example.md"));
                        ui.close_menu();
                    }
                    if ui.button("Save").clicked() {
                        self.save_file();
                        ui.close_menu();
                    }
                    if ui.button("Save As… (stub)").clicked() {
                        self.save_as(PathBuf::from("output.md"));
                        ui.close_menu();
                    }
                });
                ui.separator();
                if ui.button("Help").clicked() {
                    self.show_help = true;
                }
            });
        });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            let label = self
                .file_path
                .as_deref()
                .and_then(|p| p.to_str())
                .unwrap_or("(untitled)");
            ui.label(label);
        });

        egui::SidePanel::left("editor_panel")
            .resizable(true)
            .default_width(600.0)
            .show(ctx, |ui| {
                ui.label("Markdown");
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt("editor_scroll")
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.text)
                                .desired_width(f32::INFINITY)
                                .desired_rows(40)
                                .font(egui::TextStyle::Monospace),
                        );
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Preview");
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("preview_scroll")
                .show(ui, |ui| {
                    CommonMarkViewer::new()
                        .show(ui, &mut self.md_cache, &self.text);
                });
        });

        if self.show_help {
            egui::Window::new("Markdown Help")
                .open(&mut self.show_help)
                .resizable(true)
                .default_width(500.0)
                .default_height(600.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("help_scroll")
                        .show(ui, |ui| {
                            CommonMarkViewer::new()
                                .show(ui, &mut self.help_cache, HELP_MD);
                        });
                });
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Markdown Editor (Rust + egui)")
            .with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Markdown Editor",
        options,
        Box::new(|cc| Ok(Box::new(MarkdownEditorApp::new(cc)))),
    )
}
