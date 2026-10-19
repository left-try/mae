use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use mae_adapter_rust_slint::RustSlintTargetAdapter;
use mae_domain::{
    AcceptancePolicy, AcceptanceRejectionReason, AcceptanceVerdict, ArtifactRevision,
    CapabilityReport, ComponentNode, Contract, ContractId, ContractSeverity, InteractionBinding,
    MigrationStrategy, MigrationTask, RouteDefinition, SemanticUiModel, StateField,
    TargetAdapterPort, TargetPlatform, TaskId, TolerancePolicy, VerificationCategory,
    VerificationStatus, VerifierPluginPort,
};
use mae_verification::{
    BehavioralDifferentialVerifier, BuildVerifier, NativeComplianceVerifier, ScenarioTrace,
    VisualParityVerifier, VisualSnapshot,
};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("mae_seam4_verify_{prefix}_{pid}_{id}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn sample_model() -> SemanticUiModel {
    let mut model = SemanticUiModel::new(TargetPlatform::WindowsDesktop);
    model.components.push(ComponentNode::new(
        "CounterPanel",
        "CounterPanel",
        "src/CounterPanel.tsx",
    ));
    model
        .state_fields
        .push(StateField::new("count", "number", "0", "useState"));
    model.interactions.push(
        InteractionBinding::new("CounterPanel", "onClick", "increment").with_transition("setCount"),
    );
    model.routes.push(RouteDefinition::new("/", "Home"));
    model
}

#[test]
fn native_compliance_verifier_passes_genuine_rust_slint_windows_target() {
    let target_dir = unique_temp_dir("native_pass");
    let adapter = RustSlintTargetAdapter::default();
    adapter
        .scaffold_target(&sample_model(), target_dir.to_str().unwrap())
        .expect("scaffold");

    let verifier = NativeComplianceVerifier::new(&target_dir, true);
    let contract = Contract::new(
        ContractId::new("c.native"),
        "Native Compliance",
        "Target must be native Rust + Slint without WebView/Electron",
        VerificationCategory::NativeCompliance,
        ContractSeverity::Critical,
    );

    let evidence = verifier
        .verify(&contract, ArtifactRevision(1))
        .expect("verify");
    assert_eq!(evidence.status, VerificationStatus::Pass);
    assert!(evidence.coverage.is_complete());

    let _ = fs::remove_dir_all(&target_dir);
}

#[test]
fn native_compliance_verifier_rejects_webview_tauri_and_electron_wrappers() {
    for forbidden_dep in ["wry = \"0.35\"", "tauri = \"2.0\"", "webview2 = \"0.1\""] {
        let target_dir = unique_temp_dir("native_webview");
        let adapter = RustSlintTargetAdapter::default();
        adapter
            .scaffold_target(&sample_model(), target_dir.to_str().unwrap())
            .expect("scaffold");

        let cargo_path = target_dir.join("Cargo.toml");
        let mut cargo_toml = fs::read_to_string(&cargo_path).unwrap();
        cargo_toml.push_str(&format!("\n[dependencies]\n{forbidden_dep}\n"));
        fs::write(&cargo_path, cargo_toml).unwrap();

        let verifier = NativeComplianceVerifier::new(&target_dir, true);
        let contract = Contract::new(
            ContractId::new("c.native"),
            "Native Compliance",
            "Native compliance",
            VerificationCategory::NativeCompliance,
            ContractSeverity::Critical,
        );

        let evidence = verifier
            .verify(&contract, ArtifactRevision(1))
            .expect("verify");
        assert_eq!(
            evidence.status,
            VerificationStatus::Fail,
            "Must reject forbidden dependency: {forbidden_dep}"
        );
        assert!(evidence.summary.contains("Forbidden browser/wrapper"));

        let _ = fs::remove_dir_all(&target_dir);
    }
}

#[test]
fn native_compliance_verifier_rejects_screenshot_as_ui_cheat() {
    let target_dir = unique_temp_dir("native_screenshot");
    let adapter = RustSlintTargetAdapter::default();
    adapter
        .scaffold_target(&sample_model(), target_dir.to_str().unwrap())
        .expect("scaffold");

    // Overwrite ui/app.slint with a static screenshot image cheat (no interactive properties or callbacks)
    fs::write(
        target_dir.join("ui/app.slint"),
        r#"export component MainWindow inherits Window {
    Image {
        source: @image-url("baseline.png");
    }
}
"#,
    )
    .unwrap();

    let verifier = NativeComplianceVerifier::new(&target_dir, true);
    let (status, violations) = verifier.inspect_target().expect("inspect");
    assert_eq!(status, VerificationStatus::Fail);
    assert!(violations
        .iter()
        .any(|v| v.contains("Screenshot-as-UI violation")));

    let _ = fs::remove_dir_all(&target_dir);
}

#[test]
fn native_compliance_verifier_rejects_missing_windows_subsystem_attribute() {
    let target_dir = unique_temp_dir("native_missing_win_attr");
    let adapter = RustSlintTargetAdapter::default();
    adapter
        .scaffold_target(&sample_model(), target_dir.to_str().unwrap())
        .expect("scaffold");

    // Strip windows_subsystem attribute from src/main.rs
    let main_path = target_dir.join("src/main.rs");
    let main_rs = fs::read_to_string(&main_path).unwrap();
    let stripped = main_rs.replace(
        "#![cfg_attr(target_os = \"windows\", windows_subsystem = \"windows\")]",
        "",
    );
    fs::write(&main_path, stripped).unwrap();

    let verifier = NativeComplianceVerifier::new(&target_dir, true);
    let (status, violations) = verifier.inspect_target().expect("inspect");
    assert_eq!(status, VerificationStatus::Fail);
    assert!(violations
        .iter()
        .any(|v| v.contains("Missing Windows GUI subsystem attribute")));

    let _ = fs::remove_dir_all(&target_dir);
}

#[test]
fn build_verifier_executes_real_cargo_test_on_generated_target_crate() {
    let target_dir = unique_temp_dir("cargo_test_real");
    let adapter = RustSlintTargetAdapter::default();
    adapter
        .scaffold_target(&sample_model(), target_dir.to_str().unwrap())
        .expect("scaffold");

    let verifier = BuildVerifier::new(&target_dir, true);
    let contract = Contract::new(
        ContractId::new("c.build"),
        "Build Gate",
        "Generated Rust + Slint target crate compiles and passes cargo test",
        VerificationCategory::Build,
        ContractSeverity::Critical,
    );

    let evidence = verifier
        .verify(&contract, ArtifactRevision(1))
        .expect("verify build");
    assert_eq!(
        evidence.status,
        VerificationStatus::Pass,
        "Real cargo test failed: {}",
        evidence.summary
    );

    let _ = fs::remove_dir_all(&target_dir);
}

#[test]
fn build_verifier_detects_compile_error_in_target_crate() {
    let target_dir = unique_temp_dir("cargo_compile_err");
    let adapter = RustSlintTargetAdapter::default();
    adapter
        .scaffold_target(&sample_model(), target_dir.to_str().unwrap())
        .expect("scaffold");

    let main_path = target_dir.join("src/main.rs");
    let mut main_rs = fs::read_to_string(&main_path).unwrap();
    main_rs.push_str("\n// COMPILE_ERROR_MARKER\n");
    fs::write(&main_path, main_rs).unwrap();

    let verifier = BuildVerifier::new(&target_dir, false);
    let contract = Contract::new(
        ContractId::new("c.build"),
        "Build Gate",
        "Build gate",
        VerificationCategory::Build,
        ContractSeverity::Critical,
    );

    let evidence = verifier
        .verify(&contract, ArtifactRevision(2))
        .expect("verify");
    assert_eq!(evidence.status, VerificationStatus::Fail);

    let _ = fs::remove_dir_all(&target_dir);
}

#[test]
fn behavioral_verifier_passes_matching_traces_and_honors_declared_normalizations() {
    let declared = vec![
        "scenario.increment".to_string(),
        "scenario.route".to_string(),
    ];
    let source_traces = vec![
        ScenarioTrace::new("scenario.increment")
            .with_action("click:increment")
            .with_state("count", "1")
            .with_state("render_timestamp_ms", "17000001"),
        ScenarioTrace::new("scenario.route")
            .with_action("navigate:/settings")
            .with_state("active_route", "/settings"),
    ];
    let target_traces = vec![
        ScenarioTrace::new("scenario.increment")
            .with_action("click:increment")
            .with_state("count", "1")
            .with_state("render_timestamp_ms", "17000099"),
        ScenarioTrace::new("scenario.route")
            .with_action("navigate:/settings")
            .with_state("active_route", "/settings"),
    ];

    // Without normalization of `render_timestamp_ms`, traces fail
    let strict_verifier = BehavioralDifferentialVerifier::new(
        declared.clone(),
        source_traces.clone(),
        target_traces.clone(),
    );
    let contract = Contract::new(
        ContractId::new("c.behavior"),
        "Behavioral Parity",
        "Behavioral parity",
        VerificationCategory::Behavioral,
        ContractSeverity::Critical,
    );
    let strict_ev = strict_verifier
        .verify(&contract, ArtifactRevision(1))
        .unwrap();
    assert_eq!(strict_ev.status, VerificationStatus::Fail);

    // With explicit allowed normalization of `render_timestamp_ms`, traces pass
    let normalized_verifier =
        strict_verifier.with_allowed_normalization("render_timestamp_ms".to_string());
    let pass_ev = normalized_verifier
        .verify(&contract, ArtifactRevision(1))
        .unwrap();
    assert_eq!(pass_ev.status, VerificationStatus::Pass);
    assert!(pass_ev.coverage.is_complete());
}

#[test]
fn behavioral_verifier_discloses_unexecuted_scenarios_and_blocks_acceptance() {
    let declared = vec![
        "scenario.increment".to_string(),
        "scenario.unexecuted_modal".to_string(),
    ];
    let source_traces = vec![ScenarioTrace::new("scenario.increment")
        .with_action("click:increment")
        .with_state("count", "1")];
    let target_traces = vec![ScenarioTrace::new("scenario.increment")
        .with_action("click:increment")
        .with_state("count", "1")];

    let verifier = BehavioralDifferentialVerifier::new(declared, source_traces, target_traces);
    let contract = Contract::new(
        ContractId::new("c.behavior"),
        "Behavioral Parity",
        "Behavioral parity",
        VerificationCategory::Behavioral,
        ContractSeverity::Critical,
    );

    let evidence = verifier.verify(&contract, ArtifactRevision(1)).unwrap();
    assert!(!evidence.coverage.is_complete());
    assert_eq!(
        evidence.coverage.unexecuted,
        vec!["scenario.unexecuted_modal".to_string()]
    );

    // Verify AcceptancePolicy rejects incomplete scenario coverage
    let mut task = MigrationTask::new(
        TaskId::new("t1"),
        "Task with partial scenario coverage",
        MigrationStrategy::DirectNativeMapping,
    )
    .with_required_contract(ContractId::new("c.behavior"));
    task.mark_ready().unwrap();
    task.start_running().unwrap();
    task.start_verifying().unwrap();

    let verdict = AcceptancePolicy::evaluate(
        &task,
        &Default::default(),
        ArtifactRevision(1),
        &[contract],
        &CapabilityReport::new(),
        &[evidence],
    );
    assert!(matches!(
        verdict,
        AcceptanceVerdict::Rejected { reasons }
        if reasons.iter().any(|r| matches!(
            r,
            AcceptanceRejectionReason::ContractVerificationInconclusiveOrNotRun(_, _)
                | AcceptanceRejectionReason::PartialScenarioCoverage(_)
        ))
    ));
}

#[test]
fn visual_parity_verifier_enforces_geometry_and_pixel_delta_tolerances() {
    let baseline = VisualSnapshot::new("route.home", 1280, 800, 12.0, 16.0, "#2563eb", 0.0);
    let target_good = VisualSnapshot::new("route.home", 1280, 800, 12.0, 16.0, "#2563EB", 0.012);
    let target_bad = VisualSnapshot::new("route.home", 1280, 800, 12.0, 16.0, "#2563eb", 0.085);

    let contract = Contract::new(
        ContractId::new("c.visual"),
        "Visual Parity",
        "Visual layout parity within 2% delta",
        VerificationCategory::Visual,
        ContractSeverity::High,
    )
    .with_tolerance(TolerancePolicy::MaxPixelDeltaRatio(0.02));

    let pass_ev = VisualParityVerifier::new(baseline.clone(), target_good)
        .verify(&contract, ArtifactRevision(1))
        .unwrap();
    assert_eq!(pass_ev.status, VerificationStatus::Pass);

    let fail_ev = VisualParityVerifier::new(baseline, target_bad)
        .verify(&contract, ArtifactRevision(1))
        .unwrap();
    assert_eq!(fail_ev.status, VerificationStatus::Fail);
}

#[test]
fn native_compliance_verifier_validates_ubuntu_desktop_entry_and_rejects_when_missing() {
    let target_dir = unique_temp_dir("ubuntu_native");
    let mut model = sample_model();
    model.platform = TargetPlatform::UbuntuDesktop;

    let adapter = RustSlintTargetAdapter::with_crate_name("ubuntu-slint-app");
    adapter
        .scaffold_target(&model, target_dir.to_str().unwrap())
        .expect("scaffold ubuntu");

    let verifier =
        NativeComplianceVerifier::for_platform(&target_dir, TargetPlatform::UbuntuDesktop);
    let contract = Contract::new(
        ContractId::new("c.native.ubuntu"),
        "Ubuntu Native Compliance",
        "Target must include Ubuntu XDG .desktop entry and native Slint UI",
        VerificationCategory::NativeCompliance,
        ContractSeverity::Critical,
    );

    let pass_ev = verifier
        .verify(&contract, ArtifactRevision(1))
        .expect("verify ubuntu pass");
    assert_eq!(pass_ev.status, VerificationStatus::Pass);

    // Delete packaging/ubuntu-slint-app.desktop and verify it fails
    fs::remove_file(target_dir.join("packaging/ubuntu-slint-app.desktop")).unwrap();
    let fail_ev = verifier
        .verify(&contract, ArtifactRevision(2))
        .expect("verify ubuntu fail");
    assert_eq!(fail_ev.status, VerificationStatus::Fail);
    assert!(fail_ev.summary.contains("Missing Ubuntu/Linux XDG"));

    let _ = fs::remove_dir_all(&target_dir);
}
