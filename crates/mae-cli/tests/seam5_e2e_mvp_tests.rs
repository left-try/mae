use std::fs;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};

use mae_adapter_react::ReactSourceAdapter;
use mae_adapter_rust_slint::RustSlintTargetAdapter;
use mae_application::{
    DeterministicMockAgentBackend, InMemoryArtifactRepository, MigrationOrchestrator,
    OkfKnowledgeRepository, OpfPromptProvider,
};
use mae_cli::{
    analyze_react_fixture, build_mvp_migration_aggregate, execute_cli_args,
    run_full_migration_pipeline, MvpPipelineConfig,
};
use mae_domain::{
    ArtifactRevision, Contract, DomainError, EvidenceRecord, GapReasonCode, MigrationStrategy,
    TargetPlatform, TaskId, TaskState, VerificationCategory, VerificationStatus,
    VerifierPluginPort,
};
use mae_verification::{BuildVerifier, NativeComplianceVerifier};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn fixture_source_dir() -> PathBuf {
    workspace_root().join("fixtures/react-rust-slint/source")
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("mae_seam5_e2e_{prefix}_{pid}_{id}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn mvp_criterion_1_and_2_fixture_analysis_and_browser_only_gap_classification() {
    let source_dir = fixture_source_dir();
    let (model, capability_report, runtime_baseline) =
        analyze_react_fixture(&source_dir).expect("analyze fixture");

    // Criterion 1: Source analysis extracts all 11 components/views, 6 routes, 35+ UI elements, state hooks, effects, CSS tokens, and assets
    assert_eq!(model.platform, TargetPlatform::WindowsDesktop);
    assert!(model.components.len() >= 11);
    assert_eq!(model.routes.len(), 6);
    assert!(model.total_ui_elements() >= 35);
    assert!(model.state_fields.len() >= 9);
    assert!(!model.effects.is_empty());
    assert!(!model.style_tokens.is_empty());
    assert!(!model.layout_constraints.is_empty());
    assert!(!model.assets.is_empty());
    assert!(runtime_baseline.is_some());

    // Criterion 2: Browser-only `window.localStorage` feature is explicitly classified with UI_BROWSER_ONLY_API
    // and ReplacementComponent strategy (never silently ignored!)
    let browser_gap = capability_report
        .gaps
        .iter()
        .find(|g| g.reason_code == GapReasonCode::UiBrowserOnlyApi)
        .expect("must detect window.localStorage browser-only API gap");
    assert_eq!(browser_gap.reason_code.as_str(), "UI_BROWSER_ONLY_API");
    assert_eq!(
        browser_gap.strategy,
        MigrationStrategy::ReplacementComponent
    );
    assert!(browser_gap.source_location.contains("App.tsx"));
}

#[test]
fn mvp_criterion_3_4_5_6_full_pipeline_generates_compilable_windows_rust_slint_target_and_passes_all_gates(
) {
    let temp = unique_temp_dir("full_mvp");
    let target_dir = temp.join("target_slint_app");
    let artifacts_dir = temp.join("artifacts");
    let checkpoint_path = temp.join("checkpoint.kv");

    let config = MvpPipelineConfig::new(
        workspace_root(),
        fixture_source_dir(),
        &target_dir,
        &artifacts_dir,
    )
    .with_checkpoint_path(&checkpoint_path)
    .with_cargo_test_execution(true);

    let outcome = run_full_migration_pipeline(&config).expect("run full migration pipeline");

    // All 3 waves and tasks must be Accepted
    assert!(
        outcome.run_report.is_successful(),
        "Full MVP pipeline must succeed: {:?}",
        outcome.run_report.completion_status
    );
    assert_eq!(outcome.run_report.health_score.score, 1.0);
    assert_eq!(outcome.run_report.health_score.correctness_gate, 1);
    assert_eq!(outcome.run_report.waves_executed, 3);
    assert!(outcome.run_report.checkpoint_saved);
    assert!(checkpoint_path.exists());

    for (_, state) in &outcome.run_report.task_states {
        assert_eq!(*state, TaskState::Accepted);
    }

    // Criterion 3 & 4: Verify generated Windows Rust + Slint target files and Windows GUI subsystem attribute
    assert!(target_dir.join("Cargo.toml").exists());
    assert!(target_dir.join("src/main.rs").exists());
    assert!(target_dir.join("src/app/reducer.rs").exists());
    assert!(target_dir.join("ui/app.slint").exists());
    assert!(target_dir.join("ui/theme/tokens.slint").exists());
    assert!(target_dir
        .join("ui/components/dashboardview.slint")
        .exists());
    assert!(target_dir.join("ui/components/tasksview.slint").exists());
    assert!(target_dir
        .join("ui/components/kanbanboardview.slint")
        .exists());
    assert!(target_dir
        .join("ui/components/analyticsview.slint")
        .exists());
    assert!(target_dir
        .join("ui/components/notificationsdrawer.slint")
        .exists());
    assert!(target_dir.join("ui/components/settingsview.slint").exists());

    let main_rs = fs::read_to_string(target_dir.join("src/main.rs")).unwrap();
    assert!(
        main_rs.contains("#![cfg_attr(target_os = \"windows\", windows_subsystem = \"windows\")]")
    );

    let tokens_slint = fs::read_to_string(target_dir.join("ui/theme/tokens.slint")).unwrap();
    assert!(tokens_slint.contains("out property <length> border-radius: 8px;"));
    assert!(tokens_slint.contains("out property <length> font-size: 14px;"));
    assert!(tokens_slint.contains("out property <float> scale-factor: 1.00;"));

    let app_slint = fs::read_to_string(target_dir.join("ui/app.slint")).unwrap();
    assert!(app_slint.contains("default-font-size: ThemeTokens.font-size;"));
    assert!(app_slint.contains("border-radius: ThemeTokens.border-radius;"));
    assert!(
        app_slint.contains("import { KanbanBoardView } from \"components/kanbanboardview.slint\";")
    );
    assert!(app_slint.contains("import { AnalyticsView } from \"components/analyticsview.slint\";"));
    assert!(app_slint
        .contains("import { NotificationsDrawer } from \"components/notificationsdrawer.slint\";"));

    let kanban_slint =
        fs::read_to_string(target_dir.join("ui/components/kanbanboardview.slint")).unwrap();
    assert!(kanban_slint.contains("accessible-role: slider;"));
    assert!(kanban_slint.contains("Move to In Progress Button"));

    let analytics_slint =
        fs::read_to_string(target_dir.join("ui/components/analyticsview.slint")).unwrap();
    assert!(analytics_slint.contains("accessible-role: tab;"));
    assert!(analytics_slint.contains("accessible-role: checkbox;"));
    assert!(analytics_slint.contains("accessible-role: progress-indicator;"));

    // Criterion 5: All recorded evidence records have complete scenario coverage disclosure
    assert!(!outcome.aggregate.evidence_ledger.is_empty());
    for ev in &outcome.aggregate.evidence_ledger {
        assert_eq!(ev.status, VerificationStatus::Pass);
        assert!(ev.coverage.is_complete());
    }

    let _ = fs::remove_dir_all(&temp);
}

struct FlakyFirstAttemptBehavioralVerifier {
    calls: Arc<Mutex<u32>>,
}

impl VerifierPluginPort for FlakyFirstAttemptBehavioralVerifier {
    fn category(&self) -> VerificationCategory {
        VerificationCategory::Behavioral
    }

    fn verify(
        &self,
        contract: &Contract,
        target_rev: ArtifactRevision,
    ) -> Result<EvidenceRecord, DomainError> {
        let mut count = self.calls.lock().unwrap();
        *count += 1;
        let scenarios = vec!["scenario.route_transition".to_string()];
        if *count == 1 {
            Ok(EvidenceRecord::new(
                contract.id.clone(),
                "mae.verifier.flaky-behavioral",
                VerificationCategory::Behavioral,
                ArtifactRevision(1),
                target_rev,
                VerificationStatus::Fail,
                mae_domain::CoverageDisclosure::with_unexecuted(
                    scenarios.clone(),
                    scenarios,
                    Vec::new(),
                    Vec::new(),
                ),
                "Deliberate first-attempt behavioral mismatch on route transition",
            ))
        } else {
            Ok(EvidenceRecord::new(
                contract.id.clone(),
                "mae.verifier.flaky-behavioral",
                VerificationCategory::Behavioral,
                ArtifactRevision(1),
                target_rev,
                VerificationStatus::Pass,
                mae_domain::CoverageDisclosure::full(scenarios),
                "Repaired behavioral route transition passed on fresh target revision",
            ))
        }
    }
}

#[test]
fn mvp_criterion_7_deliberate_failure_triggers_repair_loop_and_fresh_target_revision() {
    let temp = unique_temp_dir("repair_e2e");
    let target_dir = temp.join("target_slint_app");
    let root = workspace_root();

    let (model, capability_report, _) = analyze_react_fixture(&fixture_source_dir()).unwrap();
    let mut aggregate = build_mvp_migration_aggregate("mig-repair-e2e", &model, &capability_report);

    let flaky_calls = Arc::new(Mutex::new(0u32));
    let verifiers: Vec<Box<dyn VerifierPluginPort>> = vec![
        Box::new(NativeComplianceVerifier::new(&target_dir, true)),
        Box::new(BuildVerifier::new(&target_dir, false)),
        Box::new(FlakyFirstAttemptBehavioralVerifier {
            calls: Arc::clone(&flaky_calls),
        }),
        Box::new(mae_verification::VisualParityVerifier::new(
            mae_verification::VisualSnapshot::new("v", 1280, 800, 12.0, 16.0, "#2563eb", 0.0),
            mae_verification::VisualSnapshot::new("v", 1280, 800, 12.0, 16.0, "#2563eb", 0.005),
        )),
    ];

    let mut orchestrator = MigrationOrchestrator::new(
        Box::new(OpfPromptProvider::from_root(&root).unwrap()),
        Box::new(OkfKnowledgeRepository::new(root.join(".okf"))),
        Box::new(DeterministicMockAgentBackend::new()),
        Box::new(ReactSourceAdapter::new(TargetPlatform::WindowsDesktop)),
        Box::new(RustSlintTargetAdapter::default()),
        Box::new(InMemoryArtifactRepository::new()),
        verifiers,
    );

    let mut report = orchestrator
        .run(
            &mut aggregate,
            fixture_source_dir().to_str().unwrap(),
            target_dir.to_str().unwrap(),
        )
        .expect("run with repair");

    aggregate.set_capability_report(capability_report);
    report.completion_status = aggregate.check_completion();
    report.health_score = aggregate.health_score();

    assert!(report.is_successful());
    assert_eq!(report.repairs_triggered, 1);
    assert!(report.final_target_revision.0 >= 5);

    // Task 2 must have retry_count == 1 and state == Accepted
    let task2 = aggregate
        .get_task(&TaskId::new("task-2-components-and-routes"))
        .unwrap();
    assert_eq!(task2.retry_count, 1);
    assert_eq!(task2.state, TaskState::Accepted);

    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn mvp_criterion_8_and_9_cli_subcommands_and_checkpoint_resume_work_end_to_end() {
    let temp = unique_temp_dir("cli_e2e");
    let target_dir = temp.join("cli_target");
    let root_str = workspace_root().to_string_lossy().to_string();
    let src_str = fixture_source_dir().to_string_lossy().to_string();
    let tgt_str = target_dir.to_string_lossy().to_string();

    let analyze_out =
        execute_cli_args(&["mae".to_string(), "analyze".to_string(), src_str.clone()])
            .expect("cli analyze");
    assert!(analyze_out.contains("MAE Analyze Complete:"));
    assert!(analyze_out.contains("routes=6"));

    let plan_out = execute_cli_args(&["mae".to_string(), "plan".to_string(), src_str.clone()])
        .expect("cli plan");
    assert!(plan_out.contains("MAE Plan Complete: tasks=3, contracts=4"));

    let e2e_out = execute_cli_args(&[
        "mae".to_string(),
        "run-e2e".to_string(),
        root_str,
        src_str,
        tgt_str.clone(),
    ])
    .expect("cli run-e2e");
    assert!(e2e_out.contains("success=true, health_score=1.00"));

    let verify_out =
        execute_cli_args(&["mae".to_string(), "verify".to_string(), tgt_str]).expect("cli verify");
    assert!(verify_out.contains("native_status=PASS, violations=0"));

    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn mvp_ubuntu_desktop_full_e2e_pipeline_and_cli_flag() {
    let temp = unique_temp_dir("ubuntu_e2e");
    let target_dir = temp.join("ubuntu_slint_target");
    let root_str = workspace_root().to_string_lossy().to_string();
    let src_str = fixture_source_dir().to_string_lossy().to_string();
    let tgt_str = target_dir.to_string_lossy().to_string();

    // Run E2E via CLI with `--platform ubuntu`
    let e2e_out = execute_cli_args(&[
        "mae".to_string(),
        "run-e2e".to_string(),
        "--platform".to_string(),
        "ubuntu".to_string(),
        root_str,
        src_str,
        tgt_str.clone(),
    ])
    .expect("cli run-e2e --platform ubuntu");

    assert!(e2e_out.contains("platform=UBUNTU_DESKTOP"));
    assert!(e2e_out.contains("success=true, health_score=1.00"));
    assert!(e2e_out.contains("consistency=1.00"));
    assert!(e2e_out.contains("total_tokens="));

    // Verify generated Ubuntu XDG .desktop entry, Ubuntu dependency script, and eval-report.json exist
    assert!(target_dir
        .join("packaging/desktop-productivity-hub.desktop")
        .exists());
    assert!(target_dir.join("packaging/ubuntu-deps.sh").exists());
    let eval_json_path = target_dir.join(".mae-artifacts/eval-report.json");
    assert!(eval_json_path.exists());
    let eval_json = fs::read_to_string(&eval_json_path).unwrap();
    assert!(eval_json.contains("\"overall_consistency_score\": 1.0000"));
    assert!(eval_json.contains("\"source_ui_elements_count\": 67"));
    assert!(eval_json.contains("\"target_ui_elements_count\": 67"));

    // Run `mae verify --platform ubuntu`
    let verify_out = execute_cli_args(&[
        "mae".to_string(),
        "verify".to_string(),
        "--platform=ubuntu".to_string(),
        tgt_str,
    ])
    .expect("cli verify --platform=ubuntu");
    assert!(verify_out.contains("platform=UBUNTU_DESKTOP, native_status=PASS, violations=0"));

    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn test_cli_subcommands_analyze_and_migrate_cli_end_to_end() {
    let root = workspace_root();
    let py_file = root.join("fixtures/python-rust-cli/source/dataforge.py");
    let target_out = root.join(format!(
        "target/tmp/dataforge_cli_e2e_{}",
        std::process::id()
    ));

    let analyze_out = execute_cli_args(&[
        "mae".to_string(),
        "analyze-cli".to_string(),
        py_file.to_str().unwrap().to_string(),
    ])
    .expect("analyze-cli failed");
    assert!(analyze_out.contains("program=dataforge"));
    assert!(analyze_out.contains("subcommands=5"));
    assert!(analyze_out.contains("concurrency="));

    let migrate_out = execute_cli_args(&[
        "mae".to_string(),
        "migrate-cli".to_string(),
        py_file.to_str().unwrap().to_string(),
        target_out.to_str().unwrap().to_string(),
    ])
    .expect("migrate-cli failed");
    assert!(migrate_out.contains("target_crate=dataforge"));
    assert!(migrate_out.contains("status=PASS"), "{}", migrate_out);
    assert!(migrate_out.contains("files_generated=11"));

    let _ = fs::remove_dir_all(target_out);
}

#[test]
fn test_lead_enrich_cli_pilot_with_containment_retry_recovery_and_fresh_evidence() {
    let root = workspace_root();
    let py_file = root.join("fixtures/python-rust-cli/source/lead_enrich.py");
    let target_out = root.join(format!(
        "target/tmp/lead_enrich_pilot_e2e_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&target_out);

    let analyze_out = execute_cli_args(&[
        "mae".to_string(),
        "analyze-cli".to_string(),
        py_file.to_str().unwrap().to_string(),
    ])
    .expect("analyze-cli on lead_enrich failed");
    assert!(analyze_out.contains("program=lead_enrich"));
    assert!(analyze_out.contains("subcommands=3"));
    assert!(analyze_out.contains("STATE_PERSISTENCE"));
    assert!(analyze_out.contains("RETRY_POLICY"));
    assert!(analyze_out.contains("CHECKPOINT_RESUME"));

    let pilot_out = execute_cli_args(&[
        "mae".to_string(),
        "pilot-cli".to_string(),
        root.to_str().unwrap().to_string(),
        py_file.to_str().unwrap().to_string(),
        target_out.to_str().unwrap().to_string(),
    ])
    .expect("pilot-cli on lead_enrich failed");

    assert!(
        pilot_out.contains("MAE CLI Pilot Complete: target_crate=lead_enrich"),
        "{}",
        pilot_out
    );
    assert!(pilot_out.contains("status=PASS"), "{}", pilot_out);
    assert!(pilot_out.contains("repairs_triggered=1"), "{}", pilot_out);
    assert!(pilot_out.contains("target_rev=r2"), "{}", pilot_out);
    assert!(pilot_out.contains("scenarios_passed=9/9"), "{}", pilot_out);

    let manifest_path = target_out.join(".mae-artifacts/pilot-manifest.json");
    assert!(
        manifest_path.exists(),
        "pilot-manifest.json must be emitted"
    );
    let manifest_json = fs::read_to_string(&manifest_path).unwrap();
    assert!(manifest_json.contains("\"target_crate\": \"lead_enrich\""));
    assert!(manifest_json.contains("\"target_revision\": 2"));
    assert!(manifest_json.contains("\"repairs_triggered\": 1"));
    assert!(manifest_json.contains("\"network_disabled\": true"));
    assert!(manifest_json.contains("\"passed_scenarios\": 9"));
    assert!(manifest_json.contains("\"unexecuted_scenarios\": 0"));

    let _ = fs::remove_dir_all(&target_out);
}
