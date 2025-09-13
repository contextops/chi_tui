# Widget Reference

## Menu Widgets

### Basic Menu Item
```yaml
- id: "item_id"
  title: "Display Title"
  command: "${APP_BIN} command --args"
```

### Nested Menu
```yaml
- id: "parent"
  title: "Parent Item"
  children:
    - id: "child1"
      title: "Child 1"
```

### Header Widget
```yaml
- id: "section"
  title: "── Section Header ──"
  widget: "header"
```

## List Widgets

### Lazy Items (Load on Demand)
```yaml
- id: "lazy_list"
  widget: "lazy_items"
  command: "${APP_BIN} list-items"
  unwrap: "data.items"
  initial_text: "Press Enter to load..."
```

### Auto-load Items
```yaml
- id: "auto_list"
  widget: "autoload_items"
  command: "${APP_BIN} list-items"
  unwrap: "data.items"
```

## Panel Widget

### Horizontal Split
```yaml
- id: "split_view"
  widget: "panel"
  panel_layout: "horizontal"
  panel_size: "1:2"  # Left:Right ratio
  pane_b_yaml: "panels/right.yaml"
```

### Vertical Split
```yaml
- id: "split_view"
  widget: "panel"
  panel_layout: "vertical"
  panel_size: "2:1"  # Top:Bottom ratio
  pane_b_yaml: "panels/bottom.yaml"
```

## Form Widgets

### Simple Form
```yaml
type: form
title: "Input Form"
schema_cmd: "${APP_BIN} schema command-name"
submit_cmd: "${APP_BIN} command-name"
fields:
  - name: field_name
    label: "Field Label"
    type: text
    required: true
```

### Field Types
- **text** - Single line text
- **password** - Hidden input
- **textarea** - Multi-line text
- **bool** - Checkbox
- **select** - Dropdown menu
- **number** - Numeric input

### Grouped Form
```yaml
type: form
groups:
  - title: "Section 1"
    fields:
      - name: field1
        label: "Field 1"
  - title: "Section 2"
    fields:
      - name: field2
        label: "Field 2"
```

## JSON Viewer

### Basic JSON Display
```yaml
type: json_viewer
cmd: "${APP_BIN} get-data"
```

### With Data Unwrapping
```yaml
type: json_viewer
cmd: "${APP_BIN} get-data"
unwrap: "data.results"
```

## Special Attributes

### Key Bindings
```yaml
- id: "item"
  title: "Item [[7]]"  # Binds to key '7'
```

### Conditional Display
```yaml
- id: "item"
  title: "Debug Mode"
  visible_when: "${DEBUG_MODE}"
```

### Dynamic Content
```yaml
- id: "item"
  title: "Current Time"
  command: "${APP_BIN} get-time"
  refresh_interval: 1000  # ms
```

### Streaming Commands
Run a command in streaming mode (progress updates), even if a panel is open:
```yaml
- id: "progress"
  title: "Simulate Progress"
  command: "${APP_BIN} simulate-progress --steps 12 --delay-ms 200"
  stream: true
```

## Layout Tips

1. **Use panels for complex layouts** - Split views are powerful
2. **Group related items** - Use headers to organize
3. **Progressive disclosure** - Start collapsed, expand on demand
4. **Consistent key bindings** - Numbers for quick access
5. **Error states** - Always handle command failures gracefully
