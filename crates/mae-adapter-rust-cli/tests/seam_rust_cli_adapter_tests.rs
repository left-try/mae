use std::fs;
use std::path::PathBuf;
use std::process::Command;

use mae_domain::ports::CliTargetAdapterPort;
use mae_domain::semantic_cli::{
    CliArgument, CliCommandNode, CliExitCodeMapping, CliIoContract, CliSemanticModel, CliValueType,
};

use mae_domain::task::MigrationTask;

use mae_adapter_rust_cli::RustCliTargetAdapter;

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(test_name: &str) -> Self {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path = PathBuf::from(manifest_dir)
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("target/tmp")
            .join(format!("test_target_{}_{}", test_name, std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn as_str(&self) -> &str {
        self.path.to_str().unwrap()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn create_sample_model() -> CliSemanticModel {
    let root = CliCommandNode::new("dataforge", "Data analytics CLI")
        .with_argument(CliArgument::flag("verbose", Some('v'), "Verbose output"))
        .with_argument(CliArgument::flag("quiet", Some('q'), "Suppress logs"))
        .with_subcommand(
            CliCommandNode::new("stats", "Compute statistics")
                .with_argument(CliArgument::positional("input", false, "Input file"))
                .with_argument(CliArgument::option(
                    "format",
                    "format",
                    Some('f'),
                    CliValueType::EnumChoices(vec!["json".to_string(), "csv".to_string()]),
                    "Format",
                ))
                .with_argument(CliArgument::option(
                    "round",
                    "round",
                    Some('r'),
                    CliValueType::Integer,
                    "Round digits",
                )),
        )
        .with_subcommand(
            CliCommandNode::new("filter", "Filter rows")
                .with_argument(CliArgument::positional("input", false, "Input file"))
                .with_argument(CliArgument::option(
                    "condition",
                    "condition",
                    Some('c'),
                    CliValueType::String,
                    "Condition",
                ))
                .with_argument(CliArgument::option(
                    "sort-by",
                    "sort-by",
                    Some('s'),
                    CliValueType::String,
                    "Sort by",
                ))
                .with_argument(CliArgument::flag("descending", Some('d'), "Descending"))
                .with_argument(CliArgument::option(
                    "limit",
                    "limit",
                    Some('l'),
                    CliValueType::Integer,
                    "Limit",
                )),
        )
        .with_subcommand(
            CliCommandNode::new("transform", "Transform rows")
                .with_argument(CliArgument::positional("input", false, "Input file"))
                .with_argument(CliArgument::option(
                    "upper-col",
                    "upper-col",
                    None,
                    CliValueType::String,
                    "Uppercase column",
                ))
                .with_argument(CliArgument::option(
                    "lower-col",
                    "lower-col",
                    None,
                    CliValueType::String,
                    "Lowercase column",
                ))
                .with_argument(CliArgument::option(
                    "scale-col",
                    "scale-col",
                    None,
                    CliValueType::String,
                    "Scale column",
                ))
                .with_argument(CliArgument::option(
                    "scale-factor",
                    "scale-factor",
                    None,
                    CliValueType::Float,
                    "Scale factor",
                ))
                .with_argument(CliArgument::option(
                    "hash-col",
                    "hash-col",
                    None,
                    CliValueType::String,
                    "Hash column",
                )),
        )
        .with_subcommand(
            CliCommandNode::new("inspect", "Inspect schema")
                .with_argument(CliArgument::positional("input", false, "Input file")),
        );

    let mut model = CliSemanticModel::new("dataforge", "1.4.0", "Data analytics CLI", root);
    model.exit_codes = CliExitCodeMapping::default();
    model.io_contract = CliIoContract::default();
    model
}

#[test]
fn test_scaffold_target_creates_canonical_files() {
    let tmp = TempDir::new("scaffold");
    let adapter = RustCliTargetAdapter::new();
    let model = create_sample_model();

    let summary = adapter
        .scaffold_target(&model, tmp.as_str())
        .expect("Target scaffolding failed");

    assert_eq!(summary.target_crate_name, "dataforge");
    assert!(tmp.path.join("Cargo.toml").exists());
    assert!(tmp.path.join("src/main.rs").exists());
    assert!(tmp.path.join("src/cli.rs").exists());
    assert!(tmp.path.join("src/core/mod.rs").exists());
    assert!(tmp.path.join("src/core/data.rs").exists());
    assert!(tmp.path.join("src/core/stats.rs").exists());
    assert!(tmp.path.join("src/core/filter.rs").exists());
    assert!(tmp.path.join("src/core/transform.rs").exists());
    assert!(tmp.path.join("src/core/inspect.rs").exists());
    assert!(tmp.path.join("src/core/crunch.rs").exists());
    assert!(tmp.path.join("src/core/sha256.rs").exists());
}

#[test]
fn test_scaffold_target_logic_cli_separation() {
    let tmp = TempDir::new("separation");
    let adapter = RustCliTargetAdapter::new();
    let model = create_sample_model();

    adapter
        .scaffold_target(&model, tmp.as_str())
        .expect("Scaffolding failed");

    let cli_rs = fs::read_to_string(tmp.path.join("src/cli.rs")).unwrap();
    let core_rs = fs::read_to_string(tmp.path.join("src/core/mod.rs")).unwrap();

    // cli.rs must define Command / Subcommand structs
    assert!(cli_rs.contains("pub enum Subcommand") || cli_rs.contains("pub struct CliArgs"));
    assert!(cli_rs.contains("parse_args"));

    // core_rs must expose run methods decoupled from argv
    assert!(core_rs.contains("run_stats"));
    assert!(core_rs.contains("run_filter"));
    assert!(core_rs.contains("run_transform"));
    assert!(core_rs.contains("run_inspect"));
    assert!(core_rs.contains("run_crunch"));
}

#[test]
fn test_scaffolded_target_compiles_and_passes_unit_tests() {
    let tmp = TempDir::new("compile");
    let adapter = RustCliTargetAdapter::new();
    let model = create_sample_model();

    adapter
        .scaffold_target(&model, tmp.as_str())
        .expect("Scaffolding failed");

    let workspace_tmp = tmp.path.parent().unwrap().to_str().unwrap();

    // Check cargo compilation of the generated target
    let output = Command::new("cargo")
        .arg("test")
        .current_dir(&tmp.path)
        .env("TMPDIR", workspace_tmp)
        .output()
        .expect("Failed to run cargo test on target");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "Target cargo test failed!\nSTDOUT:\n{}\nSTDERR:\n{}",
            stdout, stderr
        );
    }
}

#[test]
fn test_emit_target_code_advances_artifact_revision() {
    let tmp = TempDir::new("emit");
    let adapter = RustCliTargetAdapter::new();
    let model = create_sample_model();

    adapter
        .scaffold_target(&model, tmp.as_str())
        .expect("Scaffolding failed");

    let task = MigrationTask::new(
        "T1",
        "Emit rust CLI",
        mae_domain::capability::MigrationStrategy::DirectNativeMapping,
    );
    let rev1 = adapter
        .emit_target_code(&task, &model, tmp.as_str())
        .unwrap();
    let rev2 = adapter
        .emit_target_code(&task, &model, tmp.as_str())
        .unwrap();

    assert_eq!(rev1.as_u64(), 1);
    assert_eq!(rev2.as_u64(), 2);
}
