# YAML Configuration Guide

## File Structure

```
.tui/
├── chi-index.yaml   # Entry point (F1)
├── *.yaml           # Other screens (F2–F6)
├── panels/          # Panel configurations
│   └── *.yaml       # Reusable panel configs
└── docs/            # Documentation
    └── *.md         # Markdown files
```

## Screen Configuration

### Basic Structure
```yaml
header: "Application Title"
auto_enter: "menu_id"     # Optional: open this item on load
can_close: true           # Optional: allow closing panels with Esc
horizontal_menu:     # Optional tab bar
  - id: "tab1"
    title: "Tab 1"
    config: "screen1.yaml"
menu:               # Menu items
  - id: "item1"
    title: "Item 1"
```

### Horizontal Menu
Define tabs accessible via F1-F12:

```yaml
horizontal_menu:
  - id: "home"
    title: "Home"
    # No config = stay on current screen
  - id: "settings"
    title: "Settings"
    config: "settings.yaml"  # Load different screen
```

### Auto-enter and Close Control
- `auto_enter`: when set to a menu item `id`, triggers Enter on that item after the screen loads (useful to show a panel by default). Focus remains on the left menu.
- `can_close`: when `false`, pressing Esc will not close the panel view (handy to lock a particular layout). Default `true`.

## Menu Items

### Simple Command
```yaml
- id: "hello"
  title: "Say Hello"
  command: "${APP_BIN} hello --name World"
```

### Nested Menu
```yaml
- id: "parent"
  title: "Options"
  children:
    - id: "opt1"
      title: "Option 1"
    - id: "opt2"
      title: "Option 2"
```

### Dynamic Lists
```yaml
- id: "items"
  widget: "autoload_items"
  command: "${APP_BIN} list-items"
  unwrap: "data.items"  # Path to array in JSON
```

### Panels
```yaml
- id: "split"
  widget: "panel"
  panel_layout: "horizontal"  # or "vertical"
  panel_size: "1:2"           # Ratio
  pane_b_yaml: "panels/detail.yaml"
```

## Panel Configuration

### JSON Viewer Panel
```yaml
type: json_viewer
cmd: "${APP_BIN} get-data"
unwrap: "data"  # Optional path
```

### Form Panel
```yaml
type: form
title: "User Form"
schema_cmd: "${APP_BIN} schema save-user"
submit_cmd: "${APP_BIN} save-user"
fields:
  - name: username
    label: "Username"
    type: text
    required: true
  - name: active
    label: "Active"
    type: bool
    default: true
```

### Menu Panel
```yaml
type: menu
title: "Options"
menu:
  - id: "opt1"
    title: "Option 1"
    command: "${APP_BIN} cmd1"
```

## Variables

### Built-in Variables
- `${APP_BIN}` - Your application binary
- `${CHI_TUI_CONFIG_DIR}` - Config directory path

### Environment Variables
Any environment variable can be used:
```yaml
command: "${MY_APP} --config ${CONFIG_PATH}"
```

## Advanced Features

### Key Bindings
Add `[[key]]` to bind to keyboard:
```yaml
- id: "item"
  title: "Quick Access [[1]]"  # Press '1' to select
```

### Conditional Display
```yaml
- id: "debug"
  title: "Debug Info"
  visible_when: "${DEBUG_MODE}"
```

### Auto-refresh
```yaml
- id: "status"
  command: "${APP_BIN} status"
  refresh_interval: 5000  # Refresh every 5 seconds
```

### Streaming Commands
Force a command to run in streaming mode (show progress updates) even when a panel is open:
```yaml
- id: "progress"
  title: "Simulate Progress"
  command: "${APP_BIN} simulate-progress --steps 10"
  stream: true
```

## Best Practices

1. **Organize by feature** - Group related screens
2. **Reuse panel configs** - DRY principle
3. **Use meaningful IDs** - For debugging
4. **Handle errors** - Provide fallback text
5. **Document complex configs** - Use YAML comments

## Common Patterns

### Master-Detail View
```yaml
- id: "list_view"
  widget: "panel"
  panel_layout: "horizontal"
  panel_size: "1:2"
  pane_b_yaml: "panels/item_detail.yaml"
```

### Form with Result
```yaml
- id: "process"
  widget: "panel"
  panel_layout: "vertical"
  panel_size: "1:1"
  pane_b_yaml: "panels/form_result.yaml"
```

### Progressive Loading
```yaml
- id: "data"
  widget: "lazy_items"
  command: "${APP_BIN} fetch-data"
  initial_text: "Press Enter to load..."
```
