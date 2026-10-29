use std::fs;
use std::path::PathBuf;

use mae_domain::ids::ArtifactId;
use mae_domain::ports::CliSourceAdapterPort;
use mae_domain::semantic_cli::{CliArgumentKind, CliGapReasonCode, CliValueType};

use mae_adapter_python_cli::PythonCliSourceAdapter;

fn get_fixture_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let fixture = PathBuf::from(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/python-rust-cli/source/dataforge.py");
    fixture.to_str().unwrap().to_string()
}

#[test]
fn test_discover_python_cli_source_artifact() {
    let fixture_path = get_fixture_path();
    let adapter = PythonCliSourceAdapter::new();

    let artifact = adapter
        .discover(&fixture_path)
        .expect("Failed to discover artifact");
    assert_eq!(artifact.id, ArtifactId::new("python-cli-source-dataforge"));
    assert_eq!(artifact.revision.0, 1);
    assert!(!artifact.content_hash.0.is_empty());
}

#[test]
fn test_extract_semantics_from_dataforge_fixture() {
    let fixture_path = get_fixture_path();
    let adapter = PythonCliSourceAdapter::new();

    let model = adapter
        .extract_semantics(&fixture_path)
        .expect("Failed to extract semantics from dataforge.py");

    assert_eq!(model.program_name, "dataforge");
    assert_eq!(model.version, "1.4.0");
    assert!(
        model.description.contains("data manipulation") || model.description.contains("analytics")
    );

    // Subcommands: stats, filter, transform, inspect, crunch
    assert_eq!(model.total_subcommands(), 5);
    assert!(model.root_command.find_subcommand("stats").is_some());
    assert!(model.root_command.find_subcommand("filter").is_some());
    assert!(model.root_command.find_subcommand("transform").is_some());
    assert!(model.root_command.find_subcommand("inspect").is_some());
    assert!(model.root_command.find_subcommand("crunch").is_some());

    // Concurrency patterns
    assert!(model.is_concurrent());
    assert!(model
        .concurrency_patterns
        .contains(&mae_domain::semantic_cli::CliConcurrencyPattern::MultiProcessing));
    assert!(model
        .concurrency_patterns
        .contains(&mae_domain::semantic_cli::CliConcurrencyPattern::ParallelThreads));
    assert!(model
        .concurrency_patterns
        .contains(&mae_domain::semantic_cli::CliConcurrencyPattern::AsyncEventLoop));

    let crunch = model.root_command.find_subcommand("crunch").unwrap();
    assert!(crunch.arguments.iter().any(|a| a.name == "workers"));
    assert!(crunch.arguments.iter().any(|a| a.name == "mode"));
    assert!(crunch.arguments.iter().any(|a| a.name == "rounds"));
    assert!(crunch.arguments.iter().any(|a| a.name == "benchmark"));

    // Root flags
    let has_verbose = model.root_command.arguments.iter().any(|a| {
        a.name == "verbose"
            && matches!(
                a.kind,
                CliArgumentKind::Flag {
                    short: Some('v'),
                    ..
                }
            )
    });
    let has_quiet = model.root_command.arguments.iter().any(|a| {
        a.name == "quiet"
            && matches!(
                a.kind,
                CliArgumentKind::Flag {
                    short: Some('q'),
                    ..
                }
            )
    });
    assert!(has_verbose, "Expected --verbose / -v flag on root");
    assert!(has_quiet, "Expected --quiet / -q flag on root");

    // Stats subcommand arguments
    let stats = model.root_command.find_subcommand("stats").unwrap();
    let stats_input = stats.arguments.iter().find(|a| a.name == "input");
    assert!(stats_input.is_some());
    let stats_format = stats.arguments.iter().find(|a| a.name == "format").unwrap();
    assert!(matches!(
        &stats_format.value_type,
        CliValueType::EnumChoices(choices) if choices.contains(&"json".to_string()) && choices.contains(&"csv".to_string())
    ));

    // Filter subcommand arguments
    let filter = model.root_command.find_subcommand("filter").unwrap();
    assert!(filter.arguments.iter().any(|a| a.name == "condition"));
    assert!(filter
        .arguments
        .iter()
        .any(|a| a.name == "sort-by" || a.name == "sort_by"));
    assert!(filter.arguments.iter().any(|a| a.name == "descending"));
    assert!(filter.arguments.iter().any(|a| a.name == "limit"));

    // Transform subcommand arguments
    let transform = model.root_command.find_subcommand("transform").unwrap();
    assert!(transform.arguments.iter().any(|a| a.name.contains("upper")));
    assert!(transform.arguments.iter().any(|a| a.name.contains("scale")));
    assert!(transform.arguments.iter().any(|a| a.name.contains("hash")));

    // Exit codes
    assert_eq!(model.exit_codes.success, 0);
    assert_eq!(model.exit_codes.argument_error, 1);
    assert_eq!(model.exit_codes.io_error, 2);
    assert_eq!(model.exit_codes.data_error, 3);

    // No gaps in standard dataforge.py
    assert!(model.detected_gaps.is_empty());
}

#[test]
fn test_classify_gaps_on_dynamic_python_script() {
    let adapter = PythonCliSourceAdapter::new();
    let script_with_gaps = r#"
import sys
import os
import ctypes
import asyncio
import importlib
import pandas as pd

def run():
    eval("print('danger')")
"#;

    let tmp_path = std::env::temp_dir().join(format!("gap_test_{}.py", std::process::id()));
    fs::write(&tmp_path, script_with_gaps).unwrap();

    let model = adapter
        .extract_semantics(tmp_path.to_str().unwrap())
        .expect("Extraction failed");

    let _ = fs::remove_file(tmp_path);

    assert!(model.has_gap(&CliGapReasonCode::DynamicEvalOrExec));
    assert!(model.has_gap(&CliGapReasonCode::AsyncEventLoopUnsupported));
    assert!(model.has_gap(&CliGapReasonCode::DynamicModuleImport));
    assert!(model.has_gap(&CliGapReasonCode::NativeCExtensionRequired));
    assert!(model.has_gap(&CliGapReasonCode::UnmappedPyPiDependency(
        "pandas".to_string()
    )));
}

#[test]
fn test_assess_capabilities_report() {
    let fixture_path = get_fixture_path();
    let adapter = PythonCliSourceAdapter::new();

    let model = adapter.extract_semantics(&fixture_path).unwrap();
    let report = adapter.assess_capabilities(&model).unwrap();

    assert!(!report.has_critical_unresolved_gaps());
    assert_eq!(report.gaps.len(), 0);
    assert!(report
        .assessments
        .iter()
        .any(|a| a.feature_name == "cli_concurrency_scoped_parallelism"));
}

#[test]
fn test_extract_semantics_and_pipeline_capabilities_from_lead_enrich_fixture() {
    use mae_domain::semantic_cli::CliPipelineCapability;

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let lead_fixture = PathBuf::from(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/python-rust-cli/source/lead_enrich.py");

    let adapter = PythonCliSourceAdapter::new();
    let model = adapter
        .extract_semantics(lead_fixture.to_str().unwrap())
        .expect("Failed to extract semantics from lead_enrich.py");

    assert_eq!(model.program_name, "lead_enrich");
    assert_eq!(model.version, "1.2.0");
    assert_eq!(model.total_subcommands(), 3);
    assert!(model.root_command.find_subcommand("ingest").is_some());
    assert!(model.root_command.find_subcommand("enrich").is_some());
    assert!(model.root_command.find_subcommand("report").is_some());

    // Exit codes including EXIT_STAGE_ERROR = 4
    assert_eq!(model.exit_codes.success, 0);
    assert_eq!(model.exit_codes.argument_error, 1);
    assert_eq!(model.exit_codes.io_error, 2);
    assert_eq!(model.exit_codes.data_error, 3);
    assert_eq!(model.exit_codes.stage_error, 4);

    // Pipeline operational capabilities
    assert!(model.has_pipeline_capability(CliPipelineCapability::StatePersistence));
    assert!(model.has_pipeline_capability(CliPipelineCapability::CacheLedger));
    assert!(model.has_pipeline_capability(CliPipelineCapability::RetryPolicy));
    assert!(model.has_pipeline_capability(CliPipelineCapability::CheckpointResume));

    let report = adapter.assess_capabilities(&model).unwrap();
    assert!(!report.has_critical_unresolved_gaps());
    assert!(report
        .assessments
        .iter()
        .any(|a| a.feature_name == "cli_pipeline_state_persistence"));
    assert!(report
        .assessments
        .iter()
        .any(|a| a.feature_name == "cli_pipeline_retry_policy"));
    assert!(report
        .assessments
        .iter()
        .any(|a| a.feature_name == "cli_pipeline_checkpoint_resume"));
}
