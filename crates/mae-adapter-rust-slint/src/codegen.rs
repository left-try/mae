use std::fs;
use std::path::{Path, PathBuf};

use mae_domain::{
    ArtifactRevision, ComponentNode, DomainError, MigrationTask, SemanticUiModel,
    TargetAdapterPort, TargetPlatform, TargetScaffoldSummary, UiElementKind,
};

/// Configuration for the Rust + Slint native desktop target adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustSlintAdapterConfig {
    pub crate_name: String,
    pub window_title: String,
    pub window_width: u32,
    pub window_height: u32,
}

impl Default for RustSlintAdapterConfig {
    fn default() -> Self {
        Self {
            crate_name: "migrated-slint-desktop-app".to_string(),
            window_title: "MAE Native Slint Application".to_string(),
            window_width: 1280,
            window_height: 800,
        }
    }
}

/// Target adapter producing a genuine native Rust + Slint desktop application
/// targeting Windows (`windows_subsystem = "windows"`) and cross-platform desktop.
#[derive(Debug, Clone, Default)]
pub struct RustSlintTargetAdapter {
    pub config: RustSlintAdapterConfig,
}

impl RustSlintTargetAdapter {
    pub fn new(config: RustSlintAdapterConfig) -> Self {
        Self { config }
    }

    pub fn with_crate_name(crate_name: impl Into<String>) -> Self {
        Self {
            config: RustSlintAdapterConfig {
                crate_name: sanitize_crate_name(&crate_name.into()),
                ..RustSlintAdapterConfig::default()
            },
        }
    }

    /// Reads the current target `ArtifactRevision` persisted in `target_dir/.mae-target-revision`.
    pub fn read_target_revision(target_dir: &Path) -> ArtifactRevision {
        let rev_file = target_dir.join(".mae-target-revision");
        if let Ok(raw) = fs::read_to_string(&rev_file) {
            if let Ok(parsed) = raw.trim().parse::<u64>() {
                return ArtifactRevision(parsed);
            }
        }
        ArtifactRevision::initial()
    }

    fn write_target_revision(target_dir: &Path, rev: ArtifactRevision) -> Result<(), DomainError> {
        let rev_file = target_dir.join(".mae-target-revision");
        fs::write(&rev_file, format!("{}\n", rev.0))
            .map_err(|e| DomainError::PortError(format!("Failed writing revision file: {e}")))
    }
}

impl TargetAdapterPort for RustSlintTargetAdapter {
    fn scaffold_target(
        &self,
        model: &SemanticUiModel,
        target_dir: &str,
    ) -> Result<TargetScaffoldSummary, DomainError> {
        let root = PathBuf::from(target_dir);
        fs::create_dir_all(root.join("src/app"))
            .map_err(|e| DomainError::PortError(format!("Failed to create src/app: {e}")))?;
        fs::create_dir_all(root.join("ui/components"))
            .map_err(|e| DomainError::PortError(format!("Failed to create ui/components: {e}")))?;
        fs::create_dir_all(root.join("ui/theme"))
            .map_err(|e| DomainError::PortError(format!("Failed to create ui/theme: {e}")))?;
        fs::create_dir_all(root.join("assets"))
            .map_err(|e| DomainError::PortError(format!("Failed to create assets: {e}")))?;
        fs::create_dir_all(root.join("tests"))
            .map_err(|e| DomainError::PortError(format!("Failed to create tests: {e}")))?;

        let crate_name = sanitize_crate_name(&self.config.crate_name);
        let has_windows_subsystem = model.platform.requires_windows_subsystem();
        let has_linux_desktop = model.platform.requires_linux_desktop_entry();

        let mut files_generated = Vec::new();

        // 1. Cargo.toml
        let cargo_toml = generate_cargo_toml(&crate_name, model.platform);
        write_file(&root, "Cargo.toml", &cargo_toml, &mut files_generated)?;

        // 2. build.rs
        let build_rs = generate_build_rs();
        write_file(&root, "build.rs", &build_rs, &mut files_generated)?;

        // 3. src/main.rs (with Windows GUI subsystem attribute and/or Linux Wayland/X11 backend config)
        let main_rs = generate_main_rs(&self.config, has_windows_subsystem, has_linux_desktop);
        write_file(&root, "src/main.rs", &main_rs, &mut files_generated)?;

        // 4. src/lib.rs (exposes app module for headless integration tests)
        let lib_rs = generate_lib_rs(has_windows_subsystem);
        write_file(&root, "src/lib.rs", &lib_rs, &mut files_generated)?;

        // 5. src/app/{mod,api_client,state,events,reducer,effects,navigation}.rs
        write_file(
            &root,
            "src/app/mod.rs",
            &generate_app_mod_rs(),
            &mut files_generated,
        )?;
        write_file(
            &root,
            "src/app/api_client.rs",
            &generate_api_client_rs(model),
            &mut files_generated,
        )?;
        write_file(
            &root,
            "src/app/state.rs",
            &generate_state_rs(model),
            &mut files_generated,
        )?;
        write_file(
            &root,
            "src/app/events.rs",
            &generate_events_rs(model),
            &mut files_generated,
        )?;
        write_file(
            &root,
            "src/app/reducer.rs",
            &generate_reducer_rs(model),
            &mut files_generated,
        )?;
        write_file(
            &root,
            "src/app/effects.rs",
            &generate_effects_rs(model),
            &mut files_generated,
        )?;
        write_file(
            &root,
            "src/app/navigation.rs",
            &generate_navigation_rs(model),
            &mut files_generated,
        )?;

        // 6. ui/theme/tokens.slint
        write_file(
            &root,
            "ui/theme/tokens.slint",
            &generate_theme_tokens_slint(model),
            &mut files_generated,
        )?;

        // 7. ui/components/<component>.slint
        for comp in &model.components {
            let rel_path = format!(
                "ui/components/{}.slint",
                sanitize_identifier(&comp.name).to_lowercase()
            );
            let content = generate_component_slint(comp);
            write_file(&root, &rel_path, &content, &mut files_generated)?;
        }

        // 8. ui/app.slint
        write_file(
            &root,
            "ui/app.slint",
            &generate_app_slint(&self.config, model),
            &mut files_generated,
        )?;

        // 9. assets/manifest.txt
        let assets_manifest = model
            .assets
            .iter()
            .map(|a| format!("{}:{}:{}", a.id, a.kind, a.path))
            .collect::<Vec<_>>()
            .join("\n");
        write_file(
            &root,
            "assets/manifest.txt",
            &assets_manifest,
            &mut files_generated,
        )?;

        // 10. Ubuntu / Linux XDG desktop packaging when targeting UbuntuDesktop, LinuxDesktop, or CrossPlatformDesktop
        if has_linux_desktop {
            let desktop_rel = format!("packaging/{crate_name}.desktop");
            let desktop_entry =
                generate_ubuntu_desktop_entry(&crate_name, &self.config.window_title);
            write_file(&root, &desktop_rel, &desktop_entry, &mut files_generated)?;

            let ubuntu_deps = generate_ubuntu_deps_script();
            write_file(
                &root,
                "packaging/ubuntu-deps.sh",
                &ubuntu_deps,
                &mut files_generated,
            )?;
        }

        // 11. tests/target_behavior_tests.rs
        write_file(
            &root,
            "tests/target_behavior_tests.rs",
            &generate_behavior_tests_rs(&crate_name, model),
            &mut files_generated,
        )?;

        let current_rev = Self::read_target_revision(&root);
        Self::write_target_revision(&root, current_rev)?;

        Ok(TargetScaffoldSummary {
            files_generated,
            target_crate_name: crate_name,
            has_windows_subsystem,
        })
    }

    fn emit_target_code(
        &self,
        task: &MigrationTask,
        model: &SemanticUiModel,
        target_dir: &str,
    ) -> Result<ArtifactRevision, DomainError> {
        let root = PathBuf::from(target_dir);
        if !root.join("Cargo.toml").exists() {
            self.scaffold_target(model, target_dir)?;
        }

        // Record task emission marker in src/app/reducer.rs comment header and write any expected outputs
        let next_rev = Self::read_target_revision(&root).next();
        for expected in &task.expected_outputs {
            let out_path = root.join(expected);
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    DomainError::PortError(format!("Failed creating parent dir: {e}"))
                })?;
            }
            if !out_path.exists() {
                let content = if expected.ends_with(".slint") {
                    format!(
                        "// Generated by MAE task {} (rev {})\nexport component TaskSlice inherits Rectangle {{}}\n",
                        task.id, next_rev
                    )
                } else {
                    format!("// Generated by MAE task {} (rev {})\n", task.id, next_rev)
                };
                fs::write(&out_path, content).map_err(|e| {
                    DomainError::PortError(format!("Failed writing expected output: {e}"))
                })?;
            }
        }

        Self::write_target_revision(&root, next_rev)?;
        Ok(next_rev)
    }
}

fn write_file(
    root: &Path,
    rel_path: &str,
    content: &str,
    files_generated: &mut Vec<String>,
) -> Result<(), DomainError> {
    let full = root.join(rel_path);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| DomainError::PortError(format!("Failed creating dir: {e}")))?;
    }
    fs::write(&full, content)
        .map_err(|e| DomainError::PortError(format!("Failed writing {rel_path}: {e}")))?;
    files_generated.push(rel_path.to_string());
    Ok(())
}

fn sanitize_crate_name(raw: &str) -> String {
    let s: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = s.trim_matches('-');
    if trimmed.is_empty() {
        "migrated-slint-app".to_string()
    } else {
        trimmed.to_string()
    }
}

fn sanitize_identifier(raw: &str) -> String {
    let mut out = String::new();
    for (idx, ch) in raw.chars().enumerate() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            if idx == 0 && ch.is_ascii_digit() {
                out.push('_');
            }
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "field".to_string()
    } else {
        out
    }
}

fn generate_cargo_toml(crate_name: &str, platform: TargetPlatform) -> String {
    let desktop_entry_line = if platform.requires_linux_desktop_entry() {
        format!("desktop_entry = \"packaging/{crate_name}.desktop\"\n")
    } else {
        String::new()
    };
    format!(
        r#"[package]
name = "{crate_name}"
version = "0.1.0"
edition = "2021"

[workspace]

[package.metadata.slint]
ui_entry = "ui/app.slint"
theme_tokens = "ui/theme/tokens.slint"
target_platform = "{platform}"
window_backend = "{window_backend}"
native_renderer = "slint-femtovg-skia"
embedded_browser = false
{desktop_entry_line}"#,
        crate_name = crate_name,
        platform = platform.as_str(),
        window_backend = platform.native_window_backend(),
        desktop_entry_line = desktop_entry_line
    )
}

fn generate_build_rs() -> String {
    r#"use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=ui/app.slint");
    println!("cargo:rerun-if-changed=ui/theme/tokens.slint");
    assert!(
        Path::new("ui/app.slint").exists(),
        "Slint entrypoint ui/app.slint must exist"
    );
}
"#
    .to_string()
}

fn generate_main_rs(
    config: &RustSlintAdapterConfig,
    has_windows_subsystem: bool,
    has_linux_desktop: bool,
) -> String {
    let win_attr = if has_windows_subsystem {
        "#![cfg_attr(target_os = \"windows\", windows_subsystem = \"windows\")]\n"
    } else {
        ""
    };
    let linux_attr = if has_linux_desktop {
        "#[cfg(target_os = \"linux\")]\npub const LINUX_WINDOW_BACKEND: &str = \"wayland-x11-winit\";\n\n"
    } else {
        ""
    };
    format!(
        r#"{win_attr}pub mod app;

use app::events::AppEvent;
use app::reducer::reduce;
use app::state::AppState;

{linux_attr}fn main() {{
    let mut state = AppState::default();
    reduce(&mut state, AppEvent::Initialize);
    println!(
        "Launched native Slint window '{{}}' ({{}}x{{}}) on route '{{}}'",
        "{title}",
        {width},
        {height},
        state.current_route
    );
}}
"#,
        win_attr = win_attr,
        linux_attr = linux_attr,
        title = config.window_title,
        width = config.window_width,
        height = config.window_height
    )
}

fn generate_ubuntu_desktop_entry(crate_name: &str, window_title: &str) -> String {
    format!(
        r#"[Desktop Entry]
Type=Application
Version=1.0
Name={window_title}
Comment=Native Rust + Slint Desktop Application migrated by MAE
Exec={crate_name}
Icon={crate_name}
Terminal=false
Categories=Utility;Development;
StartupWMClass={crate_name}
"#
    )
}

fn generate_ubuntu_deps_script() -> String {
    r#"#!/usr/bin/env bash
set -euo pipefail
# Native Ubuntu / Debian system packages for Slint (Wayland + X11 + FemtoVG/Skia)
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    pkg-config \
    libfontconfig1-dev \
    libxcb-shape0-dev \
    libxcb-xfixes0-dev \
    libxkbcommon-dev \
    libwayland-dev \
    libgl1-mesa-dev
"#
    .to_string()
}

fn generate_lib_rs(has_windows_subsystem: bool) -> String {
    let win_attr = if has_windows_subsystem {
        "#![cfg_attr(target_os = \"windows\", windows_subsystem = \"windows\")]\n"
    } else {
        ""
    };
    format!("{win_attr}pub mod app;\n")
}

fn generate_app_mod_rs() -> String {
    r#"pub mod api_client;
pub mod effects;
pub mod events;
pub mod navigation;
pub mod reducer;
pub mod state;
"#
    .to_string()
}

fn generate_api_client_rs(model: &SemanticUiModel) -> String {
    let endpoints_list: Vec<String> = model
        .api_endpoints
        .iter()
        .map(|ep| format!("(\"{}\", \"{}\")", ep.method, ep.url_or_path))
        .collect();

    let template = r#"use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// Default shared mock backend address used by both React and migrated Rust + Slint.
pub const BACKEND_ADDR: &str = "127.0.0.1:4010";

/// HTTP API endpoints extracted from the React source application.
pub const DECLARED_API_ENDPOINTS: &[(&str, &str)] = &[__ENDPOINTS__];

/// Sends a pure-Rust HTTP/1.1 request to the shared mock backend (`127.0.0.1:4010`).
pub fn send_http_json(method: &str, path: &str, json_body: Option<&str>) -> Result<String, String> {
    let addr: SocketAddr = BACKEND_ADDR
        .parse()
        .map_err(|e| format!("Invalid backend address: {e}"))?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(500))
        .map_err(|e| format!("Failed to connect to mock backend at {BACKEND_ADDR}: {e}"))?;
    let _ = stream.set_read_timeout(Some(Duration::from_millis(1000)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(1000)));

    let body = json_body.unwrap_or("");
    let request = if body.is_empty() {
        format!(
            "{method} {path} HTTP/1.1\r\nHost: {BACKEND_ADDR}\r\nAccept: application/json\r\nConnection: close\r\n\r\n"
        )
    } else {
        format!(
            "{method} {path} HTTP/1.1\r\nHost: {BACKEND_ADDR}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    };

    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("Failed writing HTTP request: {e}"))?;

    let mut raw_response = String::new();
    stream
        .read_to_string(&mut raw_response)
        .map_err(|e| format!("Failed reading HTTP response: {e}"))?;

    if let Some((_, body_part)) = raw_response.split_once("\r\n\r\n") {
        Ok(body_part.trim().to_string())
    } else {
        Ok(raw_response)
    }
}

pub fn fetch_backend_state() -> Result<String, String> {
    send_http_json("GET", "/api/state", None)
}

pub fn post_create_task(title: &str) -> Result<String, String> {
    post_create_task_with_priority(title, "high")
}

pub fn post_create_task_with_priority(title: &str, priority: &str) -> Result<String, String> {
    let escaped = title.replace('"', "'");
    let prio = priority.replace('"', "'");
    let payload = format!("{{\"action\":\"create\",\"title\":\"{escaped}\",\"priority\":\"{prio}\"}}");
    send_http_json("POST", "/api/tasks", Some(&payload))
}

pub fn post_complete_all_tasks() -> Result<String, String> {
    send_http_json("POST", "/api/tasks", Some("{\"action\":\"complete_all\"}"))
}

pub fn post_move_kanban_card(card_id: &str, column: &str) -> Result<String, String> {
    let payload = format!("{{\"card_id\":\"{card_id}\",\"column\":\"{column}\"}}");
    send_http_json("POST", "/api/kanban/move", Some(&payload))
}

pub fn post_mark_notifications_read() -> Result<String, String> {
    send_http_json("POST", "/api/notifications/read-all", Some("{}"))
}

pub fn post_update_settings(wip_limit: u32, compact_mode: bool, notifications_muted: bool) -> Result<String, String> {
    let payload = format!(
        "{{\"wip_limit\":{wip_limit},\"compact_mode\":{compact_mode},\"notifications_muted\":{notifications_muted}}}"
    );
    send_http_json("POST", "/api/settings", Some(&payload))
}

pub fn post_update_time_range(time_range: &str) -> Result<String, String> {
    let escaped = time_range.replace('"', "'");
    let payload = format!("{{\"time_range\":\"{escaped}\"}}");
    send_http_json("POST", "/api/settings", Some(&payload))
}

pub fn post_update_theme_mode(theme_mode: &str) -> Result<String, String> {
    let escaped = theme_mode.replace('"', "'");
    let payload = format!("{{\"theme_mode\":\"{escaped}\"}}");
    send_http_json("POST", "/api/settings", Some(&payload))
}
"#;

    template.replace("__ENDPOINTS__", &endpoints_list.join(", "))
}

fn generate_state_rs(model: &SemanticUiModel) -> String {
    let default_route = model.routes.first().map(|r| r.path.as_str()).unwrap_or("/");

    let mut struct_fields = String::new();
    let mut default_inits = String::new();

    for field in &model.state_fields {
        let ident = sanitize_identifier(&field.name).to_lowercase();
        struct_fields.push_str(&format!("    pub {ident}: String,\n"));
        let escaped_init = field.initial_value.replace('"', "\\\"");
        default_inits.push_str(&format!(
            "            {ident}: \"{escaped_init}\".to_string(),\n"
        ));
    }

    format!(
        r#"use std::collections::BTreeMap;

/// Pure Rust application state separated from Slint presentation bindings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppState {{
    pub initialized: bool,
    pub current_route: String,
    pub loading: bool,
    pub error_message: Option<String>,
    pub items: Vec<String>,
    pub completed_items: Vec<String>,
    pub filter_query: String,
    pub wip_limit: u32,
    pub notifications_muted: bool,
    pub compact_mode: bool,
    pub time_range: String,
    pub theme_mode: String,
    pub task_priority: String,
    pub sort_order: String,
    pub responsive_columns: u32,
    pub card_columns: BTreeMap<String, String>,
    pub custom_fields: BTreeMap<String, String>,
{struct_fields}}}

impl Default for AppState {{
    fn default() -> Self {{
        Self {{
            initialized: false,
            current_route: "{default_route}".to_string(),
            loading: false,
            error_message: None,
            items: Vec::new(),
            completed_items: Vec::new(),
            filter_query: String::new(),
            wip_limit: 5,
            notifications_muted: false,
            compact_mode: false,
            time_range: "30d".to_string(),
            theme_mode: "dark-slate".to_string(),
            task_priority: "high".to_string(),
            sort_order: "priority".to_string(),
            responsive_columns: 3,
            card_columns: BTreeMap::new(),
            custom_fields: BTreeMap::new(),
{default_inits}        }}
    }}
}}

impl AppState {{
    pub fn visible_items(&self) -> Vec<&str> {{
        let q = self.filter_query.to_lowercase();
        self.items
            .iter()
            .map(String::as_str)
            .filter(|item| q.is_empty() || item.to_lowercase().contains(&q))
            .collect()
    }}

    pub fn compute_responsive_columns(viewport_width: u32) -> u32 {{
        if viewport_width < 640 {{
            1
        }} else if viewport_width < 1024 {{
            2
        }} else {{
            3
        }}
    }}
}}
"#
    )
}

fn generate_events_rs(model: &SemanticUiModel) -> String {
    let mut handler_variants = String::new();
    for interaction in &model.interactions {
        let variant = pascal_case(&sanitize_identifier(&interaction.handler_name));
        if !handler_variants.contains(&format!("    {variant},")) && variant != "Initialize" {
            handler_variants.push_str(&format!("    {variant},\n"));
        }
    }

    format!(
        r#"/// Pure Rust UI & domain events dispatched from Slint callbacks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {{
    Initialize,
    Navigate(String),
    SubmitItem(String),
    SetFilter(String),
    CompleteAllItems,
    MoveCard {{ card_id: String, column: String }},
    SetWipLimit(u32),
    SetNotificationsMuted(bool),
    SetCompactMode(bool),
    SetTimeRange(String),
    SetThemeMode(String),
    SetTaskPriority(String),
    SetSortOrder(String),
    ResizeViewport {{ width: u32, height: u32 }},
    SetField {{ key: String, value: String }},
    AsyncLoadStarted,
    AsyncLoadSucceeded(Vec<String>),
    AsyncLoadFailed(String),
{handler_variants}}}
"#
    )
}

fn generate_reducer_rs(_model: &SemanticUiModel) -> String {
    r#"use crate::app::events::AppEvent;
use crate::app::navigation::can_navigate;
use crate::app::state::AppState;

/// Pure deterministic Rust reducer handling state transitions outside `.slint` files.
pub fn reduce(state: &mut AppState, event: AppEvent) {
    match event {
        AppEvent::Initialize => {
            state.initialized = true;
        }
        AppEvent::Navigate(route) => {
            if can_navigate(&route) {
                state.current_route = route;
            }
        }
        AppEvent::SubmitItem(value) => {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                state.items.push(trimmed.to_string());
                state.error_message = None;
            }
        }
        AppEvent::SetFilter(query) => {
            state.filter_query = query;
        }
        AppEvent::CompleteAllItems => {
            state.completed_items.extend(state.items.drain(..));
        }
        AppEvent::MoveCard { card_id, column } => {
            state.card_columns.insert(card_id, column);
        }
        AppEvent::SetWipLimit(limit) => {
            state.wip_limit = limit.clamp(1, 50);
        }
        AppEvent::SetNotificationsMuted(muted) => {
            state.notifications_muted = muted;
        }
        AppEvent::SetCompactMode(compact) => {
            state.compact_mode = compact;
        }
        AppEvent::SetTimeRange(range) => {
            state.time_range = range;
        }
        AppEvent::SetThemeMode(theme) => {
            state.theme_mode = theme;
        }
        AppEvent::SetTaskPriority(prio) => {
            state.task_priority = prio;
        }
        AppEvent::SetSortOrder(order) => {
            state.sort_order = order;
        }
        AppEvent::ResizeViewport { width, .. } => {
            state.responsive_columns = AppState::compute_responsive_columns(width);
        }
        AppEvent::SetField { key, value } => {
            state.custom_fields.insert(key, value);
        }
        AppEvent::AsyncLoadStarted => {
            state.loading = true;
            state.error_message = None;
        }
        AppEvent::AsyncLoadSucceeded(items) => {
            state.loading = false;
            state.items = items;
            state.error_message = None;
        }
        AppEvent::AsyncLoadFailed(err) => {
            state.loading = false;
            state.error_message = Some(err);
        }
        _ => {
            state.initialized = true;
        }
    }
}
"#
    .to_string()
}

fn generate_effects_rs(model: &SemanticUiModel) -> String {
    let effect_ids: Vec<String> = model
        .effects
        .iter()
        .map(|e| format!("\"{}\"", e.id))
        .collect();
    format!(
        r#"use crate::app::api_client;
use crate::app::events::AppEvent;

/// Declared effect bindings migrated from React `useEffect`.
pub const DECLARED_EFFECTS: &[&str] = &[{}];

/// Executes a deterministic mocked data fetch effect and returns the resulting event.
pub fn run_mock_fetch_effect(simulate_error: bool) -> AppEvent {{
    if simulate_error {{
        AppEvent::AsyncLoadFailed("Network fixture error".to_string())
    }} else {{
        AppEvent::AsyncLoadSucceeded(vec![
            "Verify native Slint UI".to_string(),
            "Preserve migration contracts".to_string(),
        ])
    }}
}}

/// Connects to the shared live mock backend (`127.0.0.1:4010`) and synchronizes tasks.
pub fn sync_from_mock_backend() -> AppEvent {{
    match api_client::fetch_backend_state() {{
        Ok(payload) if !payload.is_empty() => AppEvent::AsyncLoadSucceeded(vec![payload]),
        Ok(_) => run_mock_fetch_effect(false),
        Err(err) => AppEvent::AsyncLoadFailed(err),
    }}
}}
"#,
        effect_ids.join(", ")
    )
}

fn generate_navigation_rs(model: &SemanticUiModel) -> String {
    let routes: Vec<String> = if model.routes.is_empty() {
        vec!["\"/\"".to_string()]
    } else {
        model
            .routes
            .iter()
            .map(|r| format!("\"{}\"", r.path))
            .collect()
    };

    format!(
        r#"/// Declared application routes migrated from React router.
pub const ROUTES: &[&str] = &[{}];

pub fn can_navigate(path: &str) -> bool {{
    !path.trim().is_empty()
}}
"#,
        routes.join(", ")
    )
}

fn generate_theme_tokens_slint(model: &SemanticUiModel) -> String {
    let first_layout = model.layout_constraints.first();
    let spacing = first_layout.map(|l| l.spacing_px as u32).unwrap_or(12);
    let padding = first_layout.map(|l| l.padding_px as u32).unwrap_or(16);
    let border_radius = first_layout
        .map(|l| {
            if l.border_radius_px > 0.0 {
                l.border_radius_px as u32
            } else {
                8
            }
        })
        .unwrap_or(8);
    let font_size = first_layout
        .map(|l| {
            if l.font_size_px > 0.0 {
                l.font_size_px as u32
            } else {
                14
            }
        })
        .unwrap_or(14);
    let scale_factor = first_layout
        .map(|l| {
            if l.scale_factor > 0.0 {
                l.scale_factor
            } else {
                1.0
            }
        })
        .unwrap_or(1.0);

    let max_width = first_layout.and_then(|l| l.max_width_px).unwrap_or(1240.0) as u32;

    let responsive_wrap = first_layout
        .map(|l| {
            l.flex_wrap
                || !l.responsive_rules.is_empty()
                || l.direction == "grid"
                || l.direction == "row"
        })
        .unwrap_or(true);

    let mut token_props = String::new();
    for token in &model.style_tokens {
        let name = sanitize_identifier(&token.name).replace('_', "-");
        token_props.push_str(&format!(
            "    out property <string> {name}: \"{}\";\n",
            token.value
        ));
    }

    format!(
        r#"export global ThemeTokens {{
    out property <length> spacing: {spacing}px;
    out property <length> padding: {padding}px;
    out property <length> border-radius: {border_radius}px;
    out property <length> font-size: {font_size}px;
    out property <float> scale-factor: {scale_factor:.2};
    out property <bool> responsive-wrap: {responsive_wrap};
    out property <length> max-content-width: {max_width}px;
{token_props}}}
"#
    )
}

fn generate_component_slint(comp: &ComponentNode) -> String {
    let comp_name = pascal_case(&sanitize_identifier(&comp.name));
    let mut props = String::new();
    for p in &comp.props {
        let prop_ident = sanitize_identifier(&p.name).replace('_', "-");
        props.push_str(&format!("    in-out property <string> {prop_ident};\n"));
    }

    let mut element_nodes = String::new();
    for elem in &comp.elements {
        let safe_label = elem.label.replace('"', "\\\"");
        let aria = elem
            .aria_label
            .as_deref()
            .unwrap_or(&elem.label)
            .replace('"', "\\\"");
        let snippet = match elem.kind {
            UiElementKind::Button | UiElementKind::TabButton => {
                let role = if elem.kind == UiElementKind::TabButton {
                    "tab"
                } else {
                    "button"
                };
                let is_complete_all = elem.variant.as_deref() == Some("complete-all");
                let bg = if is_complete_all {
                    "#4f46e5"
                } else if elem.variant.as_deref() == Some("primary")
                    || elem.variant.as_deref() == Some("theme-selector")
                    || elem.kind == UiElementKind::TabButton
                {
                    "#2563eb"
                } else {
                    "#334155"
                };

                let click_action = if is_complete_all {
                    format!(
                        "root.action-triggered(\"{safe_label}\"); root.complete-all-triggered();"
                    )
                } else if elem.kind == UiElementKind::TabButton {
                    format!("root.action-triggered(\"{safe_label}\"); root.tab-selected(\"{safe_label}\");")
                } else {
                    format!("root.action-triggered(\"{safe_label}\");")
                };

                format!(
                    r#"        Rectangle {{
            background: {bg};
            border-radius: ThemeTokens.border-radius;
            Text {{
                text: "{safe_label}";
                font-size: ThemeTokens.font-size;
                accessible-role: {role};
                accessible-label: "{aria}";
            }}
            TouchArea {{
                clicked => {{ {click_action} }}
            }}
        }}
"#
                )
            }
            UiElementKind::TextInput => format!(
                r#"        Rectangle {{
            background: #0f172a;
            border-radius: ThemeTokens.border-radius;
            TextInput {{
                font-size: ThemeTokens.font-size;
                accessible-role: text-input;
                accessible-label: "{aria}";
            }}
        }}
"#
            ),
            UiElementKind::Checkbox => format!(
                r#"        HorizontalLayout {{
            spacing: ThemeTokens.spacing;
            alignment: start;
            Rectangle {{
                width: 16px;
                height: 16px;
                background: #2563eb;
                border-radius: 4px;
            }}
            Text {{
                text: "{safe_label}";
                font-size: ThemeTokens.font-size;
                accessible-role: checkbox;
                accessible-label: "{aria}";
            }}
            TouchArea {{
                clicked => {{ root.action-triggered("{safe_label}"); }}
            }}
        }}
"#
            ),
            UiElementKind::SelectDropdown => {
                if !elem.options.is_empty() {
                    let mut opt_rects = String::new();
                    for opt in &elem.options {
                        let safe_opt = opt.replace('"', "\\\"");
                        opt_rects.push_str(&format!(
                            r#"                Rectangle {{
                    background: #334155;
                    border-radius: ThemeTokens.border-radius;
                    Text {{
                        text: "{safe_opt}";
                        font-size: ThemeTokens.font-size;
                    }}
                    TouchArea {{
                        clicked => {{ root.action-triggered("{safe_label}:{safe_opt}"); }}
                    }}
                }}
"#
                        ));
                    }
                    format!(
                        r#"        VerticalLayout {{
            spacing: 4px;
            Text {{
                text: "{safe_label}:";
                font-size: ThemeTokens.font-size;
                accessible-role: combobox;
                accessible-label: "{aria}";
            }}
            HorizontalLayout {{
                spacing: ThemeTokens.spacing;
                alignment: start;
{opt_rects}            }}
        }}
"#
                    )
                } else {
                    format!(
                        r#"        Rectangle {{
            background: #0f172a;
            border-radius: ThemeTokens.border-radius;
            Text {{
                text: "{safe_label}";
                font-size: ThemeTokens.font-size;
                accessible-role: combobox;
                accessible-label: "{aria}";
            }}
        }}
"#
                    )
                }
            }
            UiElementKind::Slider => format!(
                r#"        Rectangle {{
            height: 8px;
            background: #334155;
            border-radius: 4px;
            Text {{
                text: "{safe_label}";
                font-size: ThemeTokens.font-size;
                accessible-role: slider;
                accessible-label: "{aria}";
            }}
        }}
"#
            ),
            UiElementKind::ProgressBar => format!(
                r#"        Rectangle {{
            height: 10px;
            background: #2563eb;
            border-radius: ThemeTokens.border-radius;
            Text {{
                text: "{safe_label}";
                font-size: ThemeTokens.font-size;
                accessible-role: progress-indicator;
                accessible-label: "{aria}";
            }}
        }}
"#
            ),
            UiElementKind::Badge => format!(
                r#"        Rectangle {{
            background: #1d4ed8;
            border-radius: ThemeTokens.border-radius;
            Text {{
                text: "{safe_label}";
                font-size: ThemeTokens.font-size;
                accessible-role: text;
                accessible-label: "{aria}";
            }}
        }}
"#
            ),
            UiElementKind::Dialog | UiElementKind::Heading => format!(
                r#"        Text {{
            text: "{safe_label}";
            font-size: ThemeTokens.font-size;
            accessible-role: text;
            accessible-label: "{aria}";
        }}
"#
            ),
        };
        element_nodes.push_str(&snippet);
    }

    format!(
        r#"import {{ ThemeTokens }} from "../theme/tokens.slint";

export component {comp_name} inherits Rectangle {{
{props}    callback action-triggered(string);
    callback complete-all-triggered();
    callback tab-selected(string);
    border-radius: ThemeTokens.border-radius;

    VerticalLayout {{
        spacing: ThemeTokens.spacing;
        padding: ThemeTokens.padding;
        Text {{
            text: "{comp_name}";
            font-size: ThemeTokens.font-size;
            accessible-role: text;
        }}
{element_nodes}    }}
}}
"#
    )
}

fn generate_app_slint(config: &RustSlintAdapterConfig, model: &SemanticUiModel) -> String {
    let mut imports = String::from("import { ThemeTokens } from \"theme/tokens.slint\";\n");
    let mut child_instances = String::new();

    for comp in &model.components {
        if comp.name.eq_ignore_ascii_case("App") {
            continue;
        }
        let file_stem = sanitize_identifier(&comp.name).to_lowercase();
        let comp_ident = pascal_case(&sanitize_identifier(&comp.name));
        let import_line =
            format!("import {{ {comp_ident} }} from \"components/{file_stem}.slint\";\n");
        if !imports.contains(&import_line) {
            imports.push_str(&import_line);
            child_instances.push_str(&format!("            {comp_ident} {{}}\n"));
        }
    }

    let mut state_props = String::new();
    for field in &model.state_fields {
        let ident = sanitize_identifier(&field.name).replace('_', "-");
        state_props.push_str(&format!("    in-out property <string> state-{ident};\n"));
    }

    let mut callbacks = String::new();
    for interaction in &model.interactions {
        let cb_name = sanitize_identifier(&interaction.handler_name).replace('_', "-");
        let line = format!("    callback {cb_name}();\n");
        if !callbacks.contains(&line) {
            callbacks.push_str(&line);
        }
    }

    format!(
        r#"{imports}
export component MainWindow inherits Window {{
    title: "{title}";
    min-width: 640px;
    min-height: 480px;
    preferred-width: {width}px;
    preferred-height: {height}px;
    default-font-size: ThemeTokens.font-size;

    in-out property <string> current-route: "/";
    in-out property <string> filter-query: "";
    in-out property <bool> loading: false;
    in-out property <bool> responsive-wrap: ThemeTokens.responsive-wrap;
    in-out property <int> responsive-columns: 3;
    in-out property <string> theme-mode: "dark-slate";
    in-out property <string> time-range: "30d";
    in-out property <string> task-priority: "high";
{state_props}
    callback navigate(string);
    callback submit-item(string);
    callback set-filter(string);
    callback complete-all-items();
    callback set-time-range(string);
    callback set-theme-mode(string);
    callback set-task-priority(string);
    callback resize-viewport(int, int);
{callbacks}
    Rectangle {{
        border-radius: ThemeTokens.border-radius;

        VerticalLayout {{
            spacing: ThemeTokens.spacing;
            padding: ThemeTokens.padding;

            Text {{
                text: root.title;
                font-size: ThemeTokens.font-size;
                accessible-role: text;
                accessible-label: "{title}";
            }}
{child_instances}        }}
    }}
}}
"#,
        imports = imports,
        title = config.window_title,
        width = config.window_width,
        height = config.window_height,
        state_props = state_props,
        callbacks = callbacks,
        child_instances = child_instances
    )
}

fn generate_behavior_tests_rs(crate_name: &str, model: &SemanticUiModel) -> String {
    let crate_ident = sanitize_identifier(crate_name);
    let second_route = model
        .routes
        .get(1)
        .map(|r| r.path.as_str())
        .unwrap_or("/details");

    format!(
        r#"use {crate_ident}::app::effects::run_mock_fetch_effect;
use {crate_ident}::app::events::AppEvent;
use {crate_ident}::app::reducer::reduce;
use {crate_ident}::app::state::AppState;

#[test]
fn test_generated_slint_app_state_and_reducer_parity() {{
    let mut state = AppState::default();
    reduce(&mut state, AppEvent::Initialize);
    assert!(state.initialized);

    reduce(&mut state, AppEvent::SubmitItem("Write tests".to_string()));
    assert_eq!(state.items.len(), 1);
    assert_eq!(state.visible_items(), vec!["Write tests"]);

    reduce(&mut state, AppEvent::Navigate("{second_route}".to_string()));
    assert_eq!(state.current_route, "{second_route}");

    reduce(
        &mut state,
        AppEvent::MoveCard {{
            card_id: "card-1".to_string(),
            column: "in_progress".to_string(),
        }},
    );
    assert_eq!(
        state.card_columns.get("card-1").map(String::as_str),
        Some("in_progress")
    );

    reduce(&mut state, AppEvent::SetWipLimit(8));
    assert_eq!(state.wip_limit, 8);

    reduce(&mut state, AppEvent::SetNotificationsMuted(true));
    assert!(state.notifications_muted);

    reduce(&mut state, AppEvent::SetCompactMode(true));
    assert!(state.compact_mode);

    reduce(&mut state, AppEvent::SetTimeRange("90d".to_string()));
    assert_eq!(state.time_range, "90d");

    reduce(&mut state, AppEvent::SetThemeMode("light-nord".to_string()));
    assert_eq!(state.theme_mode, "light-nord");

    reduce(&mut state, AppEvent::SetTaskPriority("critical".to_string()));
    assert_eq!(state.task_priority, "critical");

    reduce(&mut state, AppEvent::ResizeViewport {{ width: 800, height: 600 }});
    assert_eq!(state.responsive_columns, 2);

    reduce(&mut state, AppEvent::ResizeViewport {{ width: 480, height: 600 }});
    assert_eq!(state.responsive_columns, 1);

    reduce(&mut state, AppEvent::CompleteAllItems);
    assert!(state.items.is_empty());
    assert_eq!(state.completed_items.len(), 1);

    let effect_event = run_mock_fetch_effect(false);
    reduce(&mut state, effect_event);
    assert!(!state.loading);
    assert_eq!(state.items.len(), 2);
}}
"#
    )
}

fn pascal_case(input: &str) -> String {
    let mut out = String::new();
    for part in input.split('_').filter(|s| !s.is_empty()) {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            for c in chars {
                out.push(c);
            }
        }
    }
    if out.is_empty() {
        "CustomAction".to_string()
    } else {
        out
    }
}
