# CHI TUI Implementation

This directory contains the Terminal UI implementation for CHI SDK.

## For Users

**You don't need to build this!** The TUI is included when you install CHI SDK:

```bash
pip install chi-sdk
```

## For Contributors

This is a Rust/Ratatui implementation that renders the TUI based on JSON from Python CLIs.

### Building

```bash
cargo build --release
```

### Testing

```bash
cargo test
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

### Headless/Local Run (development)

Run the TUI directly via `cargo` and point it at your config directory. Set `CHI_APP_BIN` so the TUI can invoke your backend:

```bash
cd rust-tui
CHI_APP_BIN=../.venv/bin/example-app \
CHI_TUI_CONFIG_DIR=../example-apps/example-app/.tui \
CHI_TUI_HEADLESS=1 CHI_TUI_TICKS=15 cargo run -q
```

Config resolution: the TUI expects an entry file `chi-index.yaml` inside `CHI_TUI_CONFIG_DIR`. All relative paths in YAML resolve against `CHI_TUI_CONFIG_DIR`.

### Architecture

The TUI is a thin presentation layer that:
1. Reads JSON schema from the Python CLI
2. Renders an interactive interface
3. Calls CLI commands based on user input
4. Displays JSON responses

The Python SDK remains the source of truth for all business logic.
