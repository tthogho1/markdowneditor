# Markdown Cheat Sheet

## Keyboard Shortcuts

Use `Cmd` on macOS and `Ctrl` on Windows / Linux.

| Shortcut | Action |
|----------------|------|
| `Cmd/Ctrl + N` | New file |
| `Cmd/Ctrl + O` | Open |
| `Cmd/Ctrl + S` | Save |
| `Cmd/Ctrl + Shift + S` | Save As |
| `Cmd/Ctrl + F` | Open find/replace bar |
| `Cmd/Ctrl + G` | Go to next match |
| `Cmd/Ctrl + Shift + G` | Go to previous match |
| `Esc` | Close find bar |
| `Cmd/Ctrl + B` | Bold |
| `Cmd/Ctrl + I` | Italic |
| `Cmd/Ctrl + K` | Link |
| `Cmd/Ctrl + Shift + O` | Toggle outline view |
| `Cmd/Ctrl + Shift + E` | Toggle files panel |

When there are unsaved changes, a `●` is shown in the title bar and status bar,
and a confirmation dialog appears before creating a new file, opening a file, or exiting.

You can toggle scroll synchronization between the editor and preview using
`Sync scroll` in the `View` menu.

`File → Open Folder…` lists every Markdown file in a folder (and its subfolders) in the
Files panel; click a file to switch to it. The folder is reopened on the next launch.

`Edit → Insert Table of Contents` (or the `☰ TOC` toolbar button) inserts a linked
list of all headings at the cursor. Clicking an entry in the preview jumps to that
heading, and the links also work in exported HTML. Re-insert it after changing headings.

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
