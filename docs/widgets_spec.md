# Widget Specs — Menu, Viewer, and JSON Viewer

This TUI resolves select YAML/JSON widget specs via a central registry.

- Type names are normalized (e.g., `json-viewer` -> `json_viewer`).
- Unknown or partial specs fall back to effect-based loading or plain JSON view.

## result_viewer (renderer)

Not a spec type — this is the unified renderer used everywhere to display data (pretty view + raw JSON). Widgets like `json_viewer`, Pane B results, and nested panel contents all delegate to it.

Behavior:

- Pretty “human” view by default (filters common metadata fields; keeps `ok=false`)
- Toggle raw JSON: `j`; wrapping: `w`; scroll: arrows/PgUp/PgDn/Home/End
- Same key bindings and visuals across all contexts

## json_viewer

Supported fields:

- `type`: `json_viewer` (or `json-viewer`)
- `cmd`: shell command to produce JSON (as envelope or raw JSON)
- `yaml`: path to a YAML file containing JSON (resolved relative to `CHI_TUI_CONFIG_DIR`)
- `title`: optional custom title (when provided as a spec)

Examples:

```yaml
type: json_viewer
cmd: "example-app list-items"
```

```yaml
type: json_viewer
yaml: ".tui/panels/panel_b.yaml"
```

Behavior:

- Delegates rendering to the unified `result_viewer` renderer
- A placeholder viewer is created immediately for the target pane; content loads asynchronously via effects (`LoadPanelCmd` / `LoadPanelYaml`)
- Keys: arrows/PgUp/PgDn/Home/End; `w` toggles wrap; `j` toggles raw JSON

## menu

Supported fields:

- `type`: `menu`
- `title` (optional): header title for the menu widget
- One of:
  - `spec`: path to an AppConfig YAML file (`header`, `menu`)
  - `config`: inline AppConfig object (same shape as file)

Examples:

```yaml
type: menu
spec: "chi-index.yaml"
```

```yaml
type: menu
config:
  header: "Demo"
  menu:
    - id: "welcome"
      title: "Welcome"
```

Behavior:

- Registry loads the AppConfig (from path or inline) and builds a `MenuWidget`.
- Default title: "Pane B — Menu" (Pane A analogously), or value of `title`.

Streaming:
- For menu items that must stream even when a panel is open, set `stream: true` on the item (see configuration guide). The TUI routes these to stream mode and renders with the unified viewer.

## markdown

Supported fields:

- `type`: `markdown` (alias: `markdown-viewer`)
- `path`: path to a Markdown file (resolved against `CHI_TUI_CONFIG_DIR`)
- `text` (optional): inline Markdown string (for quick demos/tests)
- `title` (optional): custom title (when provided as a spec)

Example:

```yaml
type: markdown
path: "config/README.md"
```

Behavior:

- Renders Markdown lines with simple emphasis and fenced code blocks highlighted via syntect.
- Keys: ↑/↓, PgUp/PgDn, Home/End, `w` toggles wrap.

## watchdog

Supported fields:

- `type`: `watchdog`
- `commands`: list of shell command lines
- `sequential` (optional, default: false): when true, run commands one after another
- `auto_restart` (optional, default false)
- `max_retries` (optional, default 0)
- `restart_delay_ms` (optional, default 1000)
- `stop_on_failure` (optional, default false; sequential only)
- `allowed_exit_codes` (optional, default [0])
- `on_panic_exit_cmd` (optional): command to run when retries are exhausted
- `external_check_cmd` (optional): if set, Watchdog does not spawn commands; instead, it periodically runs this command (exit code `0` means "external process running").
- `external_kill_cmd` (optional): command to terminate the external process (used when pressing `s`).

Example:

```yaml
type: watchdog
sequential: true
commands:
  - "bash -lc 'echo one; sleep 1; echo done 1'"
  - "bash -lc 'echo two; sleep 1; echo done 2'"
```

Behavior:

- Splits Pane B into N vertical sections (one per command) and streams output lines.
- Scroll: ↑/↓/PgUp/PgDn/Home (applies to all sections in tandem).
- If `external_check_cmd` is provided, the widget operates in external mode: it does not spawn processes, shows status "running (external init)" when detection succeeds, and `s` issues `external_kill_cmd` (if configured).
