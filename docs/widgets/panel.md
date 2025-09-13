# Widget: Panel

The `panel` widget renders a nested two-pane layout inside Pane B, supporting A/B content and nested focus.

## Spec fields
- `type`: `panel`
- `layout`: `horizontal` or `vertical`
- `size`: ratio string: `1:1`, `1:2`, `2:1`, `1:3`, `3:1`, `2:3`, `3:2`
- `a`: subpane A source (object with `cmd:` or `yaml:`)
- `b`: subpane B source (object with `cmd:` or `yaml:`)

## Example (inline spec)

```yaml
type: panel
layout: horizontal
size: "1:2"
a:
  yaml: "chi-index.yaml"   # path to your AppConfig (relative to CHI_TUI_CONFIG_DIR)
b:
  cmd: "${APP_BIN} list-items"
```

## Behavior
- Nested panel rendered inside Pane B; Tab cycles focus across all panes: `A → B.A → B.B → A` (Shift+Tab w odwrotnej kolejności)
- Subpane content is rendered with the unified ResultViewer when it’s JSON/JSON-like
- Subpane sources load synchronously for small YAML/JSON; larger sources can be loaded async via effects
- Scroll and viewer keys (`j`/`w`/Up/Down/PageUp/PageDown/Home/End`) are forwarded to the focused subpane
- Panel size ratios honor the provided `size` (e.g., `1:2` ≈ 33/67, `2:1` ≈ 67/33)

Notes:
- For menu items that need streaming output to work even when a panel is open, set `stream: true` on the menu item (see configuration guide). The TUI will run the command in stream mode and render it using the same viewer.

## How to verify
- Build: `cd rust-tui && cargo check`
- Run: `example-app ui`
- Otwórz dowolny ekran z panelem w example-app i sprawdź układ oraz nawigację (Tab: A → B.A → B.B → A)
