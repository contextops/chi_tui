# Widget: Form

The `form` widget renders input fields based on a simple spec or on the CLI `schema` command.

## Spec fields
- `type`: `form`
- `title`: form title
- `fields[]`: array of field definitions
  - `name`, `label`, `type` (`text`, `password`, `textarea`, `checkbox`, `select`, `multiselect`, `number`, `array`)
  - `required`: bool (optional)
  - `default`: default value (optional)
  - `options`: for select/multiselect (static list)
  - `options_cmd` + `unwrap`: dynamic options source (CLI command + JSON path)
- Grouping and ordering:
  - `group`: optional group header name (per field)
  - `order`: optional ordering integer (per field)
- Submit command:
  - `submit.command`: CLI command to run on submit
  - `submit_cmd`: legacy/alias supported; also recognized if present
  - If neither is set and a menu item `command` is provided, it will be used
- Schema loading (optional):
  - `schema_cmd`: explicit CLI command to fetch a schema; if absent, the TUI attempts `${APP_BIN} schema` derived from `submit_cmd`

## Example (basic)

```yaml
type: form
title: "Hello Form"
fields:
  - name: "name"
    label: "Your name"
    type: "text"
    default: "Ada"
    required: true
  - name: "shout"
    label: "Uppercase"
    type: "checkbox"
    default: false
submit:
  command: "example-app hello"
```

## Behavior
- Enter toggles edit mode for text/number/password/textarea/select
- Space toggles checkbox/multiselect (in multiselect edit mode, cursor moves with Up/Down)
- Numbers support stepping with Up/Down; arrays accept comma‑separated values
- Submit validates and runs `submit.command`; server‑side errors map inline to field errors
- Textarea edits open a modal editor powered by `tui-textarea`:
  - Ctrl+S: save & close
  - Esc: cancel (no save)
- Fields with a `group` are rendered under group headers; `order` controls display order within a group
- When `schema_cmd` (or derived schema) is available, the form attempts to pre-fill constraints and field kinds

## How to verify
- Build: `cd rust-tui && cargo check`
- Run: `example-app ui`
- Navigate to:
  - `[[10]] Hello (Form)` — basic submit flow
  - `[[12]] Tasks (Form Select)` — select with options
  - `[[22]] Text Area` — textarea modal (Ctrl+S / Esc)
