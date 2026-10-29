use std::fs;
use std::path::{Path, PathBuf};

use mae_adapter_python_cli::PythonCliSourceAdapter;
use mae_adapter_rust_cli::RustCliTargetAdapter;
use mae_domain::ids::ArtifactRevision;
use mae_domain::ports::{CliSourceAdapterPort, CliTargetAdapterPort};
use mae_verification::{CliDifferentialScenario, CliDifferentialVerifier};

struct TestTargetDir {
    path: PathBuf,
}

impl TestTargetDir {
    fn new(name: &str) -> Self {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path = Path::new(manifest_dir)
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("target/tmp")
            .join(format!("diff_test_{}_{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn as_str(&self) -> &str {
        self.path.to_str().unwrap()
    }
}

impl Drop for TestTargetDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn get_workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn test_cli_differential_verifier_passes_on_dataforge_fixture() {
    let root = get_workspace_root();
    let py_script = root.join("fixtures/python-rust-cli/source/dataforge.py");
    let json_data = root.join("fixtures/python-rust-cli/data/employees.json");

    assert!(py_script.exists(), "dataforge.py fixture must exist");
    assert!(json_data.exists(), "employees.json dataset must exist");

    // 1. Extract semantics using Python adapter
    let py_adapter = PythonCliSourceAdapter::new();
    let model = py_adapter
        .extract_semantics(py_script.to_str().unwrap())
        .expect("Failed to extract semantics from dataforge.py");

    // 2. Scaffold native Rust CLI target using Rust adapter
    let target = TestTargetDir::new("dataforge");
    let rust_adapter = RustCliTargetAdapter::new();
    rust_adapter
        .scaffold_target(&model, target.as_str())
        .expect("Failed to scaffold Rust target");

    // 3. Define comprehensive differential scenario matrix
    let json_path_str = json_data.to_str().unwrap();
    let scenarios = vec![
        CliDifferentialScenario::new("SCENARIO_VERSION", &["--version"]),
        CliDifferentialScenario::new("SCENARIO_INSPECT", &["inspect", json_path_str]),
        CliDifferentialScenario::new(
            "SCENARIO_STATS_ROUND",
            &["stats", json_path_str, "--round", "2"],
        ),
        CliDifferentialScenario::new(
            "SCENARIO_FILTER_COND_SORT",
            &[
                "filter",
                json_path_str,
                "-c",
                "age >= 30",
                "-s",
                "salary",
                "-d",
            ],
        ),
        CliDifferentialScenario::new(
            "SCENARIO_FILTER_LIMIT",
            &["filter", json_path_str, "-l", "2"],
        ),
        CliDifferentialScenario::new(
            "SCENARIO_TRANSFORM_ALL",
            &[
                "transform",
                json_path_str,
                "--upper-col",
                "name",
                "--scale-col",
                "salary",
                "--scale-factor",
                "1.1",
                "--hash-col",
                "department",
            ],
        ),
        CliDifferentialScenario::new("SCENARIO_MISSING_SUBCOMMAND", &[]),
        CliDifferentialScenario::new(
            "SCENARIO_SYNTAX_ERROR",
            &["filter", json_path_str, "-c", "bad condition"],
        ),
        CliDifferentialScenario::new(
            "SCENARIO_FILE_NOT_FOUND",
            &["stats", "nonexistent_file.json"],
        ),
        CliDifferentialScenario::new(
            "SCENARIO_CRUNCH_MULTIPROCESSING",
            &[
                "crunch",
                json_path_str,
                "--mode",
                "multiprocessing",
                "-w",
                "2",
                "-r",
                "15",
            ],
        ),
        CliDifferentialScenario::new(
            "SCENARIO_CRUNCH_THREADING",
            &[
                "crunch",
                json_path_str,
                "--mode",
                "threading",
                "-w",
                "2",
                "-r",
                "15",
            ],
        ),
        CliDifferentialScenario::new(
            "SCENARIO_CRUNCH_ASYNC",
            &[
                "crunch",
                json_path_str,
                "--mode",
                "async",
                "-w",
                "2",
                "-r",
                "15",
            ],
        ),
        CliDifferentialScenario::new(
            "SCENARIO_CRUNCH_SEQUENTIAL",
            &["crunch", json_path_str, "--mode", "sequential", "-r", "15"],
        ),
    ];

    // 4. Run differential verifier
    let verifier = CliDifferentialVerifier::new();
    let evidence = verifier
        .verify(
            py_script.to_str().unwrap(),
            target.as_str(),
            &scenarios,
            ArtifactRevision::new(1),
        )
        .expect("Verification run failed");

    // 5. Assert contract evidence
    assert!(
        evidence.status.is_pass(),
        "Verification must pass with exact differential parity. Summary:\n{}",
        evidence.summary
    );
    assert!(
        evidence.coverage.is_complete(),
        "Scenario coverage must be complete without omissions"
    );
    assert_eq!(evidence.coverage.passed.len(), scenarios.len());
    assert!(evidence.coverage.unexecuted.is_empty());
}

#[test]
fn test_cli_differential_verifier_lead_enrich_pipeline_cache_retry_resume_and_filesystem_parity() {
    let root = get_workspace_root();
    let py_script = root.join("fixtures/python-rust-cli/source/lead_enrich.py");
    let raw_leads = root.join("fixtures/python-rust-cli/data/leads_raw.json");
    let malformed_leads = root.join("fixtures/python-rust-cli/data/leads_malformed.json");

    let py_adapter = PythonCliSourceAdapter::new();
    let model = py_adapter
        .extract_semantics(py_script.to_str().unwrap())
        .expect("Failed to extract semantics from lead_enrich.py");

    let target = TestTargetDir::new("lead_enrich");
    let rust_adapter = RustCliTargetAdapter::new();
    rust_adapter
        .scaffold_target(&model, target.as_str())
        .expect("Failed to scaffold lead_enrich Rust target");

    let raw_str = raw_leads.to_str().unwrap();
    let bad_str = malformed_leads.to_str().unwrap();

    let scenarios = vec![
        CliDifferentialScenario::new("SCENARIO_VERSION", &["--version"]).with_expected_exit_code(0),
        CliDifferentialScenario::new("SCENARIO_MISSING_SUBCOMMAND", &[])
            .with_expected_exit_code(1)
            .with_expected_stderr_contains("ERROR [ARG]"),
        CliDifferentialScenario::new(
            "SCENARIO_MISSING_INPUT_FILE",
            &["ingest", "nonexistent_leads.json"],
        )
        .with_expected_exit_code(2)
        .with_expected_stderr_contains("ERROR [IO]"),
        CliDifferentialScenario::new("SCENARIO_MALFORMED_SCHEMA", &["ingest", bad_str])
            .with_expected_exit_code(3)
            .with_expected_stderr_contains("ERROR [DATA]"),
        CliDifferentialScenario::new(
            "SCENARIO_INGEST_DEDUPE_AND_FILE",
            &[
                "ingest",
                raw_str,
                "--dedupe",
                "-o",
                "{OUT_DIR}/ingested.json",
            ],
        )
        .with_expected_exit_code(0)
        .with_compared_file("ingested.json"),
        CliDifferentialScenario::new(
            "SCENARIO_ENRICH_RETRY_EXHAUSTED_PARTIAL_STATE",
            &[
                "enrich",
                "{OUT_DIR}/ingested.json",
                "-s",
                "{OUT_DIR}/state.json",
                "-f",
                "flaky-provider.org",
                "-r",
                "1",
            ],
        )
        .with_expected_exit_code(4)
        .with_expected_stderr_contains("WARN [RETRY]")
        .with_expected_stderr_contains("ERROR [STAGE]: retries exhausted")
        .with_compared_file("state.json"),
        CliDifferentialScenario::new(
            "SCENARIO_REPORT_FAILED_STAGE",
            &["report", "{OUT_DIR}/state.json", "-f", "summary"],
        )
        .with_expected_exit_code(0),
        CliDifferentialScenario::new(
            "SCENARIO_ENRICH_RESUME_AND_RECOVER",
            &[
                "enrich",
                "{OUT_DIR}/ingested.json",
                "-s",
                "{OUT_DIR}/state.json",
                "-o",
                "{OUT_DIR}/enriched.json",
                "-f",
                "flaky-provider.org",
                "-r",
                "2",
                "--resume",
            ],
        )
        .with_expected_exit_code(0)
        .with_expected_stderr_contains("WARN [RETRY]")
        .with_compared_file("state.json")
        .with_compared_file("enriched.json"),
        CliDifferentialScenario::new(
            "SCENARIO_REPORT_COMPLETED_JSON",
            &["report", "{OUT_DIR}/state.json", "-f", "json"],
        )
        .with_expected_exit_code(0),
    ];

    let verifier = CliDifferentialVerifier::new();
    let evidence = verifier
        .verify(
            py_script.to_str().unwrap(),
            target.as_str(),
            &scenarios,
            ArtifactRevision::new(1),
        )
        .expect("Lead enrich verification run failed");

    assert!(
        evidence.status.is_pass(),
        "Lead enrich differential verification must pass. Summary:\n{}",
        evidence.summary
    );
    assert!(evidence.coverage.is_complete());
    assert_eq!(evidence.coverage.passed.len(), scenarios.len());
}

#[test]
fn test_cli_differential_verifier_discloses_excluded_scenarios_and_blocks_pass_by_omission() {
    use mae_domain::verification::VerificationStatus;

    let root = get_workspace_root();
    let py_script = root.join("fixtures/python-rust-cli/source/lead_enrich.py");
    let py_adapter = PythonCliSourceAdapter::new();
    let model = py_adapter
        .extract_semantics(py_script.to_str().unwrap())
        .unwrap();

    let target = TestTargetDir::new("lead_enrich_excluded");
    RustCliTargetAdapter::new()
        .scaffold_target(&model, target.as_str())
        .unwrap();

    let scenarios = vec![
        CliDifferentialScenario::new("SCENARIO_VERSION", &["--version"]),
        CliDifferentialScenario::new("SCENARIO_LIVE_WEBHOOK", &["enrich", "live"])
            .with_excluded_reason("Live external webhook excluded from offline pilot"),
    ];

    let verifier = CliDifferentialVerifier::new();
    let evidence = verifier
        .verify(
            py_script.to_str().unwrap(),
            target.as_str(),
            &scenarios,
            ArtifactRevision::new(1),
        )
        .unwrap();

    assert_eq!(evidence.status, VerificationStatus::Inconclusive);
    assert!(!evidence.coverage.is_complete());
    assert_eq!(
        evidence.coverage.unexecuted,
        vec!["SCENARIO_LIVE_WEBHOOK".to_string()]
    );
}

#[test]
fn test_cli_differential_verifier_invalidates_stale_binary_on_broken_target_rebuild() {
    let root = get_workspace_root();
    let py_script = root.join("fixtures/python-rust-cli/source/lead_enrich.py");
    let py_adapter = PythonCliSourceAdapter::new();
    let model = py_adapter
        .extract_semantics(py_script.to_str().unwrap())
        .unwrap();

    let target = TestTargetDir::new("lead_enrich_stale_bin");
    RustCliTargetAdapter::new()
        .scaffold_target(&model, target.as_str())
        .unwrap();

    let scenarios = vec![CliDifferentialScenario::new(
        "SCENARIO_VERSION",
        &["--version"],
    )];

    let verifier = CliDifferentialVerifier::new();
    let ev1 = verifier
        .verify(
            py_script.to_str().unwrap(),
            target.as_str(),
            &scenarios,
            ArtifactRevision::new(1),
        )
        .unwrap();
    assert!(ev1.status.is_pass());

    // Now corrupt src/main.rs for revision 2: verifier must not reuse the revision 1 binary!
    fs::write(
        target.path.join("src/main.rs"),
        "fn main() { compile_error!(\"broken target generation\"); }",
    )
    .unwrap();

    let ev2_res = verifier.verify(
        py_script.to_str().unwrap(),
        target.as_str(),
        &scenarios,
        ArtifactRevision::new(2),
    );
    assert!(
        ev2_res.is_err(),
        "Broken rebuild must fail and remove stale binary rather than reusing revision 1 executable"
    );
}

#[test]
fn test_cli_build_and_native_compliance_verifiers_reject_python_wrapper_cheats() {
    use mae_domain::{
        Contract, ContractId, ContractSeverity, VerificationCategory, VerifierPluginPort,
    };
    use mae_verification::{BuildVerifier, NativeComplianceVerifier};

    let root = get_workspace_root();
    let py_script = root.join("fixtures/python-rust-cli/source/lead_enrich.py");
    let model = PythonCliSourceAdapter::new()
        .extract_semantics(py_script.to_str().unwrap())
        .unwrap();

    let target = TestTargetDir::new("lead_enrich_compliance");
    RustCliTargetAdapter::new()
        .scaffold_target(&model, target.as_str())
        .unwrap();

    let c_build = Contract::new(
        ContractId::new("C_CLI_BUILD"),
        "CLI Build",
        "Must compile",
        VerificationCategory::Build,
        ContractSeverity::Critical,
    );
    let c_native = Contract::new(
        ContractId::new("C_CLI_NATIVE"),
        "CLI Native Compliance",
        "No Python wrapper",
        VerificationCategory::NativeCompliance,
        ContractSeverity::Critical,
    );

    let build_v = BuildVerifier::for_cli_target(&target.path, false);
    let native_v = NativeComplianceVerifier::for_cli_target(&target.path);

    let ev_b1 = build_v.verify(&c_build, ArtifactRevision::new(1)).unwrap();
    let ev_n1 = native_v
        .verify(&c_native, ArtifactRevision::new(1))
        .unwrap();
    assert!(ev_b1.status.is_pass(), "{}", ev_b1.summary);
    assert!(ev_n1.status.is_pass(), "{}", ev_n1.summary);

    // Inject Python subprocess wrapper cheat into src/main.rs
    let main_path = target.path.join("src/main.rs");
    let orig_main = fs::read_to_string(&main_path).unwrap();
    fs::write(
        &main_path,
        format!("{orig_main}\n// cheat: std::process::Command::new(\"python3\")\n"),
    )
    .unwrap();

    let ev_n2 = native_v
        .verify(&c_native, ArtifactRevision::new(2))
        .unwrap();
    assert!(
        ev_n2.status.is_fail(),
        "NativeComplianceVerifier::for_cli_target must reject Python subprocess wrapper cheats"
    );
}
