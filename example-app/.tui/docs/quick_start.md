# Quick Start Guide

## Installation

```bash
pip install chi-sdk[tui]
```

## Creating Your First TUI App

### 1. Initialize Your Project

```bash
chi-admin init . --binary-name=myapp
```

This creates:
- `.tui/chi-index.yaml` - Your main navigation (entry)
- `.tui/panels/` - Panel configurations
- `.tui/docs/` - Documentation files

### 2. Define Your CLI Commands

```python
from chi_sdk import chi_command, build_cli
from pydantic import BaseModel

class HelloIn(BaseModel):
    name: str

class HelloOut(BaseModel):
    greeting: str

@chi_command(input_model=HelloIn, output_model=HelloOut)
def hello(inp: HelloIn) -> HelloOut:
    return HelloOut(greeting=f"Hello, {inp.name}!")

app = build_cli("myapp", [hello])
```

### 3. Configure Your Menu

Edit `.tui/chi-index.yaml`:

```yaml
header: "My Application"
menu:
  - id: "welcome"
    title: "Welcome"
  - id: "greet"
    title: "Say Hello"
    command: "${APP_BIN} hello --name World"
```

### 4. Run Your TUI

```bash
myapp ui
```

## Key Concepts

### Commands & JSON Envelope
Every command outputs structured JSON that the TUI can display:
- `emit_ok(data)` - Success response
- `emit_error(msg)` - Error response
- `emit_progress(text, percent)` - Streaming updates

### Widget Types
- **menu** - Navigation items
- **panel** - Split views
- **form** - Input forms
- **lazy_items** - Load on demand
- **autoload_items** - Auto-load lists

### Navigation
- Arrow keys to navigate
- Enter to select
- F1-F6 for tabs
- Esc to go back
- q to quit

## Next Steps

1. Explore each tab (F2-F6) to see patterns
2. Check `.tui/` for YAML examples
3. Copy patterns that fit your needs
4. Customize styling and branding

## Tips

- Start simple with static menus
- Add dynamic content gradually
- Use panels for complex layouts
- Test with `CHI_TUI_HEADLESS=1` for CI/CD
