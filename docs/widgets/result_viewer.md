# Renderer: ResultViewer

ResultViewer is the unified renderer used across the TUI to display results and JSON. It powers:
- Global JSON view
- Pane B results after running commands
- Nested panel subpanes (A/B) when they display JSON-like content

## Features
- Pretty “human” view by default:
  - Filters technical fields such as `version`, `ts`, `request_id`, and envelope `type`
  - Keeps `ok=false` errors visible; hides `ok=true`
  - Highlights keys and common value types; compact list/object summaries
- Raw JSON toggle: press `j` to switch to raw, press again to return to pretty
- Wrap toggle: `w`
- Scroll: Up/Down/PageUp/PageDown/Home/End

## Integration
- `json_viewer` widget specs delegate to ResultViewer
- Panel subpanes (when showing data) use ResultViewer; keys `j`/`w`/scroll are forwarded to the focused subpane
- Global JSON (outside of Panel mode) is rendered by the same ResultViewer for full consistency

## Why unify?
One renderer means consistent visuals, behavior, and shortcuts regardless of where the result comes from. It also enables a single place to refine the “human” view (e.g., filter rules, colors) and have those improvements apply everywhere.

