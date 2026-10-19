use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use mae_adapter_rust_slint::RustSlintTargetAdapter;
use mae_domain::{
    ArtifactRevision, ComponentNode, EffectBinding, InteractionBinding, LayoutConstraint,
    MigrationStrategy, MigrationTask, PropDefinition, RouteDefinition, SemanticUiModel, StateField,
    StyleToken, TargetAdapterPort, TargetPlatform, TaskId,
};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("mae_seam4_slint_{prefix}_{pid}_{id}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn sample_windows_semantic_model() -> SemanticUiModel {
    let mut model = SemanticUiModel::new(TargetPlatform::WindowsDesktop);
    model.components.push(
        ComponentNode::new("App", "App", "src/App.tsx")
            .with_child("Header")
            .with_child("TaskCard"),
    );
    model.components.push(
        ComponentNode::new("Header", "Header", "src/components/Header.tsx")
            .with_prop(PropDefinition::new("title", "string", true)),
    );
    model.components.push(
        ComponentNode::new("TaskCard", "TaskCard", "src/components/TaskCard.tsx")
            .with_prop(PropDefinition::new("label", "string", true)),
    );

    model
        .state_fields
        .push(StateField::new("completedCount", "number", "0", "useState"));
    model
        .state_fields
        .push(StateField::new("filterText", "string", "all", "useState"));

    model.interactions.push(
        InteractionBinding::new("TaskCard", "onClick", "incrementCompleted")
            .with_transition("setCompletedCount"),
    );

    model
        .effects
        .push(EffectBinding::new("loadInitialTasks", "mount", true));

    model
        .routes
        .push(RouteDefinition::new("/", "DashboardView"));
    model
        .routes
        .push(RouteDefinition::new("/settings", "SettingsView"));

    model.layout_constraints.push(
        LayoutConstraint::new("column", 12.0, 16.0)
            .with_responsive_rule("@media (min-width: 1024px)"),
    );

    model
        .style_tokens
        .push(StyleToken::new("--color-primary", "#2563eb"));
    model
        .style_tokens
        .push(StyleToken::new("--surface-bg", "#0f172a"));

    model
}

#[test]
fn scaffold_target_generates_canonical_windows_rust_slint_layout() {
    let target_dir = unique_temp_dir("scaffold");
    let model = sample_windows_semantic_model();
    let adapter = RustSlintTargetAdapter::with_crate_name("DesktopTaskTracker");

    let summary = adapter
        .scaffold_target(&model, target_dir.to_str().unwrap())
        .expect("scaffold target");

    assert_eq!(summary.target_crate_name, "desktoptasktracker");
    assert!(summary.has_windows_subsystem);
    assert!(summary.files_generated.len() >= 12);

    assert!(target_dir.join("Cargo.toml").exists());
    assert!(target_dir.join("build.rs").exists());
    assert!(target_dir.join("src/main.rs").exists());
    assert!(target_dir.join("src/app/mod.rs").exists());
    assert!(target_dir.join("src/app/state.rs").exists());
    assert!(target_dir.join("src/app/events.rs").exists());
    assert!(target_dir.join("src/app/reducer.rs").exists());
    assert!(target_dir.join("src/app/effects.rs").exists());
    assert!(target_dir.join("src/app/navigation.rs").exists());
    assert!(target_dir.join("ui/app.slint").exists());
    assert!(target_dir.join("ui/theme/tokens.slint").exists());
    assert!(target_dir.join("ui/components/header.slint").exists());
    assert!(target_dir.join("ui/components/taskcard.slint").exists());
    assert!(target_dir.join("tests/target_behavior_tests.rs").exists());

    let _ = fs::remove_dir_all(&target_dir);
}

#[test]
fn scaffold_target_enforces_windows_gui_subsystem_attribute_in_main_rs() {
    let target_dir = unique_temp_dir("win_subsys");
    let model = sample_windows_semantic_model();
    let adapter = RustSlintTargetAdapter::default();

    adapter
        .scaffold_target(&model, target_dir.to_str().unwrap())
        .expect("scaffold target");

    let main_rs = fs::read_to_string(target_dir.join("src/main.rs")).expect("read main.rs");
    assert!(
        main_rs.contains("#![cfg_attr(target_os = \"windows\", windows_subsystem = \"windows\")]"),
        "src/main.rs must include Windows GUI subsystem attribute for WindowsDesktop target"
    );

    let _ = fs::remove_dir_all(&target_dir);
}

#[test]
fn scaffold_target_enforces_logic_ui_separation_between_slint_and_rust_reducer() {
    let target_dir = unique_temp_dir("logic_ui_sep");
    let model = sample_windows_semantic_model();
    let adapter = RustSlintTargetAdapter::default();

    adapter
        .scaffold_target(&model, target_dir.to_str().unwrap())
        .expect("scaffold target");

    let app_slint = fs::read_to_string(target_dir.join("ui/app.slint")).expect("read app.slint");
    let reducer_rs =
        fs::read_to_string(target_dir.join("src/app/reducer.rs")).expect("read reducer.rs");
    let events_rs =
        fs::read_to_string(target_dir.join("src/app/events.rs")).expect("read events.rs");

    // Declarative Slint entrypoint exposes properties and callbacks
    assert!(app_slint.contains("export component MainWindow inherits Window"));
    assert!(app_slint.contains("in-out property <string> state-completedCount;"));
    assert!(app_slint.contains("callback incrementCompleted();"));

    // Business state mutation lives in pure Rust reducer & events
    assert!(events_rs.contains("IncrementCompleted"));
    assert!(reducer_rs.contains("pub fn reduce(state: &mut AppState, event: AppEvent)"));
    assert!(reducer_rs.contains("AppEvent::SubmitItem"));
    assert!(reducer_rs.contains("AppEvent::Navigate"));

    let _ = fs::remove_dir_all(&target_dir);
}

#[test]
fn emit_target_code_advances_artifact_revision_monotonically() {
    let target_dir = unique_temp_dir("revision");
    let model = sample_windows_semantic_model();
    let adapter = RustSlintTargetAdapter::default();

    adapter
        .scaffold_target(&model, target_dir.to_str().unwrap())
        .expect("scaffold");

    let task = MigrationTask::new(
        TaskId::new("task-1"),
        "Migrate TaskCard component and state",
        MigrationStrategy::DirectNativeMapping,
    );

    let rev2 = adapter
        .emit_target_code(&task, &model, target_dir.to_str().unwrap())
        .expect("emit 1");
    let rev3 = adapter
        .emit_target_code(&task, &model, target_dir.to_str().unwrap())
        .expect("emit 2");

    assert_eq!(rev2, ArtifactRevision(2));
    assert_eq!(rev3, ArtifactRevision(3));

    let _ = fs::remove_dir_all(&target_dir);
}
