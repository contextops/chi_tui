# Widget: JSON Viewer

The `json_viewer` widget displays data using the unified ResultViewer renderer. It accepts either a CLI command (producing JSON) or a path to a YAML/JSON file.

## Spec fields
- `type`: `json_viewer` (alias `json-viewer` is normalized)
- `cmd`: command to execute that returns JSON (envelope or raw JSON)
- `yaml`: path to a YAML/JSON file to render
- `title` (optional): header title when provided as a spec

## Examples

Command source:

```yaml
type: json_viewer
cmd: "example-app list-items"
```

YAML source:

```yaml
type: json_viewer
yaml: "panels/panel_b.yaml"
```

## Behavior
- Uses the same renderer everywhere (ResultViewer): global JSON, Pane B results, and nested panels.
- Pretty “human” view by default (filters technical fields like `version`, `ts`, `request_id`, and hides `ok: true`).
- Toggle raw JSON: press `j` (also resets scroll to top when toggled).
- Toggle wrapping: `w`. Scroll: Up/Down/PageUp/PageDown/Home/End.
- Content loads asynchronously when backed by a command.

## How to verify
- Build: `cd rust-tui && cargo check`
- Run: `example-app ui`
- Navigate to `[[7]] Panel Demo (YAML json_viewer)` → Pane B shows the pretty view; press `j` to switch to raw JSON.
