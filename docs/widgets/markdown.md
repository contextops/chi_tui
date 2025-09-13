# Markdown Widget

Render a Markdown file or inline text in a scrollable pane.

Spec fields:
- `type`: `markdown` (alias: `markdown-viewer`)
- `path`: path to a `.md` file (resolved against `CHI_TUI_CONFIG_DIR`)
- `text` (optional): inline Markdown string
- `title` (optional, for spec files): overrides the widget header title
- `pane_b_title` (optional): overrides the title when rendered in Pane B

Example:

```yaml
- id: "markdown_demo"
  title: "Markdown (README) [[23]]"
  widget: "markdown"
  path: "config/markdown_demo.md"
  pane_b_title: "Docs — Markdown README"
```

Keys: ↑/↓, PgUp/PgDn, Home/End; `w` toggles wrapping.
