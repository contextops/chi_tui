# Widget: Menu

The `menu` widget renders a navigable list using an AppConfig (same shape as `.tui/chi-index.yaml`).

## Spec fields
- `type`: `menu`
- One of:
  - `spec`: path to an AppConfig YAML file
  - `config`: inline AppConfig object

## Examples

From a YAML file:

```yaml
type: menu
spec: "chi-index.yaml"  # path to your AppConfig (relative to CHI_TUI_CONFIG_DIR)
```

Inline:

```yaml
type: menu
config:
  header: "Demo"
  menu:
    - id: "welcome"
      title: "Welcome"
```

## Behavior
- Renders a list in Pane B with its own selection and scroll offset
- Key bindings: Up/Down/Home/End/PageUp/PageDown

Streaming:
- Set `stream: true` on a menu item to force stream mode, including when a panel is open; the output renders using the unified ResultViewer.

## How to verify
- Build: `cd rust-tui && cargo check`
- Run: `example-app ui`
- Dodaj w swoim `.tui/*.yaml` widget `menu` i sprawdź przewijanie oraz selekcję w panelu B.
