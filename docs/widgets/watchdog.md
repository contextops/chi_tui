# Watchdog Widget

Run multiple shell commands and stream their logs side-by-side in vertical sections.

Spec fields:
- `type`: `watchdog`
- `commands`: list of command lines
- `sequential` (optional, default false): run one-by-one instead of in parallel
- `auto_restart` (optional, default false): restart failing command
- `max_retries` (optional, default 0): number of retries before panic
- `restart_delay_ms` (optional, default 1000): delay between retries
- `stop_on_failure` (optional, default false): for `sequential` — abort subsequent commands after panic
- `allowed_exit_codes` (optional, default [0]): treat these exit codes as success
- `on_panic_exit_cmd` (optional): command to run when retries are exhausted
- `title` (optional, for spec files): overrides the widget header title
- `pane_b_title` (optional): overrides the title when rendered in Pane B
- `stats` (optional): list of `{label, regexp}` patterns to count across all logs
- `external_check_cmd` (optional): enable external mode — do not spawn processes, periodically run this command to detect an already-running process (exit code `0` ⇒ running)
- `external_kill_cmd` (optional): command used to terminate the external process when pressing `s`

Example:

```yaml
- id: "watchdog_demo"
  title: "Watchdog (2 cmds) [[24]]"
  widget: "watchdog"
  sequential: true
  commands:
    - "bash -lc 'echo One; sleep 1; echo Done 1'"
    - "bash -lc 'echo Two; sleep 1; echo Done 2'"
```

Keys: Tab/Shift+Tab (zmiana aktywnej sekcji), ↑/↓, PgUp/PgDn, Home, End, f, s, r.

Notes:
- Parallel mode ignores `stop_on_failure` (applies to sequential only).
- On panic, widget appends a line `[panic: retries exhausted]` and (if provided) runs `on_panic_exit_cmd`.
- Regexes in `stats` run against each raw output line (stdout/stderr). If your backend emits JSON envelopes (e.g., via `emit_progress`), the match still works on the full line (including message text inside the JSON).
- Auto-follow: logs auto-follow the latest output by default. Any manual scroll (↑/↓/PgUp/PgDn/Home) pauses follow. Press `End` lub `f` aby wznowić auto-follow i przejść na dół w aktywnej sekcji.
- Fokus sekcji: gdy widget jest aktywny w Panelu B, tylko jedna sekcja (log) jest podświetlona; Tab/Shift+Tab zmienia aktywną sekcję. Przewijanie dotyczy wszystkich sekcji jednocześnie.
- Start/Stop/Restart: `s` przełącza start/stop i teraz kończy aktywne procesy (kill). `r` czyści bufory i restartuje wszystkie komendy z polityką retry.
  - W trybie external: `s` wywołuje `external_kill_cmd` (jeśli ustawione), a `r` jest niedostępne.

## Quick Reference

Field | Type | Default | Description
----- | ---- | ------- | -----------
`commands` | array[string] | — | List of shell commands to run (one per section)
`sequential` | bool | `false` | Run commands one-after-another (vs parallel)
`auto_restart` | bool | `false` | Retry a failing command automatically
`max_retries` | int | `0` | Maximum retries when `auto_restart=true`
`restart_delay_ms` | int (ms) | `1000` | Delay before each retry
`allowed_exit_codes` | array[int] | `[0]` | Exit codes treated as success
`stop_on_failure` | bool | `false` | Sequential only: abort remaining commands after a panic
`on_panic_exit_cmd` | string | — | Optional hook run when retries are exhausted (panic)
`stats` | array[{label, regexp}] | — | Aggregate matches across all panes and show a footer summary
`external_check_cmd` | string | — | If set, do not spawn; detect external process via exit code 0
`external_kill_cmd` | string | — | Kill command used in external mode when pressing `s`

## Stats Footer

When `stats` are defined, Watchdog renders a compact footer with per-pattern counts:

- Format: `● LABEL  × COUNT` (color-coded by label: ERROR=red, WARN=yellow, INFO=cyan, DEBUG=blue, other=gray)
- The footer uses a subtle dark background to separate it from the logs
- Counts update live as new lines arrive

Behavior notes:
- Success is determined by the process exit code being in `allowed_exit_codes` (or `[0]` by default).
- When a command panics (retries exhausted), a log line is added and `on_panic_exit_cmd` (if set) is executed with its output appended.
- In sequential mode with `stop_on_failure=true`, remaining sections get a marker `[aborted by stop_on_failure]`.
- External mode: widget nie uruchamia komend; status "running (external init)" jest pokazywany, gdy `external_check_cmd` zwraca kod 0. Menu wykorzystuje ten stan do wskaźnika gwiazdki. `s` może wywołać `external_kill_cmd`.
