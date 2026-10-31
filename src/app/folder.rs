//! "Open Folder": a side panel listing every Markdown file under a chosen folder,
//! so a multi-file set of notes can be switched between with one click.
//!
//! Picking and scanning happen on a background thread (the `rfd` dialog blocks), using
//! the same store-`Receiver` + poll-in-`update` pattern as the other file operations.

use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};

/// Don't descend forever into deep trees (or symlink loops).
const MAX_DEPTH: usize = 8;

pub struct Folder {
    pub root: PathBuf,
    /// Markdown files under `root`, sorted by relative path.
    pub files: Vec<PathBuf>,
}

#[derive(Default)]
pub struct FolderPanel {
    pub folder: Option<Folder>,
    pub visible: bool,
    filter: String,
    rx: Option<Receiver<Option<Folder>>>,
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown"))
}

/// Recursively collect Markdown files, skipping hidden entries and build/dependency dirs.
pub fn scan(root: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') {
                continue;
            }
            let Ok(ft) = entry.file_type() else { continue };
            if ft.is_dir() {
                if depth < MAX_DEPTH && !matches!(name.as_ref(), "node_modules" | "target") {
                    walk(&path, depth + 1, out);
                }
            } else if is_markdown(&path) {
                out.push(path);
            }
        }
    }

    let mut out = Vec::new();
    walk(root, 0, &mut out);
    out.sort_by_key(|p| {
        p.strip_prefix(root)
            .unwrap_or(p)
            .to_string_lossy()
            .to_lowercase()
    });
    out
}

impl FolderPanel {
    pub fn busy(&self) -> bool {
        self.rx.is_some()
    }

    /// Show a native folder picker on a background thread, then scan the chosen folder.
    pub fn pick(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        std::thread::spawn(move || {
            let folder = rfd::FileDialog::new().pick_folder().map(|root| Folder {
                files: scan(&root),
                root,
            });
            tx.send(folder).ok();
        });
    }

    /// (Re)scan `root` on a background thread without a dialog.
    pub fn load(&mut self, root: PathBuf) {
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        std::thread::spawn(move || {
            let folder = root.is_dir().then(|| Folder {
                files: scan(&root),
                root,
            });
            tx.send(folder).ok();
        });
    }

    /// Poll the background scan. Returns the folder's root when a new one finished loading.
    pub fn poll(&mut self) -> Option<PathBuf> {
        let received = match self.rx.as_ref()?.try_recv() {
            Err(TryRecvError::Empty) => return None,
            other => other,
        };
        // Done either way: a result arrived, or the thread died (Disconnected) without
        // sending one. Clearing `rx` in both cases keeps `busy()` from sticking at true.
        self.rx = None;
        let folder = received.ok()??;
        let root = folder.root.clone();
        self.folder = Some(folder);
        self.visible = true;
        Some(root)
    }

    pub fn close(&mut self) {
        self.folder = None;
        self.filter.clear();
    }

    /// Render the file list. Returns the file to open, if one was clicked.
    pub fn show(&mut self, ui: &mut egui::Ui, current: Option<&Path>) -> Option<PathBuf> {
        let Some(folder) = &self.folder else {
            ui.weak("フォルダが開かれていません");
            if ui.button("Open Folder…").clicked() {
                self.pick();
            }
            return None;
        };

        let mut open = None;
        let mut refresh = false;
        let mut close = false;

        let root_name = folder
            .root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| folder.root.to_string_lossy().to_string());
        ui.horizontal(|ui| {
            ui.strong(format!("📁 {root_name}"))
                .on_hover_text(folder.root.to_string_lossy());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("✖").on_hover_text("Close folder").clicked() {
                    close = true;
                }
                if ui.small_button("⟳").on_hover_text("Refresh").clicked() {
                    refresh = true;
                }
            });
        });
        ui.add(egui::TextEdit::singleline(&mut self.filter).hint_text("Filter…"));
        ui.separator();

        let needle = self.filter.to_lowercase();
        egui::ScrollArea::vertical()
            .id_salt("folder_scroll")
            .show(ui, |ui| {
                let mut shown = 0;
                for path in &folder.files {
                    let rel = path.strip_prefix(&folder.root).unwrap_or(path);
                    let rel_str = rel.to_string_lossy();
                    if !needle.is_empty() && !rel_str.to_lowercase().contains(&needle) {
                        continue;
                    }
                    shown += 1;

                    // File name in normal text, its subfolder (if any) dimmed after it.
                    let name = rel
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let mut label = egui::text::LayoutJob::default();
                    let style = ui.style();
                    egui::RichText::new(&name).append_to(
                        &mut label,
                        style,
                        egui::FontSelection::Default,
                        egui::Align::Center,
                    );
                    if let Some(dir) = rel.parent().filter(|d| !d.as_os_str().is_empty()) {
                        egui::RichText::new(format!("  {}", dir.to_string_lossy()))
                            .weak()
                            .small()
                            .append_to(
                                &mut label,
                                style,
                                egui::FontSelection::Default,
                                egui::Align::Center,
                            );
                    }

                    let selected = current == Some(path.as_path());
                    if ui
                        .selectable_label(selected, label)
                        .on_hover_text(rel_str)
                        .clicked()
                        && !selected
                    {
                        open = Some(path.clone());
                    }
                }
                if shown == 0 {
                    ui.weak("Markdown ファイルがありません");
                }
            });

        if refresh {
            let root = folder.root.clone();
            self.load(root);
        }
        if close {
            self.close();
        }
        open
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poll_clears_busy_when_scan_thread_dies() {
        let mut panel = FolderPanel::default();
        let (tx, rx) = mpsc::channel();
        panel.rx = Some(rx);
        assert_eq!(panel.poll(), None);
        assert!(panel.busy(), "still waiting while the sender is alive");
        drop(tx); // simulates the thread panicking before it sends
        assert_eq!(panel.poll(), None);
        assert!(!panel.busy());
    }

    #[test]
    fn scans_markdown_recursively_and_skips_hidden_and_build_dirs() {
        let root = std::env::temp_dir().join(format!("mdfolder_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for dir in ["sub", ".git", "node_modules", "target"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        for file in [
            "b.md",
            "A.MD",
            "notes.txt",
            "sub/c.markdown",
            ".hidden.md",
            ".git/x.md",
            "node_modules/y.md",
            "target/z.md",
        ] {
            std::fs::write(root.join(file), "").unwrap();
        }

        let rel: Vec<_> = scan(&root)
            .iter()
            .map(|p| {
                p.strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        std::fs::remove_dir_all(&root).unwrap();

        assert_eq!(rel, ["A.MD", "b.md", "sub/c.markdown"]);
    }
}
