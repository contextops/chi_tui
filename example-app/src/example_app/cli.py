from typing import List
import time

from pydantic import BaseModel, Field

from chi_sdk import chi_command, build_cli, emit_progress


class HelloIn(BaseModel):
    name: str = Field(..., description="Person's name")
    shout: bool = Field(False, description="Uppercase the greeting")


class HelloOut(BaseModel):
    greeting: str


class SumIn(BaseModel):
    numbers: List[float] = Field(
        ..., description="Numbers to sum (use --numbers multiple times)"
    )


class SumOut(BaseModel):
    total: float
    count: int

    def __str__(self):
        return f"Sum of {self.count} numbers: {self.total}"


class ItemsOut(BaseModel):
    items: List[dict]


class SimProgressIn(BaseModel):
    steps: int = Field(8, description="Number of progress steps", ge=1, le=100)
    delay_ms: int = Field(
        400, description="Delay per step in milliseconds", ge=50, le=5000
    )


class SimProgressOut(BaseModel):
    status: str
    steps: int
    duration_ms: int


@chi_command(input_model=HelloIn, output_model=HelloOut, description="Say hello.")
def hello(inp: HelloIn) -> HelloOut:
    text = f"Hello, {inp.name}!"
    if inp.shout:
        text = text.upper()
    return HelloOut(greeting=text)


@chi_command(
    input_model=SumIn, output_model=SumOut, description="Sum a list of numbers."
)
def sum_numbers(inp: SumIn) -> SumOut:
    total = float(sum(inp.numbers))
    return SumOut(total=total, count=len(inp.numbers))


@chi_command(output_model=ItemsOut, description="Produce a dynamic list for the TUI.")
def list_items() -> ItemsOut:
    data = [
        {"name": "Alpha", "value": 1},
        {"name": "Bravo", "value": 2},
        {"name": "Charlie", "value": 3},
        {"name": "Delta", "value": 4},
    ]
    return ItemsOut(items=data)


@chi_command(
    output_model=ItemsOut,
    description="List tag options (strings).",
    human_renderer=lambda data: "\n".join(
        f"📌 {item['title']} (use --tag {item['id']})" for item in data.get("items", [])
    ),
)
def list_tags() -> ItemsOut:
    # Return tags as simple strings in items array
    return ItemsOut(
        items=[
            {"title": "urgent", "id": "urgent"},
            {"title": "normal", "id": "normal"},
            {"title": "low", "id": "low"},
        ]
    )


# --------- Choose Tags (echo multiselect) ----------------------------------
class ChooseTagsIn(BaseModel):
    tags: List[str]


class ChooseTagsOut(BaseModel):
    selected: List[str]


@chi_command(
    input_model=ChooseTagsIn,
    output_model=ChooseTagsOut,
    description="Echo selected tags.",
)
def choose_tags(inp: ChooseTagsIn) -> ChooseTagsOut:
    return ChooseTagsOut(selected=inp.tags)


# --------- Grouped settings -------------------------------------------------
class UserSettingsIn(BaseModel):
    username: str
    email: str
    newsletter: bool = False
    theme: str = "system"


class UserSettingsOut(BaseModel):
    ok: bool
    saved: dict


@chi_command(
    input_model=UserSettingsIn,
    output_model=UserSettingsOut,
    description="Save user settings (echo).",
)
def save_settings(inp: UserSettingsIn) -> UserSettingsOut:
    return UserSettingsOut(ok=True, saved=inp.model_dump())


@chi_command(
    output_model=ItemsOut,
    description="Produce a dynamic list (slow, 5s) for testing spinners.",
)
def list_items_slow() -> ItemsOut:
    # Simulate long-running task
    time.sleep(5)
    data = [
        {"name": "Alpha", "value": 1},
        {"name": "Bravo", "value": 2},
        {"name": "Charlie", "value": 3},
        {"name": "Delta", "value": 4},
    ]
    return ItemsOut(items=data)


@chi_command(
    input_model=SimProgressIn,
    output_model=SimProgressOut,
    description="Simulate a long task with streaming progress events.",
)
def simulate_progress(inp: SimProgressIn) -> SimProgressOut:
    start = time.time()
    for i in range(inp.steps):
        pct = (i / max(1, inp.steps)) * 100.0
        emit_progress(
            message=f"Step {i+1}/{inp.steps}",
            percent=pct,
            stage="working",
            command="simulate-progress",
        )
        time.sleep(inp.delay_ms / 1000.0)
    # final 100%
    emit_progress(
        message="Finalizing",
        percent=100.0,
        stage="finalize",
        command="simulate-progress",
    )
    dur_ms = int((time.time() - start) * 1000)
    return SimProgressOut(status="done", steps=inp.steps, duration_ms=dur_ms)


# --------- Numeric form demo (for TUI auto-mapping) ------------------------
class ParamsIn(BaseModel):
    count: int = Field(5, description="Item count", ge=1, le=10)
    ratio: float = Field(0.5, description="Ratio", ge=0.0, le=1.0)


class ParamsOut(BaseModel):
    echo: dict


@chi_command(
    input_model=ParamsIn, output_model=ParamsOut, description="Echo numeric params."
)
def test_params(inp: ParamsIn) -> ParamsOut:
    # Simulate a short processing delay so the TUI can show a submit spinner
    time.sleep(1.2)
    return ParamsOut(echo={"count": inp.count, "ratio": inp.ratio})


# ---------- Nested demo: projects -> tasks -> details -----------------------
class TasksIn(BaseModel):
    project: str = Field(..., description="Project identifier")


@chi_command(
    output_model=ItemsOut, description="List demo projects (lazy children of tasks)."
)
def list_projects() -> ItemsOut:
    # Each project node is lazy and will fetch its tasks on demand
    items = []
    for pid, title in [("alpha", "Project Alpha"), ("bravo", "Project Bravo")]:
        items.append(
            {
                "id": pid,
                "title": title,
                "widget": "lazy_items",
                "command": f"example-app list-tasks --project {pid}",
                "unwrap": "data.items",
                "initial_text": "Enter to load tasks",
            }
        )
    return ItemsOut(items=items)


@chi_command(
    input_model=TasksIn, output_model=ItemsOut, description="List tasks for a project."
)
def list_tasks(inp: TasksIn) -> ItemsOut:
    # Return tasks as regular (non-lazy) items. Each has a detail command.
    tasks_map = {
        "alpha": [
            {"id": "a-1", "title": "Design API", "status": "open"},
            {"id": "a-2", "title": "Implement SDK", "status": "in_progress"},
        ],
        "bravo": [
            {"id": "b-1", "title": "Prototype UI", "status": "open"},
            {"id": "b-2", "title": "Write docs", "status": "open"},
        ],
    }
    items: List[dict] = []
    for t in tasks_map.get(inp.project, []):
        items.append(
            {
                "id": t["id"],
                "title": f"{t['title']} [{t['status']}]",
                "command": f"example-app task-detail --project {inp.project} --task {t['id']}",
            }
        )
    return ItemsOut(items=items)


class TaskDetailIn(BaseModel):
    project: str
    task: str


@chi_command(input_model=TaskDetailIn, description="Show task details (raw JSON).")
def task_detail(inp: TaskDetailIn) -> dict:
    # In a real app, fetch from DB/API. Here, return a small JSON.
    return {
        "project": inp.project,
        "task": inp.task,
        "assignee": "jane.doe",
        "priority": "normal",
        "description": f"Details for {inp.task} in project {inp.project}",
        "links": [
            {"title": "Spec", "url": "https://example.invalid/spec"},
            {"title": "Ticket", "url": "https://example.invalid/ticket"},
        ],
    }


# Removed manual 'ui' command - now provided automatically by CHI SDK's build_cli()
# The 'ui' subcommand is added by the SDK and launches the prebuilt TUI


def main():
    cli()


if __name__ == "__main__":
    main()


# --------- Text constraints demo (for TUI auto-mapping) ---------------------
class TextIn(BaseModel):
    username: str = Field(
        ...,
        description="Username",
        min_length=3,
        max_length=12,
        pattern=r"^[a-z0-9_]+$",
    )


class TextOut(BaseModel):
    ok: bool
    username: str


@chi_command(
    input_model=TextIn, output_model=TextOut, description="Validate a username."
)
def validate_text(inp: TextIn) -> TextOut:
    return TextOut(ok=True, username=inp.username)


# New commands for the improved example app


class RenderMarkdownIn(BaseModel):
    file: str = Field(..., description="Path to markdown file")


class RenderMarkdownOut(BaseModel):
    content: str
    title: str


@chi_command(
    input_model=RenderMarkdownIn,
    output_model=RenderMarkdownOut,
    description="Render markdown file content",
)
def render_markdown(inp: RenderMarkdownIn) -> RenderMarkdownOut:
    """Read and return markdown file content."""
    from pathlib import Path

    # Try to read the file
    try:
        file_path = Path(inp.file)
        if not file_path.is_absolute():
            # Try relative to current directory
            if not file_path.exists():
                # Try relative to app directory
                file_path = Path(__file__).parent / inp.file

        with open(file_path, "r") as f:
            content = f.read()

        # Extract title from first heading if present
        lines = content.split("\n")
        title = "Document"
        for line in lines:
            if line.startswith("# "):
                title = line[2:].strip()
                break

        return RenderMarkdownOut(content=content, title=title)
    except FileNotFoundError:
        return RenderMarkdownOut(content=f"File not found: {inp.file}", title="Error")
    except Exception as e:
        return RenderMarkdownOut(content=f"Error reading file: {str(e)}", title="Error")


class ShowShortcutsOut(BaseModel):
    shortcuts: dict


@chi_command(output_model=ShowShortcutsOut, description="Show keyboard shortcuts")
def show_shortcuts() -> ShowShortcutsOut:
    """Return keyboard shortcuts reference."""
    return ShowShortcutsOut(
        shortcuts={
            "Navigation": {
                "↑/↓": "Move selection",
                "←/→": "Collapse/Expand items",
                "Enter": "Select item",
                "Esc": "Go back",
                "q": "Quit application",
            },
            "Tabs": {
                "F1-F6": "Switch between tabs",
                "Tab": "Next field (in forms)",
                "Shift+Tab": "Previous field (in forms)",
            },
            "Panels": {"Tab": "Switch panel focus", "PgUp/PgDn": "Scroll content"},
            "Forms": {
                "Enter": "Submit form",
                "Esc": "Cancel form",
                "Space": "Toggle checkbox",
            },
        }
    )


class StreamLogsIn(BaseModel):
    lines: int = Field(50, description="Number of lines to stream")


class StreamLogsOut(BaseModel):
    status: str
    lines_streamed: int


@chi_command(
    input_model=StreamLogsIn,
    output_model=StreamLogsOut,
    description="Stream simulated log output",
)
def stream_logs(inp: StreamLogsIn) -> StreamLogsOut:
    """Stream simulated log lines."""
    import random
    import time
    from datetime import datetime

    log_levels = ["INFO", "DEBUG", "WARN", "ERROR"]
    messages = [
        "Processing request",
        "Connecting to database",
        "Query executed successfully",
        "Cache miss, fetching from source",
        "User authenticated",
        "Starting background job",
        "Task completed",
        "Cleaning up resources",
    ]

    for i in range(inp.lines):
        level = random.choice(log_levels)
        msg = random.choice(messages)
        timestamp = datetime.now().strftime("%H:%M:%S.%f")[:-3]

        log_line = f"[{timestamp}] [{level}] {msg}"

        # Stream progress as JSON envelope; Watchdog will show raw lines
        emit_progress(message=log_line, percent=(i / inp.lines) * 100, stage="logs")

        time.sleep(0.1)  # Simulate processing time

    return StreamLogsOut(status="completed", lines_streamed=inp.lines)


@chi_command(
    output_model=ItemsOut,
    description="List documentation with mixed widget types.",
)
def list_mixed_widgets() -> ItemsOut:
    """Demonstrate dynamic widget selection for list items."""
    return ItemsOut(
        items=[
            {
                "title": "📖 Quick Start Guide",
                "widget": "markdown",
                "path": ".tui/docs/quick_start.md",
                "id": "quick_start",
            },
            {
                "title": "⌨️ Keyboard Shortcuts",
                "widget": "markdown",
                "content": "# Keyboard Shortcuts\n\n- **↑/↓** - Navigate\n- **Enter** - Select\n- **Esc** - Back\n- **q** - Quit",
                "id": "shortcuts_inline",
            },
            {
                "title": "📊 System Monitor",
                "widget": "watchdog",
                "commands": [
                    "echo 'CPU: 42%'",
                    "echo 'Memory: 8GB/16GB'",
                    "echo 'Disk: 250GB/500GB'",
                ],
                "sequential": False,
                "id": "system_monitor",
            },
            {
                "title": "🔧 View Current Config",
                "command": "${APP_BIN} schema",
                "id": "view_config",
            },
            {
                "title": "📝 Plain JSON Data",
                "data": {"type": "info", "message": "This is raw JSON data"},
                "id": "json_data",
            },
        ]
    )


# Build the CLI group from registered commands (placed at end of file)


# -------------------- Large list with pagination (demo) --------------------
class ListLargeIn(BaseModel):
    count: int = Field(1000, description="Total number of items", ge=1, le=1000000)
    page: int = Field(1, description="Page number (1-based)", ge=1)
    per_page: int = Field(50, description="Items per page", ge=1, le=1000)


@chi_command(
    input_model=ListLargeIn,
    # no output_model so we can include both items and pagination
    description="List a large dataset with pagination metadata.",
)
def list_large(inp: ListLargeIn) -> dict:
    """Generate a deterministic large list and return the requested page.

    Returns structure:
    {
      "items": [ {"id": "item-1", "title": "Item 1"}, ... ],
      "pagination": {
        "current_page": 1,
        "total_pages": 10,
        "total_items": 1000,
        "prev_page_cmd": "${APP_BIN} list-large --count 1000 --page 0 --per-page 50",
        "next_page_cmd": "${APP_BIN} list-large --count 1000 --page 2 --per-page 50"
      }
    }
    """
    total = int(inp.count)
    per = int(inp.per_page)
    total_pages = max(1, (total + per - 1) // per)
    page = max(1, min(int(inp.page), total_pages))

    start = (page - 1) * per
    end = min(start + per, total)

    items = []
    for i in range(start, end):
        n = i + 1
        items.append({"id": f"item-{n}", "title": f"Item {n}"})

    pag = {
        "current_page": page,
        "total_pages": total_pages,
        "total_items": total,
    }
    base = f"${{APP_BIN}} list-large --count {total} --per-page {per}"
    if page > 1:
        pag["prev_page_cmd"] = f"{base} --page {page - 1}"
    if page < total_pages:
        pag["next_page_cmd"] = f"{base} --page {page + 1}"

    return {"items": items, "pagination": pag}


# Build CLI last, so all commands (including list-large) are registered
cli = build_cli("example-app")
