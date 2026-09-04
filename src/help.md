# Markdown Cheat Sheet

## キーボードショートカット (Keyboard Shortcuts)

macOS では `Cmd`、Windows / Linux では `Ctrl` を使います。

| ショートカット | 動作 |
|----------------|------|
| `Cmd/Ctrl + N` | 新規ファイル |
| `Cmd/Ctrl + O` | 開く |
| `Cmd/Ctrl + S` | 保存 |
| `Cmd/Ctrl + Shift + S` | 名前を付けて保存 |
| `Cmd/Ctrl + F` | 検索・置換バーを開く |
| `Cmd/Ctrl + G` | 次の検索結果へ |
| `Cmd/Ctrl + Shift + G` | 前の検索結果へ |
| `Esc` | 検索バーを閉じる |
| `Cmd/Ctrl + B` | 太字 |
| `Cmd/Ctrl + I` | 斜体 |
| `Cmd/Ctrl + K` | リンク |
| `Cmd/Ctrl + Shift + O` | アウトライン表示の切り替え |

未保存の変更があるときはタイトルバーとステータスバーに `●` が表示され、
新規作成・ファイルを開く・終了の前に確認ダイアログが出ます。

`View` メニューの `Sync scroll` で、エディタとプレビューのスクロール同期を
オン / オフできます。

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
