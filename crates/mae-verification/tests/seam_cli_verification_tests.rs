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
