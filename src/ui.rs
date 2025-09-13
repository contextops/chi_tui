use crate::app::{update, AppMsg, Effect};
use crate::model::{AppConfig, MenuItem};
use crate::nav::flatten::flatten_nodes;
use crate::nav::keys::menu_key;
use crate::services::cli_runner::spawn_streaming_cmd;
use crate::widgets::json_viewer::{draw_json, JsonViewerWidget};
// use crate::widgets::form::{draw_form, FormState};
use crate::widgets::menu::draw_menu;
use crate::widgets::Widget;
use anyhow::{Context, Result};
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::prelude::*;
use ratatui::widgets::*;
use std::collections::VecDeque;
#[allow(dead_code)]
pub(crate) fn compute_scroll_window_menu(
    total: usize,
    selected: usize,
    inner_h: u16,
) -> (usize, usize) {
    if inner_h == 0 || total == 0 {
        return (0, 0);
    }
    let ih = inner_h as usize;
    let sel = selected.min(total.saturating_sub(1));
    let start = if sel >= ih.saturating_sub(1) {
        sel - ih.saturating_sub(1)
    } else {
        0
    };
    let end = (start + ih).min(total);
    (start, end)
}
fn run_effects(state: &mut AppState, effects: Vec<Effect>) {
    for eff in effects {
        match eff {
            Effect::LoadMenu { mi, key } => {
                if let Some(cmd) = mi.command.clone() {
                    state.dbg(format!("load menu {key} -> {cmd}"));
                } else {
                    state.dbg(format!("load menu {key}"));
                }
                if let Some(tx) = &state.tx {
                    crate::services::loader::spawn_load_for_menu(mi, key, tx.clone());
                }
            }
            Effect::LoadChild { val, key } => {
                if let Some(cmd) = val.get("command").and_then(|s| s.as_str()) {
                    state.dbg(format!("load child {key} -> {cmd}"));
                } else {
                    state.dbg(format!("load child {key}"));
                }
                if let Some(tx) = &state.tx {
                    crate::services::loader::spawn_load_for_value(val, key, tx.clone());
                }
            }
            Effect::RunStream { cmdline, title } => {
                state.dbg(format!("run stream: {title} :: {cmdline}"));
                state.status_text = Some(format!("Running: {title}"));
                state.status_percent = None;
                // Restart animation when stream starts
                if state.animations_enabled {
                    state.animation_start_tick = state.tick;
                }
                if let Some(ptx) = &state.p_tx {
                    spawn_streaming_cmd(cmdline, ptx.clone());
                }
            }
            Effect::LoadPanelCmd { pane, cmdline } => {
                state.dbg(format!("load panel {pane:?} cmd -> {cmdline}"));
                if let Some(tx) = &state.tx {
                    let kind = match pane {
                        PanelPane::A => LoadKind::PanelA,
                        PanelPane::B => LoadKind::PanelB,
                    };
                    crate::services::loader::spawn_load_panel_cmd(cmdline, kind, tx.clone());
                }
            }
            Effect::LoadPanelYaml { pane, path } => {
                state.dbg(format!("load panel {pane:?} yaml -> {path}"));
                if let Some(tx) = &state.tx {
                    let kind = match pane {
                        PanelPane::A => LoadKind::PanelA,
                        PanelPane::B => LoadKind::PanelB,
                    };
                    crate::services::loader::spawn_load_panel_yaml(path, kind, tx.clone());
                }
            }
            Effect::CancelForm { pane } => {
                if let Some(ps) = &mut state.panel {
                    match pane {
                        PanelPane::A => {
                            ps.a.last_error = None;
                            ps.a.last_json_pretty = None;
                        }
                        PanelPane::B => {
                            ps.b.last_error = None;
                            ps.b.last_json_pretty = None;
                            let title = state
                                .pane_b_title
                                .clone()
                                .unwrap_or_else(|| "Pane B".to_string());
                            ps.b_content = PaneContent::Widget(Box::new(
                                JsonViewerWidget::from_text(title, ""),
                            ));
                        }
                    }
                }
            }
            Effect::LoadFormOptions {
                field,
                cmdline,
                unwrap,
                force,
            } => {
                state.dbg(format!(
                    "load form options field={field} cmd={cmdline} unwrap={unwrap:?} force={force}"
                ));
                if let Some(tx) = &state.tx {
                    let key = format!("form:opt:{field}");
                    // Show a short status while refreshing options
                    state.status_text = Some(format!("Refreshing options: {field}"));
                    state.status_percent = None;
                    crate::services::loader::spawn_load_options_cmd(
                        cmdline,
                        unwrap,
                        key,
                        force,
                        tx.clone(),
                    );
                }
            }
            Effect::SubmitForm { pane, cmdline } => {
                state.dbg(format!("submit form {pane:?} :: {cmdline}"));
                if let Some(tx) = &state.tx {
                    // show submitting spinner and disable form inputs
                    state.status_text = Some("Submitting...".into());
                    state.status_percent = None;
                    state.submitting = true;
                    if let Some(ps) = &mut state.panel {
                        if let PaneContent::Widget(ref mut w) = &mut ps.b_content {
                            if let Some(fw) = w
                                .as_any_mut()
                                .downcast_mut::<crate::widgets::form_widget::FormWidget>()
                            {
                                fw.form.disabled = true;
                                fw.form.editing = false;
                                fw.form.message = Some("Submitting...".into());
                            }
                        }
                    }
                    let kind = match pane {
                        PanelPane::A => LoadKind::PanelA,
                        PanelPane::B => LoadKind::SubmitForm,
                    };
                    crate::services::loader::spawn_submit_form(cmdline, kind, tx.clone());
                }
            }
            Effect::ShowToast {
                text,
                level,
                seconds,
            } => {
                let ticks = seconds.saturating_mul(5); // ~200ms tick
                let exp = state.tick.saturating_add(ticks);
                state.toast = Some(Toast {
                    text,
                    level,
                    expires_at_tick: exp,
                });
            }
        }
    }
}
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};
// (threads used by services)
// std::io, std::process helpers moved to services
#[derive(Default)]
pub(crate) struct AppState {
    pub(crate) config: AppConfig,
    pub(crate) header_h: u16,
    pub(crate) logo_lines: Vec<String>,
    pub(crate) selected: usize,
    pub(crate) view: View,
    pub(crate) children: HashMap<String, Vec<JsonValue>>,
    pub(crate) expanded: HashSet<String>,
    pub(crate) last_json_pretty: Option<String>,
    pub(crate) last_error: Option<String>,
    pub(crate) tick: u64,
    pub(crate) boot_autoload_done: bool,
    pub(crate) loading: HashSet<String>,
    tx: Option<Sender<LoadMsg>>,
    rx: Option<Receiver<LoadMsg>>,
    // JSON view state
    pub(crate) json_scroll_y: u16,
    #[allow(dead_code)]
    pub(crate) json_viewport_h: u16,
    #[allow(dead_code)]
    pub(crate) json_total_lines: u16,
    #[allow(dead_code)]
    pub(crate) json_wrap: bool,
    // Pretty JSON viewer for global (non-panel) results
    pub(crate) json_viewer: Option<crate::widgets::result_viewer::ResultViewerWidget>,
    // Left menu viewport (for PgUp/PgDn)
    pub(crate) menu_viewport_h: u16,
    // Left menu scroll offset (persistent)
    pub(crate) menu_offset: usize,
    // Streaming progress
    pub(crate) status_text: Option<String>,
    pub(crate) status_percent: Option<f64>,
    p_tx: Option<Sender<ProgressEvent>>,
    p_rx: Option<Receiver<ProgressEvent>>,
    // Panel view state
    pub(crate) panel: Option<PanelState>,
    pub(crate) panel_focus: PanelPane,
    pub(crate) panel_nested_focus: PanelPane,
    pub(crate) submitting: bool,
    pub(crate) toast: Option<Toast>,
    // Optional custom titles for panel panes (applies to generic JSON viewers)
    #[allow(dead_code)]
    pub(crate) pane_a_title: Option<String>,
    pub(crate) pane_b_title: Option<String>,
    // Stack of Pane B titles to restore on Back
    pub(crate) pane_b_title_stack: Vec<Option<String>>,
    // Theme
    pub(crate) theme: crate::theme::Theme,
    pub(crate) animations_enabled: bool,
    pub(crate) animation_start_tick: u64,
    // Horizontal menu state
    pub(crate) horizontal_tab_index: usize,
    pub(crate) current_config_path: Option<String>,
    // Debug log (rendered in bottom debug pane)
    pub(crate) debug_log: VecDeque<String>,
    // Persistent watchdog sessions keyed by menu key (menu:<id>)
    pub(crate) watchdog_sessions: HashMap<String, crate::widgets::watchdog::WatchdogSessionRef>,
}
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum View {
    #[default]
    Menu,
    Welcome,
    Json,
    Panel,
}

impl AppState {
    pub fn dbg(&mut self, msg: impl Into<String>) {
        const MAX_LOG_LINES: usize = 200;
        if self.debug_log.len() >= MAX_LOG_LINES {
            self.debug_log.pop_front();
        }
        self.debug_log.push_back(msg.into());
    }
}

// -------- Pane B helpers: history + back ----------------------------------
pub(crate) fn pane_b_replace_with_widget(
    state: &mut AppState,
    widget: Box<dyn crate::widgets::Widget>,
    push_old: bool,
) {
    if let Some(ps) = &mut state.panel {
        if push_old {
            // Move current content into history
            let old = std::mem::replace(&mut ps.b_content, PaneContent::Json);
            ps.b_history.push(old);
            state.pane_b_title_stack.push(state.pane_b_title.clone());
        }
        ps.b_content = PaneContent::Widget(widget);
    }
}

pub(crate) fn pane_b_back(state: &mut AppState) -> bool {
    if let Some(ps) = &mut state.panel {
        if let Some(prev) = ps.b_history.pop() {
            if let Some(prev_title) = state.pane_b_title_stack.pop() {
                state.pane_b_title = prev_title;
            }
            ps.b_content = prev;
            return true;
        }
    }
    false
}

#[derive(Clone, Copy)]
pub enum ToastLevel {
    Info,
    Success,
    Error,
}

pub struct Toast {
    pub text: String,
    pub level: ToastLevel,
    pub expires_at_tick: u64,
}
#[derive(Clone)]
pub(crate) enum FlatNode {
    Header {
        idx: usize,
        depth: usize,
    },
    Menu {
        idx: usize,
        depth: usize,
    },
    Child {
        key: String,
        depth: usize,
        val: JsonValue,
    },
}
// Default is derived for View
pub fn run() -> Result<()> {
    // Load config anchored by CHI_TUI_CONFIG_DIR or by discovering chi-index.yaml
    let cfg = load_config()?;
    let mut state = AppState {
        config: cfg,
        header_h: 3,
        logo_lines: Vec::new(),
        panel_focus: PanelPane::A,
        panel_nested_focus: PanelPane::A,
        theme: crate::theme::Theme::synthwave_dark(),
        animations_enabled: true,
        animation_start_tick: 0,
        horizontal_tab_index: 0,
        current_config_path: None,
        ..Default::default()
    };
    // Load logo from config (if any) and adjust header height
    init_logo_and_header(&mut state);
    let (tx, rx) = mpsc::channel::<LoadMsg>();
    state.tx = Some(tx);
    state.rx = Some(rx);
    let (p_tx, p_rx) = mpsc::channel::<ProgressEvent>();
    state.p_tx = Some(p_tx);
    state.p_rx = Some(p_rx);
    // Headless smoke mode
    let headless = std::env::var("CHI_TUI_HEADLESS")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes"))
        .unwrap_or(false);
    let headless_ticks: u64 = std::env::var("CHI_TUI_TICKS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(10);
    let headless_enter_id: Option<String> = std::env::var("CHI_TUI_HEADLESS_ENTER_ID").ok();
    let headless_summary: bool = std::env::var("CHI_TUI_SMOKE_SUMMARY")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes"))
        .unwrap_or(false);
    if headless {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend)?;
        let tick_rate = Duration::from_millis(200);
        let mut last_tick = Instant::now();
        let mut headless_enter_done = false;
        let mut progress_seen = false;
        let mut status_seen = false;
        for _ in 0..headless_ticks {
            if !state.boot_autoload_done {
                trigger_initial_autoloads(&mut state);
                state.boot_autoload_done = true;
            }
            if !headless_enter_done {
                if let Some(ref id) = headless_enter_id {
                    if let Some(mi) = state.config.menu.iter().find(|m| &m.id == id).cloned() {
                        let effs = update(&mut state, AppMsg::EnterMenu(mi));
                        run_effects(&mut state, effs);
                        headless_enter_done = true;
                    }
                }
            }
            terminal.draw(|f| ui(f, &mut state))?;
            // Pump async loader results
            let mut drained_msgs: Vec<LoadMsg> = Vec::new();
            if let Some(rx) = &state.rx {
                while let Ok(msg) = rx.try_recv() {
                    drained_msgs.push(msg);
                }
            }
            for msg in drained_msgs {
                state.loading.remove(&msg.key);
                let key = msg.key;
                let outcome = msg.outcome;
                let effects = match msg.kind {
                    LoadKind::Menu => update(&mut state, AppMsg::LoadedMenu { key, outcome }),
                    LoadKind::Child => update(&mut state, AppMsg::LoadedChild { key, outcome }),
                    LoadKind::PanelA => update(
                        &mut state,
                        AppMsg::LoadedPanel {
                            pane: PanelPane::A,
                            outcome,
                        },
                    ),
                    LoadKind::PanelB => update(
                        &mut state,
                        AppMsg::LoadedPanel {
                            pane: PanelPane::B,
                            outcome,
                        },
                    ),
                    LoadKind::PanelBNestedA => update(
                        &mut state,
                        AppMsg::LoadedNested {
                            subpane: PanelPane::A,
                            outcome,
                        },
                    ),
                    LoadKind::PanelBNestedB => update(
                        &mut state,
                        AppMsg::LoadedNested {
                            subpane: PanelPane::B,
                            outcome,
                        },
                    ),
                    LoadKind::SubmitForm => update(
                        &mut state,
                        AppMsg::LoadedSubmitForm {
                            pane: PanelPane::B,
                            outcome,
                        },
                    ),
                    LoadKind::FormOptions => {
                        update(&mut state, AppMsg::LoadedFormOptions { key, outcome })
                    }
                };
                run_effects(&mut state, effects);
            }
            // Pump streaming progress/results
            let mut drained_pev: Vec<ProgressEvent> = Vec::new();
            if let Some(prx) = &state.p_rx {
                while let Ok(ev) = prx.try_recv() {
                    drained_pev.push(ev);
                }
            }
            for ev in drained_pev {
                if !ev.done {
                    progress_seen = true;
                }
                if ev.text.is_some() {
                    status_seen = true;
                }
                let effects = if ev.done {
                    update(
                        &mut state,
                        AppMsg::StreamDone {
                            result: ev.result,
                            err: ev.err,
                        },
                    )
                } else {
                    update(
                        &mut state,
                        AppMsg::StreamProgress {
                            text: ev.text,
                            percent: ev.percent,
                        },
                    )
                };
                run_effects(&mut state, effects);
            }
            if last_tick.elapsed() >= tick_rate {
                state.tick = state.tick.wrapping_add(1);
                last_tick = Instant::now();
            }
            std::thread::sleep(std::cmp::min(tick_rate, Duration::from_millis(200)));
        }
        if headless_summary {
            let view = match state.view {
                View::Menu => "Menu",
                View::Welcome => "Welcome",
                View::Json => "Json",
                View::Panel => "Panel",
            };
            let ok = state.last_error.is_none();
            let result_present = state.last_json_pretty.is_some()
                || state
                    .panel
                    .as_ref()
                    .and_then(|ps| ps.b.last_json_pretty.as_ref())
                    .is_some();
            let summary = serde_json::json!({
                "ok": ok,
                "progress_seen": progress_seen,
                "status_seen": status_seen,
                "view": view,
                "result_present": result_present,
                "enter_done": headless_enter_done,
            });
            println!("{summary}");
        }
        return Ok(());
    }
    // Setup terminal (interactive)
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let tick_rate = Duration::from_millis(200);
    let mut last_tick = Instant::now();
    let res = loop {
        if !state.boot_autoload_done {
            trigger_initial_autoloads(&mut state);
            state.boot_autoload_done = true;
        }
        terminal.draw(|f| ui(f, &mut state))?;
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_millis(0));
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                // Check if a form in Pane B is in editing/confirm to gate global shortcuts
                let mut form_editing_b = false;
                let mut form_confirm_b = false;
                if matches!(state.view, View::Panel) && matches!(state.panel_focus, PanelPane::B) {
                    if let Some(ps) = &state.panel {
                        if let PaneContent::Widget(w) = &ps.b_content {
                            if let Some(fw) = w
                                .as_any()
                                .downcast_ref::<crate::widgets::form_widget::FormWidget>()
                            {
                                form_editing_b = fw.form.editing;
                                form_confirm_b = fw.form.confirm.is_some();
                            }
                        }
                    }
                }
                match key.code {
                    // Handle F1-F12 for horizontal menu
                    KeyCode::F(n) if (1..=12).contains(&n) => {
                        let prev_index = state.horizontal_tab_index;
                        if let Some(config_path) =
                            crate::widgets::horizontal_menu::handle_function_key(&mut state, n)
                        {
                            state.dbg(format!("load config: {config_path}"));
                            // Load the new config file
                            if let Err(e) = load_config_from_path(&mut state, &config_path) {
                                let msg = format!("Failed to load {config_path}: {e}");
                                state.dbg(&msg);
                                state.last_error = Some(msg);
                            } else {
                                state.dbg(format!("loaded config: {config_path}"));
                                // Reset menu state for new config
                                state.selected = 0;
                                state.menu_offset = 0;
                                state.expanded.clear();
                                state.children.clear();
                                state.view = View::Menu;

                                // Trigger autoloads for the new config
                                trigger_initial_autoloads(&mut state);

                                // Auto-enter a default menu item if specified by the screen config
                                if let Some(id) = state.config.auto_enter.clone() {
                                    if let Some(mi) =
                                        state.config.menu.iter().find(|m| m.id == id).cloned()
                                    {
                                        let effs = crate::app::update(
                                            &mut state,
                                            crate::app::AppMsg::EnterMenu(mi),
                                        );
                                        run_effects(&mut state, effs);
                                        // UX: when auto-opened, keep focus on left/menu (Pane A)
                                        if matches!(state.view, View::Panel) {
                                            state.panel_focus = PanelPane::A;
                                            state.panel_nested_focus = PanelPane::A;
                                        }
                                    }
                                }
                            }
                        } else {
                            // handle_function_key returned None.
                            // Two possible cases:
                            // 1) Same tab pressed again -> do nothing.
                            // 2) Switched to a tab without config (Home) -> load main config.
                            let index = (n - 1) as usize;
                            let switched = state.horizontal_tab_index != prev_index;
                            if switched && index < state.config.horizontal_menu.len() {
                                let item = &state.config.horizontal_menu[index];
                                if item.config.is_none() && state.current_config_path.is_some() {
                                    // This is a "Home" tab - reload main config
                                    state.dbg("load config: main (home)");
                                    state.config = load_config().unwrap_or_default();
                                    state.current_config_path = None;
                                    init_logo_and_header(&mut state);

                                    // Reset menu state
                                    state.selected = 0;
                                    state.menu_offset = 0;
                                    state.expanded.clear();
                                    state.children.clear();
                                    state.view = View::Menu;
                                    state.horizontal_tab_index = index;

                                    // Trigger autoloads for the main config
                                    trigger_initial_autoloads(&mut state);
                                    // No auto-enter on home by default
                                }
                            }
                        }
                    }
                    KeyCode::Char('c') => {
                        // Ctrl+C copies panel content to clipboard
                        if key.modifiers.contains(KeyModifiers::CONTROL) {
                            if state.view == View::Panel {
                                if let Some(ps) = &state.panel {
                                    let content = match state.panel_focus {
                                        PanelPane::A => {
                                            // Copy Pane A content (menu items)
                                            ps.a.last_json_pretty
                                                .clone()
                                                .or_else(|| ps.a.last_error.clone())
                                                .unwrap_or_else(|| {
                                                    // If no JSON, get current menu selection
                                                    let nodes = flatten_nodes(&state);
                                                    if let Some(node) = nodes.get(state.selected) {
                                                        match node {
                                                            FlatNode::Menu { idx, .. } => {
                                                                state.config.menu[*idx]
                                                                    .title
                                                                    .clone()
                                                            }
                                                            FlatNode::Child { val, .. } => {
                                                                title_from_value(val)
                                                            }
                                                            FlatNode::Header { .. } => {
                                                                String::new()
                                                            }
                                                        }
                                                    } else {
                                                        String::new()
                                                    }
                                                })
                                        }
                                        PanelPane::B => {
                                            // Copy Pane B content
                                            match &ps.b_content {
                                                PaneContent::Widget(w) => {
                                                    // Try to get content from widget
                                                    if let Some(md) = w.as_any().downcast_ref::<crate::widgets::markdown::MarkdownWidget>() {
                                                        md.raw_content.clone()
                                                    } else if let Some(jv) = w.as_any().downcast_ref::<crate::widgets::json_viewer::JsonViewerWidget>() {
                                                        jv.text.clone()
                                                    } else if let Some(fw) = w.as_any().downcast_ref::<crate::widgets::form_widget::FormWidget>() {
                                                        // Copy form data as text
                                                        fw.form.fields.iter()
                                                            .map(|field| format!("{}: {:?}", field.name, field.value))
                                                            .collect::<Vec<_>>()
                                                            .join("\n")
                                                    } else if let Some(wd) = w.as_any().downcast_ref::<crate::widgets::watchdog::WatchdogWidget>() {
                                                        // Copy watchdog output
                                                        wd.cmds.iter()
                                                            .map(|cmd| {
                                                                let output = cmd.output.lock().unwrap();
                                                                let lines: Vec<String> = output.iter().cloned().collect();
                                                                format!("=== {} ===\n{}", cmd.cmd, lines.join("\n"))
                                                            })
                                                            .collect::<Vec<_>>()
                                                            .join("\n\n")
                                                    } else {
                                                        String::new()
                                                    }
                                                }
                                                PaneContent::Panel(_) => {
                                                    // Nested panel - copy from last JSON
                                                    ps.b.last_json_pretty
                                                        .clone()
                                                        .or_else(|| ps.b.last_error.clone())
                                                        .unwrap_or_default()
                                                }
                                                _ => {
                                                    ps.b.last_json_pretty
                                                        .clone()
                                                        .or_else(|| ps.b.last_error.clone())
                                                        .unwrap_or_default()
                                                }
                                            }
                                        }
                                    };

                                    // Copy to clipboard
                                    if !content.is_empty() {
                                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                            let _ = clipboard.set_text(&content);
                                            state.status_text =
                                                Some("Copied to clipboard!".to_string());
                                        }
                                    }
                                }
                            } else if state.view == View::Json {
                                // Copy JSON view content or error
                                let content = state
                                    .last_json_pretty
                                    .as_ref()
                                    .or(state.last_error.as_ref())
                                    .cloned()
                                    .unwrap_or_default();

                                if !content.is_empty() {
                                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                        let _ = clipboard.set_text(&content);
                                        state.status_text =
                                            Some("Copied to clipboard!".to_string());
                                    }
                                }
                            }
                        } else {
                            // Regular 'c' key - forward to widget if in panel
                            if state.view == View::Panel
                                && matches!(state.panel_focus, PanelPane::B)
                            {
                                if let Some(ps) = &mut state.panel {
                                    if let PaneContent::Widget(ref mut w) = ps.b_content {
                                        let effs = w.on_key(KeyCode::Char('c'));
                                        run_effects(&mut state, effs);
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Char('s') => {
                        // Ctrl+S saves textarea content when editing a textarea
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && state.view == View::Panel
                            && matches!(state.panel_focus, PanelPane::B)
                        {
                            if let Some(ps) = &mut state.panel {
                                if let PaneContent::Widget(ref mut w) = ps.b_content {
                                    if let Some(fw) = w
                                        .as_any_mut()
                                        .downcast_mut::<crate::widgets::form_widget::FormWidget>(
                                    ) {
                                        let _ = fw.commit_textarea();
                                    }
                                }
                            }
                        } else {
                            // Treat as normal char; forward to widget and allow quick submit path later
                            if state.view == View::Panel
                                && matches!(state.panel_focus, PanelPane::B)
                            {
                                if let Some(ps) = &mut state.panel {
                                    if let PaneContent::Widget(ref mut w) = ps.b_content {
                                        let effs = w.on_key(KeyCode::Char('s'));
                                        run_effects(&mut state, effs);
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Char('q') => {
                        if form_editing_b {
                            // Forward to widget when editing (e.g., textarea should accept 'q')
                            if state.view == View::Panel
                                && matches!(state.panel_focus, PanelPane::B)
                            {
                                if let Some(ps) = &mut state.panel {
                                    if let PaneContent::Widget(ref mut w) = ps.b_content {
                                        let effs = w.on_key(KeyCode::Char('q'));
                                        run_effects(&mut state, effs);
                                    }
                                }
                            }
                        } else {
                            break Ok(());
                        }
                    }
                    KeyCode::Up => {
                        if state.view == View::Json {
                            if state.json_scroll_y > 0 {
                                state.json_scroll_y -= 1;
                            }
                        } else if state.view == View::Panel
                            && matches!(state.panel_focus, PanelPane::B)
                        {
                            if let Some(ps) = &mut state.panel {
                                match ps.b_content {
                                    PaneContent::Widget(ref mut w) => {
                                        let effs = w.on_key(KeyCode::Up);
                                        run_effects(&mut state, effs);
                                    }
                                    PaneContent::Panel(_) => {}
                                    _ => {}
                                }
                            }
                        } else {
                            let total_sel = flatten_nodes(&state).len();
                            if total_sel > 0 && state.selected > 0 {
                                state.selected -= 1;
                                // adjust persistent offset to keep selected in view
                                let ih = state.menu_viewport_h as usize;
                                if state.selected < state.menu_offset {
                                    state.menu_offset = state.selected;
                                } else if ih > 0 && state.selected >= state.menu_offset + ih {
                                    state.menu_offset =
                                        state.selected.saturating_sub(ih.saturating_sub(1));
                                }
                            }
                        }
                    }
                    KeyCode::Down => {
                        if state.view == View::Json {
                            if let Some(w) = &mut state.json_viewer {
                                let _ = w.on_key(KeyCode::Down);
                            }
                        } else if state.view == View::Panel
                            && matches!(state.panel_focus, PanelPane::B)
                        {
                            if let Some(ps) = &mut state.panel {
                                match ps.b_content {
                                    PaneContent::Widget(ref mut w) => {
                                        let effs = w.on_key(KeyCode::Down);
                                        run_effects(&mut state, effs);
                                    }
                                    PaneContent::Panel(_) => {}
                                    _ => {}
                                }
                            }
                        } else {
                            let total_sel = flatten_nodes(&state).len();
                            if total_sel > 0 && state.selected + 1 < total_sel {
                                state.selected += 1;
                                let ih = state.menu_viewport_h as usize;
                                if ih > 0 && state.selected >= state.menu_offset + ih {
                                    state.menu_offset =
                                        state.selected.saturating_sub(ih.saturating_sub(1));
                                }
                            }
                        }
                    }
                    KeyCode::PageUp => {
                        if state.view == View::Json {
                            if let Some(w) = &mut state.json_viewer {
                                let _ = w.on_key(KeyCode::PageUp);
                            }
                        } else if state.view == View::Panel
                            && matches!(state.panel_focus, PanelPane::B)
                        {
                            if !form_editing_b {
                                if let Some(ps) = &mut state.panel {
                                    match ps.b_content {
                                        PaneContent::Widget(ref mut w) => {
                                            let effs = w.on_key(KeyCode::PageUp);
                                            run_effects(&mut state, effs);
                                        }
                                        PaneContent::Panel(_) => {}
                                        _ => {}
                                    }
                                }
                            }
                        } else {
                            // Left menu page-up
                            let step = state.menu_viewport_h as usize;
                            if step > 0 {
                                let total = flatten_nodes(&state).len();
                                if total > 0 {
                                    state.selected = state.selected.saturating_sub(step);
                                    state.menu_offset = state.menu_offset.saturating_sub(step);
                                }
                            }
                        }
                    }
                    KeyCode::PageDown => {
                        if state.view == View::Json {
                            if let Some(w) = &mut state.json_viewer {
                                let _ = w.on_key(KeyCode::PageDown);
                            }
                        } else if state.view == View::Panel
                            && matches!(state.panel_focus, PanelPane::B)
                        {
                            if !form_editing_b {
                                if let Some(ps) = &mut state.panel {
                                    match ps.b_content {
                                        PaneContent::Widget(ref mut w) => {
                                            let effs = w.on_key(KeyCode::PageDown);
                                            run_effects(&mut state, effs);
                                        }
                                        PaneContent::Panel(_) => {}
                                        _ => {}
                                    }
                                }
                            }
                        } else {
                            // Left menu page-down
                            let step = state.menu_viewport_h as usize;
                            if step > 0 {
                                let total = flatten_nodes(&state).len();
                                if total > 0 {
                                    let max_idx = total - 1;
                                    state.selected =
                                        state.selected.saturating_add(step).min(max_idx);
                                    state.menu_offset = (state.menu_offset + step).min(max_idx);
                                }
                            }
                        }
                    }
                    KeyCode::Home => {
                        if state.view == View::Json {
                            if let Some(w) = &mut state.json_viewer {
                                let _ = w.on_key(KeyCode::Home);
                            }
                        } else if state.view == View::Panel
                            && matches!(state.panel_focus, PanelPane::B)
                            && !form_editing_b
                        {
                            if let Some(ps) = &mut state.panel {
                                if let PaneContent::Widget(ref mut w) = ps.b_content {
                                    let effs = w.on_key(KeyCode::Home);
                                    run_effects(&mut state, effs);
                                }
                            }
                        }
                    }
                    KeyCode::End => {
                        if state.view == View::Json {
                            if let Some(w) = &mut state.json_viewer {
                                let _ = w.on_key(KeyCode::End);
                            }
                        } else if state.view == View::Panel
                            && matches!(state.panel_focus, PanelPane::B)
                            && !form_editing_b
                        {
                            if let Some(ps) = &mut state.panel {
                                if let PaneContent::Widget(ref mut w) = ps.b_content {
                                    let effs = w.on_key(KeyCode::End);
                                    run_effects(&mut state, effs);
                                }
                            }
                        }
                    }
                    KeyCode::Char('w') => {
                        if state.view == View::Json {
                            if let Some(w) = &mut state.json_viewer {
                                let _ = w.on_key(KeyCode::Char('w'));
                            }
                        } else if state.view == View::Panel
                            && matches!(state.panel_focus, PanelPane::B)
                        {
                            // Always pass to widget so textareas can type 'w'
                            if let Some(ps) = &mut state.panel {
                                if let PaneContent::Widget(ref mut w) = ps.b_content {
                                    let effs = w.on_key(KeyCode::Char('w'));
                                    run_effects(&mut state, effs);
                                }
                            }
                        }
                    }
                    KeyCode::Char('j') => {
                        if state.view == View::Json {
                            if let Some(w) = &mut state.json_viewer {
                                let _ = w.on_key(KeyCode::Char('j'));
                            }
                        } else if state.view == View::Panel
                            && matches!(state.panel_focus, PanelPane::B)
                        {
                            if let Some(ps) = &mut state.panel {
                                if let PaneContent::Widget(ref mut w) = ps.b_content {
                                    let effs = w.on_key(KeyCode::Char('j'));
                                    run_effects(&mut state, effs);
                                }
                            }
                        }
                    }
                    KeyCode::Tab => {
                        if state.view == View::Panel && !form_editing_b {
                            if matches!(state.panel_focus, PanelPane::A) {
                                // A -> if nested panel exists in B, go to B.A; else go to B
                                if let Some(ps) = &mut state.panel {
                                    let has_nested_panel =
                                        matches!(ps.b_content, PaneContent::Panel(_));
                                    let is_panel_widget = if let PaneContent::Widget(ref mut w) =
                                        ps.b_content
                                    {
                                        w.as_any()
                                            .downcast_ref::<crate::widgets::panel::PanelWidget>()
                                            .is_some()
                                    } else {
                                        false
                                    };
                                    let is_watchdog_widget = if let PaneContent::Widget(ref mut w) =
                                        ps.b_content
                                    {
                                        w.as_any().downcast_ref::<crate::widgets::watchdog::WatchdogWidget>()
                                            .is_some()
                                    } else {
                                        false
                                    };
                                    state.panel_focus = PanelPane::B;
                                    if has_nested_panel {
                                        state.panel_nested_focus = PanelPane::A;
                                    } else if is_panel_widget {
                                        if let PaneContent::Widget(ref mut w) = ps.b_content {
                                            if let Some(pw) = w
                                                .as_any_mut()
                                                .downcast_mut::<crate::widgets::panel::PanelWidget>(
                                            ) {
                                                pw.set_nested_focus(PanelPane::A);
                                            }
                                        }
                                    } else if is_watchdog_widget {
                                        if let PaneContent::Widget(ref mut w) = ps.b_content {
                                            if let Some(wd) = w
                                                .as_any_mut()
                                                .downcast_mut::<crate::widgets::watchdog::WatchdogWidget>()
                                            {
                                                wd.set_focused_pane(0);
                                            }
                                        }
                                    }
                                }
                            } else if let Some(ps) = &mut state.panel {
                                // Currently focused on B; cycle B.A -> B.B -> A
                                if let PaneContent::Panel(_) = ps.b_content {
                                    if matches!(state.panel_nested_focus, PanelPane::A) {
                                        state.panel_nested_focus = PanelPane::B;
                                    } else {
                                        state.panel_focus = PanelPane::A;
                                    }
                                } else if let PaneContent::Widget(ref mut w) = ps.b_content {
                                    if let Some(pw) = w
                                        .as_any_mut()
                                        .downcast_mut::<crate::widgets::panel::PanelWidget>()
                                    {
                                        if matches!(pw.nested_focus(), PanelPane::A) {
                                            pw.set_nested_focus(PanelPane::B);
                                        } else {
                                            state.panel_focus = PanelPane::A;
                                        }
                                    } else if let Some(wd) = w
                                        .as_any_mut()
                                        .downcast_mut::<crate::widgets::watchdog::WatchdogWidget>()
                                    {
                                        let n = wd.pane_count();
                                        if n > 1 {
                                            let cur = wd.focused_pane();
                                            if cur + 1 < n {
                                                wd.set_focused_pane(cur + 1);
                                            } else {
                                                state.panel_focus = PanelPane::A;
                                            }
                                        } else {
                                            state.panel_focus = PanelPane::A;
                                        }
                                    } else {
                                        // No nested: B -> A
                                        state.panel_focus = PanelPane::A;
                                    }
                                } else {
                                    // Not a widget or nested panel: B -> A
                                    state.panel_focus = PanelPane::A;
                                }
                            }
                        }
                    }
                    KeyCode::BackTab => {
                        if state.view == View::Panel && !form_editing_b {
                            if matches!(state.panel_focus, PanelPane::A) {
                                // Reverse from A: prefer B.B (or last watchdog pane) if nested exists
                                if let Some(ps) = &mut state.panel {
                                    let has_nested_panel =
                                        matches!(ps.b_content, PaneContent::Panel(_));
                                    let is_panel_widget = if let PaneContent::Widget(ref mut w) =
                                        ps.b_content
                                    {
                                        w.as_any()
                                            .downcast_ref::<crate::widgets::panel::PanelWidget>()
                                            .is_some()
                                    } else {
                                        false
                                    };
                                    let is_watchdog_widget = if let PaneContent::Widget(ref mut w) =
                                        ps.b_content
                                    {
                                        w.as_any().downcast_ref::<crate::widgets::watchdog::WatchdogWidget>()
                                            .is_some()
                                    } else {
                                        false
                                    };
                                    state.panel_focus = PanelPane::B;
                                    if has_nested_panel {
                                        state.panel_nested_focus = PanelPane::B;
                                    } else if is_panel_widget {
                                        if let PaneContent::Widget(ref mut w) = ps.b_content {
                                            if let Some(pw) = w
                                                .as_any_mut()
                                                .downcast_mut::<crate::widgets::panel::PanelWidget>(
                                            ) {
                                                pw.set_nested_focus(PanelPane::B);
                                            }
                                        }
                                    } else if is_watchdog_widget {
                                        if let PaneContent::Widget(ref mut w) = ps.b_content {
                                            if let Some(wd) = w
                                                .as_any_mut()
                                                .downcast_mut::<crate::widgets::watchdog::WatchdogWidget>()
                                            {
                                                let n = wd.pane_count();
                                                if n > 0 { wd.set_focused_pane(n - 1); }
                                            }
                                        }
                                    }
                                }
                            } else if let Some(ps) = &mut state.panel {
                                // Reverse within B: for nested panel, B.B -> B.A -> A; for watchdog, last -> ... -> 0 -> A
                                if let PaneContent::Panel(_) = ps.b_content {
                                    if matches!(state.panel_nested_focus, PanelPane::B) {
                                        state.panel_nested_focus = PanelPane::A;
                                    } else {
                                        state.panel_focus = PanelPane::A;
                                    }
                                } else if let PaneContent::Widget(ref mut w) = ps.b_content {
                                    if let Some(pw) = w
                                        .as_any_mut()
                                        .downcast_mut::<crate::widgets::panel::PanelWidget>()
                                    {
                                        if matches!(pw.nested_focus(), PanelPane::B) {
                                            pw.set_nested_focus(PanelPane::A);
                                        } else {
                                            state.panel_focus = PanelPane::A;
                                        }
                                    } else if let Some(wd) = w
                                        .as_any_mut()
                                        .downcast_mut::<crate::widgets::watchdog::WatchdogWidget>()
                                    {
                                        let cur = wd.focused_pane();
                                        if cur > 0 {
                                            wd.set_focused_pane(cur - 1);
                                        } else {
                                            state.panel_focus = PanelPane::A;
                                        }
                                    } else {
                                        state.panel_focus = PanelPane::A;
                                    }
                                } else {
                                    state.panel_focus = PanelPane::A;
                                }
                            }
                        }
                    }
                    KeyCode::Enter => {
                        // In Panel + focus B, handle Pane B content only (do not trigger left menu)
                        if state.view == View::Panel && matches!(state.panel_focus, PanelPane::B) {
                            // Defer actions until after panel borrow ends
                            let mut action_enter_menu: Option<MenuItem> = None;
                            if let Some(ps) = &mut state.panel {
                                if let PaneContent::Widget(ref mut w) = ps.b_content {
                                    // If it's a FormWidget, delegate; if it's a MenuWidget, capture selection
                                    if let Some(_fw) = w
                                        .as_any_mut()
                                        .downcast_mut::<crate::widgets::form_widget::FormWidget>(
                                    ) {
                                        let effs = w.on_key(KeyCode::Enter);
                                        run_effects(&mut state, effs);
                                    } else if let Some(mw) =
                                        w.as_any()
                                            .downcast_ref::<crate::widgets::menu::MenuWidget>()
                                    {
                                        if let Some(mi) = mw.config.menu.get(mw.selected).cloned() {
                                            action_enter_menu = Some(mi);
                                        }
                                    } else {
                                        // generic enter to widget
                                        let effs = w.on_key(KeyCode::Enter);
                                        run_effects(&mut state, effs);
                                    }
                                }
                            }
                            // Now perform deferred actions
                            if let Some(mi) = action_enter_menu {
                                let effects = update(&mut state, AppMsg::EnterMenu(mi));
                                run_effects(&mut state, effects);
                            }
                        } else {
                            // Normal mode (or Panel focus A): trigger left/main menu item
                            let nodes = flatten_nodes(&state);
                            if let Some(node) = nodes.get(state.selected).cloned() {
                                let mut effects = Vec::new();
                                match node {
                                    FlatNode::Header { .. } => {}
                                    FlatNode::Menu { idx, .. } => {
                                        if let Some(mi) = state.config.menu.get(idx).cloned() {
                                            effects = update(&mut state, AppMsg::EnterMenu(mi));
                                        }
                                    }
                                    FlatNode::Child { key, val, .. } => {
                                        effects =
                                            update(&mut state, AppMsg::EnterChild { key, val });
                                    }
                                }
                                run_effects(&mut state, effects);
                            }
                        }
                    }
                    KeyCode::Left => {
                        if state.view == View::Panel && matches!(state.panel_focus, PanelPane::B) {
                            if let Some(ps) = &mut state.panel {
                                if let PaneContent::Widget(ref mut w) = ps.b_content {
                                    let effs = w.on_key(KeyCode::Left);
                                    run_effects(&mut state, effs);
                                }
                            }
                        }
                    }
                    KeyCode::Right => {
                        if state.view == View::Panel && matches!(state.panel_focus, PanelPane::B) {
                            if let Some(ps) = &mut state.panel {
                                if let PaneContent::Widget(ref mut w) = ps.b_content {
                                    let effs = w.on_key(KeyCode::Right);
                                    run_effects(&mut state, effs);
                                }
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        if state.view == View::Panel && matches!(state.panel_focus, PanelPane::B) {
                            if let Some(ps) = &mut state.panel {
                                if let PaneContent::Widget(ref mut w) = ps.b_content {
                                    let effs = w.on_key(KeyCode::Backspace);
                                    run_effects(&mut state, effs);
                                }
                            }
                            // If not editing/confirming a form, treat Backspace as "Back"
                            if !form_editing_b && !form_confirm_b {
                                let _ = pane_b_back(&mut state);
                            }
                        } else if matches!(state.view, View::Json) {
                            // Global JSON view: Backspace behaves like Esc (back to menu)
                            state.view = View::Menu;
                        }
                    }
                    KeyCode::Char('r') => {
                        // Always pass to widget first so textareas can type 'r'.
                        let mut handled_by_widget = false;
                        if state.view == View::Panel && matches!(state.panel_focus, PanelPane::B) {
                            if let Some(ps) = &mut state.panel {
                                if let PaneContent::Widget(ref mut w) = ps.b_content {
                                    let effs = w.on_key(KeyCode::Char('r'));
                                    handled_by_widget = !effs.is_empty();
                                    run_effects(&mut state, effs);
                                }
                            }
                        }
                        if !form_editing_b && !handled_by_widget {
                            // Fallback: refresh left menu/autoload nodes
                            let nodes = flatten_nodes(&state);
                            if let Some(node) = nodes.get(state.selected).cloned() {
                                let mut effects = Vec::new();
                                match node {
                                    FlatNode::Menu { idx, .. } => {
                                        if let Some(mi) = state.config.menu.get(idx).cloned() {
                                            effects = update(&mut state, AppMsg::RefreshMenu(mi));
                                        }
                                    }
                                    FlatNode::Child { key, val, .. } => {
                                        effects =
                                            update(&mut state, AppMsg::RefreshChild { key, val });
                                    }
                                    FlatNode::Header { .. } => {}
                                }
                                run_effects(&mut state, effects);
                            }
                        }
                    }
                    KeyCode::Esc => {
                        // Always forward to widget first (cancel textarea edits or cancel confirms)
                        let consumed = form_editing_b || form_confirm_b;
                        if let Some(ps) = &mut state.panel {
                            if let PaneContent::Widget(ref mut w) = ps.b_content {
                                let _ = w.on_key(KeyCode::Esc);
                            }
                        }
                        if !consumed {
                            // Fallback: leave Panel to Menu (unless screen locks layout)
                            if state.config.can_close {
                                state.view = View::Menu;
                            } else {
                                // Ignore ESC when can_close is false
                            }
                        }
                    }
                    KeyCode::Char(c) => {
                        // Form input/editing + submit shortcut
                        if state.view == View::Panel && matches!(state.panel_focus, PanelPane::B) {
                            let mut submit_cmd_from_char: Option<String> = None;
                            // 1) Let widget process the character
                            let mut effs_from_widget: Vec<Effect> = Vec::new();
                            if let Some(ps) = &mut state.panel {
                                if let PaneContent::Widget(ref mut w) = ps.b_content {
                                    effs_from_widget = w.on_key(KeyCode::Char(c));
                                }
                            }
                            run_effects(&mut state, effs_from_widget);
                            // 2) Handle submit shortcut when not editing
                            if c == 's' || c == 'S' {
                                if let Some(ps) = &mut state.panel {
                                    if let PaneContent::Widget(ref mut w) = ps.b_content {
                                        if let Some(fw) = w.as_any_mut().downcast_mut::<crate::widgets::form_widget::FormWidget>() {
                                            let form = &mut fw.form;
                    if !form.editing
                        && !form.disabled
                        && crate::widgets::form::validate_form(form)
                    {
                        if let Some(cmdline) = crate::widgets::form::build_cmdline(form) {
                            submit_cmd_from_char = Some(cmdline);
                        }
                    }
                                        }
                                    }
                                }
                            }
                            if let Some(cmdline) = submit_cmd_from_char {
                                let effects = vec![Effect::SubmitForm {
                                    pane: PanelPane::B,
                                    cmdline,
                                }];
                                run_effects(&mut state, effects);
                            }
                        } else {
                            // Quick numeric jump in left menu: match titles containing "[[n]]"
                            if c.is_ascii_digit() {
                                let hint = format!("[[{c}]]");
                                if let Some(menu_idx) = state
                                    .config
                                    .menu
                                    .iter()
                                    .position(|m| m.title.contains(&hint))
                                {
                                    // Find the flattened index for this top-level menu item
                                    let nodes = flatten_nodes(&state);
                                    if let Some((flat_idx, _)) = nodes.iter().enumerate().find(|(_, n)| {
                                        matches!(n, FlatNode::Menu { idx, depth } if *idx == menu_idx && *depth == 0)
                                    }) {
                                        state.selected = flat_idx;
                                        // Keep selection visible in viewport
                                        let total = nodes.len();
                                        let (start, _end) = compute_scroll_window_menu(
                                            total,
                                            state.selected,
                                            state.menu_viewport_h,
                                        );
                                        state.menu_offset = start;
                                    }
                                }
                            }
                        }
                    }
                    // removed separate space handler; handled inside Char(c) branch
                    _ => {}
                }
            }
        }
        // Pump async loader results
        let mut drained_msgs: Vec<LoadMsg> = Vec::new();
        if let Some(rx) = &state.rx {
            while let Ok(msg) = rx.try_recv() {
                drained_msgs.push(msg);
            }
        }
        for msg in drained_msgs {
            state.loading.remove(&msg.key);
            let key = msg.key;
            let outcome = msg.outcome;
            let effects = match msg.kind {
                LoadKind::Menu => update(&mut state, AppMsg::LoadedMenu { key, outcome }),
                LoadKind::Child => update(&mut state, AppMsg::LoadedChild { key, outcome }),
                LoadKind::PanelA => update(
                    &mut state,
                    AppMsg::LoadedPanel {
                        pane: PanelPane::A,
                        outcome,
                    },
                ),
                LoadKind::PanelB => update(
                    &mut state,
                    AppMsg::LoadedPanel {
                        pane: PanelPane::B,
                        outcome,
                    },
                ),
                LoadKind::PanelBNestedA => update(
                    &mut state,
                    AppMsg::LoadedNested {
                        subpane: PanelPane::A,
                        outcome,
                    },
                ),
                LoadKind::PanelBNestedB => update(
                    &mut state,
                    AppMsg::LoadedNested {
                        subpane: PanelPane::B,
                        outcome,
                    },
                ),
                LoadKind::SubmitForm => update(
                    &mut state,
                    AppMsg::LoadedSubmitForm {
                        pane: PanelPane::B,
                        outcome,
                    },
                ),
                LoadKind::FormOptions => {
                    update(&mut state, AppMsg::LoadedFormOptions { key, outcome })
                }
            };
            run_effects(&mut state, effects);
            if matches!(msg.kind, LoadKind::SubmitForm) {
                state.submitting = false;
                state.status_text = None;
                state.status_percent = None;
                if let Some(ps) = &mut state.panel {
                    if let PaneContent::Widget(ref mut w) = ps.b_content {
                        if let Some(fw) = w
                            .as_any_mut()
                            .downcast_mut::<crate::widgets::form_widget::FormWidget>()
                        {
                            fw.form.disabled = false;
                        }
                    }
                }
            }
        }
        // Pump streaming progress/results
        let mut drained_pev: Vec<ProgressEvent> = Vec::new();
        if let Some(prx) = &state.p_rx {
            while let Ok(ev) = prx.try_recv() {
                drained_pev.push(ev);
            }
        }
        for ev in drained_pev {
            let effects = if ev.done {
                update(
                    &mut state,
                    AppMsg::StreamDone {
                        result: ev.result,
                        err: ev.err,
                    },
                )
            } else {
                update(
                    &mut state,
                    AppMsg::StreamProgress {
                        text: ev.text,
                        percent: ev.percent,
                    },
                )
            };
            run_effects(&mut state, effects);
        }
        if last_tick.elapsed() >= tick_rate {
            state.tick = state.tick.wrapping_add(1);
            last_tick = Instant::now();
        }
    };
    // Restore
    disable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    res
}
fn load_config_from_path(state: &mut AppState, relative_path: &str) -> Result<()> {
    // Resolve absolute or CHI_TUI_CONFIG_DIR-relative path
    let rp = PathBuf::from(relative_path);
    let cfg_path = if rp.is_absolute() {
        rp
    } else {
        let base_dir = std::env::var("CHI_TUI_CONFIG_DIR")
            .map(PathBuf::from)
            .with_context(|| "CHI_TUI_CONFIG_DIR not set when loading relative config path")?;
        base_dir.join(relative_path)
    };

    let s =
        fs::read_to_string(&cfg_path).with_context(|| format!("reading config: {cfg_path:?}"))?;
    let new_config: AppConfig =
        serde_yaml::from_str(&s).with_context(|| format!("parsing config: {cfg_path:?}"))?;
    state.config = new_config;
    state.current_config_path = Some(relative_path.to_string());
    init_logo_and_header(state);
    Ok(())
}

fn load_config() -> Result<AppConfig> {
    // 1) If CHI_TUI_CONFIG_DIR is set, expect chi-index.yaml inside it
    if let Ok(base) = std::env::var("CHI_TUI_CONFIG_DIR") {
        let base_dir = PathBuf::from(&base);
        let entry = base_dir.join("chi-index.yaml");
        let s = fs::read_to_string(&entry).with_context(|| format!("reading {entry:?}"))?;
        // Ensure normalized for relative includes
        std::env::set_var("CHI_TUI_CONFIG_DIR", &base_dir);
        let cfg: AppConfig =
            serde_yaml::from_str(&s).with_context(|| format!("parsing {entry:?}"))?;
        return Ok(cfg);
    }

    // 2) Discover chi-index.yaml from CWD and upwards
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    // Try CWD/chi-index.yaml
    let candidates = [
        cwd.join("chi-index.yaml"),
        cwd.join(".tui").join("chi-index.yaml"),
    ];
    for p in &candidates {
        if p.exists() {
            let base_dir = p.parent().unwrap_or(&cwd).to_path_buf();
            let s = fs::read_to_string(p).with_context(|| format!("reading {p:?}"))?;
            std::env::set_var("CHI_TUI_CONFIG_DIR", &base_dir);
            let cfg: AppConfig =
                serde_yaml::from_str(&s).with_context(|| format!("parsing {p:?}"))?;
            return Ok(cfg);
        }
    }
    // Walk up ancestors looking for <ancestor>/.tui/chi-index.yaml
    let mut cur = cwd.as_path();
    while let Some(parent) = cur.parent() {
        let p = parent.join(".tui").join("chi-index.yaml");
        if p.exists() {
            let base_dir = p.parent().unwrap_or(parent).to_path_buf();
            let s = fs::read_to_string(&p).with_context(|| format!("reading {p:?}"))?;
            std::env::set_var("CHI_TUI_CONFIG_DIR", &base_dir);
            let cfg: AppConfig =
                serde_yaml::from_str(&s).with_context(|| format!("parsing {p:?}"))?;
            return Ok(cfg);
        }
        cur = parent;
    }
    // Last attempt: ~/.tui/chi-index.yaml
    if let Some(home) = std::env::var("HOME")
        .ok()
        .or_else(|| std::env::var("USERPROFILE").ok())
        .map(PathBuf::from)
    {
        let p = home.join(".tui").join("chi-index.yaml");
        if p.exists() {
            let base_dir = p.parent().unwrap_or(&home).to_path_buf();
            let s = fs::read_to_string(&p).with_context(|| format!("reading {p:?}"))?;
            std::env::set_var("CHI_TUI_CONFIG_DIR", &base_dir);
            let cfg: AppConfig =
                serde_yaml::from_str(&s).with_context(|| format!("parsing {p:?}"))?;
            return Ok(cfg);
        }
    }

    Err(anyhow::anyhow!(
        "No config found. Set CHI_TUI_CONFIG_DIR=<dir with chi-index.yaml> or place chi-index.yaml in CWD/.tui and ancestors"
    ))
}

fn init_logo_and_header(state: &mut AppState) {
    // Determine logo lines from config.logo, relative to CHI_TUI_CONFIG_DIR when needed.
    let mut lines: Vec<String> = Vec::new();
    if let Some(path) = state.config.logo.clone() {
        let pb = PathBuf::from(&path);
        let full = if pb.is_absolute() {
            pb
        } else if let Ok(dir) = std::env::var("CHI_TUI_CONFIG_DIR") {
            PathBuf::from(dir).join(&path)
        } else {
            // Try CWD as a last resort
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(&path)
        };
        if let Ok(s) = fs::read_to_string(&full) {
            lines = s.lines().map(|l| l.to_string()).collect();
        }
    }
    if lines.is_empty() {
        // Fallback: simple 3-line 'chi-tui'
        lines = vec!["".to_string(), "chi-tui".to_string(), "".to_string()];
    }
    // Reserve one extra row for the banner's bottom border so content isn't clipped.
    state.header_h = (lines.len() as u16).saturating_add(1);
    state.logo_lines = lines;
}
// run_cmdline_to_json moved to services::cli_runner
// -------- Streaming progress runner (NDJSON envelopes) ---------------------
#[derive(Debug)]
pub(crate) struct ProgressEvent {
    pub(crate) text: Option<String>,
    pub(crate) percent: Option<f64>,
    pub(crate) done: bool,
    pub(crate) result: Option<JsonValue>,
    pub(crate) err: Option<String>,
}
// spawn_streaming_cmd moved to services::cli_runner
// moved to services::loader
// get_by_path moved to services::loader
// menu_key imported from crate::nav::keys
// moved to nav::keys
// child_key moved to nav::keys
pub(crate) fn is_lazy_value(v: &JsonValue) -> bool {
    v.get("widget").and_then(|s| s.as_str()) == Some("lazy_items")
}
pub(crate) fn is_autoload_value(v: &JsonValue) -> bool {
    v.get("widget").and_then(|s| s.as_str()) == Some("autoload_items")
}
pub(crate) fn auto_expand_value(v: &JsonValue) -> bool {
    if !is_autoload_value(v) {
        return false;
    }
    !v.get("expand_on_enter")
        .and_then(|b| b.as_bool())
        .unwrap_or(false)
        && v.get("auto_expand")
            .and_then(|b| b.as_bool())
            .unwrap_or(true)
}
pub(crate) fn expand_on_enter_value(v: &JsonValue) -> bool {
    if !is_autoload_value(v) {
        return false;
    }
    v.get("expand_on_enter")
        .and_then(|b| b.as_bool())
        .unwrap_or(false)
}
pub(crate) fn initial_text_value(v: &JsonValue) -> Option<&str> {
    v.get("initial_text").and_then(|s| s.as_str())
}
pub(crate) fn title_from_value(v: &JsonValue) -> String {
    if let Some(t) = v.get("title").and_then(|s| s.as_str()) {
        return t.to_string();
    }
    if let Some(n) = v.get("name").and_then(|s| s.as_str()) {
        return n.to_string();
    }
    v.to_string().chars().take(60).collect()
}
fn ui(f: &mut Frame, state: &mut AppState) {
    // Clear expired toast
    if let Some(t) = &state.toast {
        if state.tick >= t.expires_at_tick {
            state.toast = None;
        }
    }

    // Fill entire screen with theme background
    let screen = f.area();
    let bg = Block::default().style(Style::default().bg(state.theme.bg));
    f.render_widget(bg, screen);

    // Split screen: 5% left margin, 90% content, 5% right margin
    let layout_h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(5),
            Constraint::Percentage(90),
            Constraint::Percentage(5),
        ])
        .split(screen);

    let left_side = layout_h[0];
    let content_area = layout_h[1];
    let right_side = layout_h[2];

    // Draw animated backgrounds on side strips
    // Start with a vivid animation for at least MIN ticks; extend while loading/streaming.
    const ANIMATION_MIN_TICKS: u64 = 15; // 3 seconds @ 200ms

    if state.animations_enabled {
        let elapsed_ticks = state.tick.saturating_sub(state.animation_start_tick);
        let loading_active = !state.loading.is_empty()
            || state.status_text.is_some()
            || state.status_percent.is_some();

        if elapsed_ticks < ANIMATION_MIN_TICKS || loading_active {
            // Full matrix animation during startup and while loading
            let palette = [
                state.theme.primary,
                state.theme.accent,
                state.theme.secondary,
            ];

            // Reverse palette for right side for visual balance
            let palette_r = [
                state.theme.secondary,
                state.theme.accent,
                state.theme.primary,
            ];

            crate::visuals::draw_matrix_bg_custom(f, left_side, &palette, state.tick);
            crate::visuals::draw_matrix_bg_custom(f, right_side, &palette_r, state.tick);
        } else {
            // After 3 seconds - switch to subtle ambient dots (like in banner)
            crate::visuals::draw_ambient_bg(f, left_side, &state.theme, state.tick);
            crate::visuals::draw_ambient_bg(f, right_side, &state.theme, state.tick);
        }
    }

    // Show a small loading indicator at top-left when loading/streaming
    if !state.loading.is_empty() || state.status_text.is_some() || state.status_percent.is_some() {
        // Animated spinner
        let spinner = ["⠋", "⠙", "⠸", "⠴", "⠦", "⠇"][state.tick as usize % 6];
        let msg = format!(" {spinner} loading...");
        let overlay = Rect {
            x: screen.x,
            y: screen.y,
            width: msg.len() as u16 + 1,
            height: 1,
        };
        let p = Paragraph::new(msg).style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, overlay);
    }

    // Dynamic footer: show separate Status + Help only if there is enough space
    let mut constraints = vec![Constraint::Length(state.header_h.max(1))];

    // Add space for horizontal menu (always shown)
    constraints.push(Constraint::Length(2)); // Horizontal menu height

    constraints.push(Constraint::Min(0)); // Main content
                                          // Dedicated debug pane (fixed height)
    const DEBUG_H: u16 = 4;
    constraints.push(Constraint::Length(DEBUG_H));
    constraints.push(Constraint::Length(1)); // Footer

    let dual_footer = state.status_text.is_some() && content_area.height >= 6;
    if dual_footer {
        constraints.push(Constraint::Length(1));
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(content_area);

    let mut chunk_idx = 0;

    // Header
    draw_header(f, chunks[chunk_idx], state);
    chunk_idx += 1;

    // Horizontal menu (always shown)
    crate::widgets::horizontal_menu::draw_horizontal_menu(f, chunks[chunk_idx], state);
    chunk_idx += 1;

    let main_content_chunk = chunks[chunk_idx];
    let debug_chunk = chunks[chunk_idx + 1];
    let footer_chunk = chunks[chunk_idx + 2];

    // Demo: Show loading border animation when something is loading
    if !state.loading.is_empty() && state.animations_enabled {
        crate::visuals::draw_loading_border(f, main_content_chunk, &state.theme, state.tick);
    }

    match state.view {
        View::Menu => {
            state.menu_viewport_h = main_content_chunk.height.saturating_sub(2);
            draw_menu(f, main_content_chunk, state)
        }
        View::Welcome => draw_welcome(f, main_content_chunk, state),
        View::Json => draw_json(f, main_content_chunk, state),
        View::Panel => draw_panel(f, main_content_chunk, state),
    }
    // Debug pane (bottom, fixed height)
    draw_debug(f, debug_chunk, state);
    let help_text: String = match state.view {
        View::Json => {
            "↑/↓ scroll • PgUp/PgDn • Home/End • w wrap • Backspace/Esc back • q quit".to_string()
        }
        View::Panel => String::new(), // Hints rendered inside the focused panel bar
        _ => "↑/↓ select • Enter open • r refresh • esc back • q quit".to_string(),
    };
    if dual_footer {
        draw_status(f, footer_chunk, state);
        let help = Paragraph::new(help_text.as_str()).style(Style::default().fg(Color::DarkGray));
        // Last chunk exists when dual_footer is true
        f.render_widget(help, chunks[chunk_idx + 3]);
    } else {
        draw_footer_combined(f, footer_chunk, state, help_text.as_str());
    }

    // Draw color palette bars LAST so they appear on top of everything else
    crate::visuals::draw_color_bars(f, screen, &state.theme);
}
use crate::widgets::header::draw_header;
fn draw_welcome(f: &mut Frame, area: Rect, state: &AppState) {
    let block = crate::widgets::chrome::panel_block(
        "Welcome",
        // single panel view => highlight
        !matches!(state.view, View::Panel),
    );
    let p = Paragraph::new("Welcome! This is the CHI TUI demo.\nPress esc to return to the menu.")
        .block(block);
    f.render_widget(p, area);
}
use crate::widgets::status_bar::{draw_footer_combined, draw_status};
fn draw_debug(f: &mut Frame, area: Rect, state: &AppState) {
    let b = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            "Debug",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ));
    // Take last `area.height` lines
    let h = area.height as usize;
    let mut lines: Vec<Line> = Vec::new();
    let total = state.debug_log.len();
    let start = total.saturating_sub(h);
    for s in state.debug_log.iter().skip(start) {
        lines.push(Line::raw(s.clone()));
    }
    let p = Paragraph::new(lines)
        .style(Style::default().fg(Color::Gray))
        .block(b)
        .wrap(Wrap { trim: true });
    f.render_widget(p, area);
}
pub(crate) fn is_header(mi: &MenuItem) -> bool {
    matches!(mi.widget.as_deref(), Some("header"))
}
pub(crate) fn is_lazy(mi: &MenuItem) -> bool {
    matches!(mi.widget.as_deref(), Some("lazy_items"))
}
pub(crate) fn is_autoload(mi: &MenuItem) -> bool {
    matches!(mi.widget.as_deref(), Some("autoload_items"))
}
pub(crate) fn is_panel(mi: &MenuItem) -> bool {
    matches!(mi.widget.as_deref(), Some("panel"))
}
pub(crate) fn is_markdown(mi: &MenuItem) -> bool {
    matches!(mi.widget.as_deref(), Some("markdown"))
}
pub(crate) fn is_watchdog(mi: &MenuItem) -> bool {
    matches!(mi.widget.as_deref(), Some("watchdog"))
}
pub(crate) fn auto_expand_menu(mi: &MenuItem) -> bool {
    if !is_autoload(mi) {
        return false;
    }
    !mi.expand_on_enter.unwrap_or(false) && mi.auto_expand.unwrap_or(true)
}
pub(crate) fn expand_on_enter_menu(mi: &MenuItem) -> bool {
    if !is_autoload(mi) {
        return false;
    }
    mi.expand_on_enter.unwrap_or(false)
}
// Flatten top-level menu + expanded children into a linear list
// flatten_nodes moved to crate::nav::flatten
// Async loading support
pub(crate) enum LoadOutcome {
    Items(Vec<JsonValue>),
    ItemsWithPagination {
        items: Vec<JsonValue>,
        pagination: JsonValue,
    },
    Fallback(JsonValue),
}
pub(crate) struct LoadMsg {
    pub(crate) key: String,
    pub(crate) outcome: Result<LoadOutcome, String>,
    pub(crate) kind: LoadKind,
}
#[derive(Clone, Copy)]
pub(crate) enum LoadKind {
    Menu,
    Child,
    PanelA,
    PanelB,
    #[allow(dead_code)]
    PanelBNestedA,
    #[allow(dead_code)]
    PanelBNestedB,
    SubmitForm,
    FormOptions,
}
// spawn_load_for_* moved to services::loader
fn trigger_initial_autoloads(state: &mut AppState) {
    let Some(tx) = state.tx.clone() else {
        return;
    };
    for mi in state.config.menu.clone() {
        if is_autoload(&mi) && auto_expand_menu(&mi) {
            let key = menu_key(&mi);
            if !state.children.contains_key(&key) && !state.loading.contains(&key) {
                state.loading.insert(key.clone());
                state.expanded.insert(key.clone());
                crate::services::loader::spawn_load_for_menu(mi, key, tx.clone());
            }
        }
    }
}

// ---------------- Panel support (first pass) -------------------------------
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PanelPane {
    #[default]
    A,
    B,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PanelLayout {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PanelRatio {
    Half,       // 50/50
    OneToThree, // 25/75
    ThreeToOne, // 75/25
    OneToTwo,   // ~33/67
    TwoToOne,   // ~67/33
    TwoToThree, // 40/60
    ThreeToTwo, // 60/40
}

#[derive(Default, Clone)]
pub(crate) struct PaneData {
    pub last_json_pretty: Option<String>,
    pub last_error: Option<String>,
}

// Default is derived on PaneData

pub(crate) struct PanelState {
    pub layout: PanelLayout,
    pub ratio: PanelRatio,
    pub a: PaneData,
    pub b: PaneData,
    // Pane B content mode
    pub b_content: PaneContent,
    // History of Pane B content for Back navigation
    pub b_history: Vec<PaneContent>,
}

impl Default for PanelState {
    fn default() -> Self {
        Self {
            layout: PanelLayout::Horizontal,
            ratio: PanelRatio::Half,
            a: PaneData::default(),
            b: PaneData::default(),
            b_content: PaneContent::Widget(Box::new(JsonViewerWidget::from_text("Pane B", ""))),
            b_history: Vec::new(),
        }
    }
}
#[allow(dead_code)]
pub(crate) enum PaneContent {
    Json,
    Menu {
        config: crate::model::AppConfig,
        selected: usize,
    },
    Panel(Box<PanelState>),
    Widget(Box<dyn crate::widgets::Widget>),
}

pub(crate) fn parse_panel_layout(s: Option<&str>) -> PanelLayout {
    match s.unwrap_or("horizontal").to_ascii_lowercase().as_str() {
        "vertical" => PanelLayout::Vertical,
        _ => PanelLayout::Horizontal,
    }
}

pub(crate) fn parse_panel_ratio(s: Option<&str>) -> PanelRatio {
    match s.unwrap_or("1:1") {
        "1:3" => PanelRatio::OneToThree,
        "3:1" => PanelRatio::ThreeToOne,
        "1:2" => PanelRatio::OneToTwo,
        "2:1" => PanelRatio::TwoToOne,
        "2:3" => PanelRatio::TwoToThree,
        "3:2" => PanelRatio::ThreeToTwo,
        _ => PanelRatio::Half,
    }
}

fn draw_panel(f: &mut Frame, area: Rect, state: &mut AppState) {
    let Some(ps_ref) = state.panel.as_ref() else {
        let p = Paragraph::new("Panel not initialized")
            .block(Block::default().borders(Borders::ALL).title("Panel"));
        f.render_widget(p, area);
        return;
    };
    let constraints = match ps_ref.ratio {
        PanelRatio::Half => [Constraint::Percentage(50), Constraint::Percentage(50)],
        PanelRatio::OneToThree => [Constraint::Percentage(25), Constraint::Percentage(75)],
        PanelRatio::ThreeToOne => [Constraint::Percentage(75), Constraint::Percentage(25)],
        PanelRatio::OneToTwo => [Constraint::Percentage(33), Constraint::Percentage(67)],
        PanelRatio::TwoToOne => [Constraint::Percentage(67), Constraint::Percentage(33)],
        PanelRatio::TwoToThree => [Constraint::Percentage(40), Constraint::Percentage(60)],
        PanelRatio::ThreeToTwo => [Constraint::Percentage(60), Constraint::Percentage(40)],
    };
    let chunks = if matches!(ps_ref.layout, PanelLayout::Horizontal) {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area)
    };

    // Compute help text for focused pane (rendered as an inner bottom bar)
    let help = panel_help_text(state);
    let focus_on_a = matches!(state.view, View::Panel) && matches!(state.panel_focus, PanelPane::A);

    // Prepare areas for A and B; reserve one line for help in the focused pane
    let mut area_a = chunks[0];
    let mut area_b = chunks[1];
    let mut help_area = None;
    if focus_on_a {
        if area_a.height > 2 {
            help_area = Some(Rect {
                x: area_a.x,
                y: area_a.y + area_a.height.saturating_sub(1),
                width: area_a.width,
                height: 1,
            });
            area_a.height = area_a.height.saturating_sub(1);
        }
    } else if area_b.height > 2 {
        help_area = Some(Rect {
            x: area_b.x,
            y: area_b.y + area_b.height.saturating_sub(1),
            width: area_b.width,
            height: 1,
        });
        area_b.height = area_b.height.saturating_sub(1);
    }

    // Left/Top pane (A): render the main menu directly (no extra wrapper)
    draw_menu(f, area_a, state);

    // Right/Bottom pane (B)
    match &ps_ref.b_content {
        PaneContent::Panel(nested) => {
            // Draw nested panel inside Pane B area (highlight nested focus)
            draw_nested_panel(f, chunks[1], nested, state.panel_nested_focus);
        }
        PaneContent::Widget(_) => {
            if let Some(ps_mut) = state.panel.as_mut() {
                if let PaneContent::Widget(ref mut w) = ps_mut.b_content {
                    let area_b = area_b;
                    let highlight = matches!(state.view, View::Panel)
                        && matches!(state.panel_focus, PanelPane::B);
                    w.render(f, area_b, highlight, state.tick);
                }
            }
        }
        PaneContent::Json => {}
        PaneContent::Menu { .. } => {}
    }

    // Draw help text inside the focused panel's bottom bar
    if let Some(hrect) = help_area {
        let theme = &state.theme;
        let line = Line::from(vec![
            Span::styled(
                "keys: ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(help, Style::default().fg(Color::DarkGray)),
        ]);
        let p = Paragraph::new(line);
        f.render_widget(p, hrect);
    }
}

fn panel_help_text(state: &AppState) -> String {
    // Default when no panel
    let default = "↑/↓ select • Enter open • r refresh • esc back • q quit".to_string();
    let Some(ps) = &state.panel else {
        return default;
    };
    // Focused pane decides hints. If B and hosts a form, tailor hints; else use generic.
    let focus_b = matches!(state.panel_focus, PanelPane::B);
    if focus_b {
        if let PaneContent::Widget(w) = &ps.b_content {
            if let Some(fw) = w
                .as_any()
                .downcast_ref::<crate::widgets::form_widget::FormWidget>()
            {
                let form = &fw.form;
                if form.editing {
                    if let Some(fld) = form.fields.get(form.selected) {
                        return match fld.kind {
                            crate::widgets::form::FieldKind::Select { .. } =>
                                "↑/↓ move • Enter select • ←/→ commit • esc exit edit • s submit • q quit".to_string(),
                            crate::widgets::form::FieldKind::MultiSelect { .. } =>
                                "↑/↓ move • Space/Enter toggle • esc exit edit • s submit • q quit".to_string(),
                            crate::widgets::form::FieldKind::TextArea { .. } =>
                                "Type • Enter newline • esc finish • s submit • q quit".to_string(),
                            _ => "↑/↓ move • Enter finish • esc exit edit • s submit • q quit".to_string(),
                        };
                    } else {
                        return "↑/↓ move • Enter • esc exit edit • s submit • q quit".to_string();
                    }
                } else if let Some(fld) = form.fields.get(form.selected) {
                    let refresh_hint = if fld.dyn_options_cmd.is_some() {
                        " • r refresh"
                    } else {
                        ""
                    };
                    return match fld.kind {
                        crate::widgets::form::FieldKind::Select { .. } =>
                            format!("↑/↓ select field • Enter edit • ←/→ change{refresh_hint} • s submit • esc back • q quit"),
                        crate::widgets::form::FieldKind::TextArea { .. } =>
                            format!("↑/↓ select field • Enter edit • esc back • q quit{refresh_hint}"),
                        _ => format!("↑/↓ select field • Enter edit{refresh_hint} • s submit • esc back • q quit"),
                    };
                } else {
                    return "↑/↓ select • Enter edit • s submit • esc back • q quit".to_string();
                }
            }
            // Watchdog-specific hints
            if w.as_any()
                .downcast_ref::<crate::widgets::watchdog::WatchdogWidget>()
                .is_some()
            {
                return "Tab next pane • Shift+Tab prev • ↑/↓/PgUp/PgDn/Home/End scroll (all panes) • f/End follow • s start/stop • r restart • esc back • q quit".to_string();
            }
        }
    }
    default
}

#[cfg(test)]
mod tests {
    use super::compute_scroll_window_menu;

    #[test]
    fn pane_b_menu_scroll_window_keeps_selected_visible() {
        // total 12, height 4 → window size 4
        assert_eq!(compute_scroll_window_menu(12, 0, 4), (0, 4));
        assert_eq!(compute_scroll_window_menu(12, 3, 4), (0, 4));
        assert_eq!(compute_scroll_window_menu(12, 4, 4), (1, 5));
        assert_eq!(compute_scroll_window_menu(12, 11, 4), (8, 12));
    }
}

#[cfg(test)]
mod registry_bridge_tests {
    use crate::app::Effect;
    use crate::chi_core::registry::resolve_widget_effect;
    use crate::ui::PanelPane;
    use serde_json::json;

    #[test]
    fn registry_routes_json_viewer_specs() {
        let v = json!({"type":"json_viewer","cmd":"example-app list-items"});
        match resolve_widget_effect(PanelPane::A, &v) {
            Some(Effect::LoadPanelCmd { pane, cmdline }) => {
                assert!(matches!(pane, PanelPane::A));
                assert_eq!(cmdline, "example-app list-items");
            }
            _ => panic!("expected LoadPanelCmd"),
        }
    }
}

#[cfg(test)]
mod focus_tests {
    use super::*;

    #[test]
    fn tab_toggles_panel_focus_when_no_nested() {
        let mut st = AppState {
            panel_focus: PanelPane::A,
            panel_nested_focus: PanelPane::A,
            ..Default::default()
        };
        // Simulate Tab handling using FocusState logic
        let mut f = crate::chi_core::focus::FocusState::new(st.panel_focus, st.panel_nested_focus);
        f.toggle_panel();
        st.panel_focus = f.panel_focus;
        assert!(matches!(st.panel_focus, PanelPane::B));
    }

    #[test]
    fn tab_toggles_nested_focus_when_nested_present() {
        let mut st = AppState {
            panel_focus: PanelPane::B,
            panel_nested_focus: PanelPane::A,
            ..Default::default()
        };
        // In nested scenario, toggling nested focus flips A<->B
        let mut f = crate::chi_core::focus::FocusState::new(st.panel_focus, st.panel_nested_focus);
        f.toggle_nested();
        st.panel_nested_focus = f.nested_focus;
        assert!(matches!(st.panel_nested_focus, PanelPane::B));
    }
}

fn draw_nested_panel(f: &mut Frame, area: Rect, ps: &PanelState, nested_focus: PanelPane) {
    let constraints = match ps.ratio {
        PanelRatio::Half => [Constraint::Percentage(50), Constraint::Percentage(50)],
        PanelRatio::OneToThree => [Constraint::Percentage(25), Constraint::Percentage(75)],
        PanelRatio::ThreeToOne => [Constraint::Percentage(75), Constraint::Percentage(25)],
        PanelRatio::OneToTwo => [Constraint::Percentage(33), Constraint::Percentage(67)],
        PanelRatio::TwoToOne => [Constraint::Percentage(67), Constraint::Percentage(33)],
        PanelRatio::TwoToThree => [Constraint::Percentage(40), Constraint::Percentage(60)],
        PanelRatio::ThreeToTwo => [Constraint::Percentage(60), Constraint::Percentage(40)],
    };
    let chunks = if matches!(ps.layout, PanelLayout::Horizontal) {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area)
    };
    // Pane A
    let mut lines_a: Vec<Line> = Vec::new();
    if let Some(err) = &ps.a.last_error {
        lines_a.push(Line::from(err.clone()).style(Style::default().fg(Color::Red)));
        lines_a.push(Line::from(""));
    }
    if let Some(txt) = &ps.a.last_json_pretty {
        for l in txt.lines() {
            lines_a.push(Line::from(l.to_string()));
        }
    }
    let block_a =
        crate::widgets::chrome::panel_block("Pane B.A", matches!(nested_focus, PanelPane::A));
    let pa = Paragraph::new(lines_a).block(block_a);
    f.render_widget(pa, chunks[0]);
    // Pane B
    let mut lines_b: Vec<Line> = Vec::new();
    if let Some(err) = &ps.b.last_error {
        lines_b.push(Line::from(err.clone()).style(Style::default().fg(Color::Red)));
        lines_b.push(Line::from(""));
    }
    if let Some(txt) = &ps.b.last_json_pretty {
        for l in txt.lines() {
            lines_b.push(Line::from(l.to_string()));
        }
    }
    let block_b =
        crate::widgets::chrome::panel_block("Pane B.B", matches!(nested_focus, PanelPane::B));
    let pb = Paragraph::new(lines_b).block(block_b);
    f.render_widget(pb, chunks[1]);
}
// Default is derived on PanelPane
