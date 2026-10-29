use mae_domain::semantic_cli::{
    CliArgument, CliArgumentKind, CliCommandNode, CliExitCodeMapping, CliGapReasonCode,
    CliIoContract, CliSemanticModel, CliValueType,
};

#[test]
fn test_cli_semantic_model_construction_and_hash_determinism() {
    let root = CliCommandNode::new("dataforge", "Data analytics CLI")
        .with_argument(CliArgument::flag("verbose", Some('v'), "Verbose output"))
        .with_subcommand(
            CliCommandNode::new("stats", "Compute statistics")
                .with_argument(CliArgument::positional("input", false, "Input file path"))
                .with_argument(CliArgument::option(
                    "format",
                    "format",
                    Some('f'),
                    CliValueType::EnumChoices(vec!["json".to_string(), "csv".to_string()]),
                    "Output format",
                )),
        );

    let model1 = CliSemanticModel::new("dataforge", "1.4.0", "Data analytics CLI", root.clone());
    let model2 = CliSemanticModel::new("dataforge", "1.4.0", "Data analytics CLI", root);

    assert_eq!(model1, model2);
    assert_eq!(model1.content_hash(), model2.content_hash());
    assert_eq!(model1.total_subcommands(), 1);
}

#[test]
fn test_cli_argument_kinds_and_defaults() {
    let pos = CliArgument::positional("target", true, "Target file");
    match pos.kind {
        CliArgumentKind::Positional { required } => assert!(required),
        _ => panic!("Expected positional argument"),
    }

    let flag = CliArgument::flag("quiet", Some('q'), "Suppress logs");
    match flag.kind {
        CliArgumentKind::Flag { short, long } => {
            assert_eq!(short, Some('q'));
            assert_eq!(long, "quiet");
        }
        _ => panic!("Expected flag argument"),
    }
    assert_eq!(flag.default_value, Some("false".to_string()));

    let opt = CliArgument::option(
        "limit",
        "limit",
        Some('l'),
        CliValueType::Integer,
        "Row limit",
    );
    match opt.kind {
        CliArgumentKind::NamedOption { short, long } => {
            assert_eq!(short, Some('l'));
            assert_eq!(long, "limit");
        }
        _ => panic!("Expected option argument"),
    }
    assert_eq!(opt.value_type, CliValueType::Integer);
}

#[test]
fn test_cli_subcommand_hierarchy_traversal() {
    let root = CliCommandNode::new("app", "Root")
        .with_subcommand(
            CliCommandNode::new("db", "Database commands")
                .with_subcommand(CliCommandNode::new("migrate", "Run migrations")),
        )
        .with_subcommand(CliCommandNode::new("serve", "Start server"));

    assert_eq!(root.subcommands.len(), 2);
    let db = root.find_subcommand("db").expect("Subcommand 'db' missing");
    assert_eq!(db.name, "db");
    let migrate = db
        .find_subcommand("migrate")
        .expect("Subcommand 'migrate' missing");
    assert_eq!(migrate.name, "migrate");
    assert!(root.find_subcommand("nonexistent").is_none());
}

#[test]
fn test_cli_gap_reason_codes_and_classification() {
    let mut model = CliSemanticModel::new(
        "script",
        "0.1.0",
        "Legacy script",
        CliCommandNode::new("script", "Legacy"),
    );

    model
        .detected_gaps
        .push(CliGapReasonCode::DynamicEvalOrExec);
    model
        .detected_gaps
        .push(CliGapReasonCode::UnmappedPyPiDependency(
            "pandas".to_string(),
        ));

    assert!(model.has_gap(&CliGapReasonCode::DynamicEvalOrExec));
    assert!(model.has_gap(&CliGapReasonCode::UnmappedPyPiDependency(
        "pandas".to_string()
    )));
    assert!(!model.has_gap(&CliGapReasonCode::NativeCExtensionRequired));

    assert_eq!(
        CliGapReasonCode::DynamicEvalOrExec.as_str(),
        "CLI_DYNAMIC_EVAL_OR_EXEC"
    );
    assert_eq!(
        CliGapReasonCode::UnmappedPyPiDependency("pandas".to_string()).to_string(),
        "CLI_UNMAPPED_PYPI_DEPENDENCY: pandas"
    );
}

#[test]
fn test_cli_exit_code_mappings() {
    let exit_codes = CliExitCodeMapping::default();
    assert_eq!(exit_codes.success, 0);
    assert_eq!(exit_codes.argument_error, 1);
    assert_eq!(exit_codes.io_error, 2);
    assert_eq!(exit_codes.data_error, 3);
}

#[test]
fn test_cli_io_contract_defaults() {
    let contract = CliIoContract::default();
    assert!(contract.supports_stdin);
    assert_eq!(contract.default_stdout_format, "json");
    assert!(contract.supported_formats.contains(&"json".to_string()));
    assert!(contract.supported_formats.contains(&"csv".to_string()));
    assert!(contract.stderr_is_diagnostic_only);
}

#[test]
fn test_cli_content_hash_sensitivity_to_command_changes() {
    let base_root = CliCommandNode::new("tool", "CLI Tool").with_argument(CliArgument::flag(
        "verbose",
        Some('v'),
        "Verbose output",
    ));

    let model1 = CliSemanticModel::new("tool", "1.0.0", "CLI Tool", base_root.clone());

    let modified_root = base_root.with_argument(CliArgument::flag("debug", None, "Debug output"));
    let model2 = CliSemanticModel::new("tool", "1.0.0", "CLI Tool", modified_root);

    assert_ne!(model1.content_hash(), model2.content_hash());
}

#[test]
fn test_cli_concurrency_patterns_and_performance_contract() {
    use mae_domain::semantic_cli::{CliConcurrencyPattern, CliPerformanceContract};

    let crunch_cmd = CliCommandNode::new("crunch", "CPU-heavy parallel crunching")
        .with_concurrency(CliConcurrencyPattern::MultiProcessing);

    assert_eq!(
        crunch_cmd.concurrency,
        CliConcurrencyPattern::MultiProcessing
    );
    assert_eq!(crunch_cmd.concurrency.as_str(), "MULTIPROCESSING");

    let root = CliCommandNode::new("dataforge", "CLI").with_subcommand(crunch_cmd);
    let mut model = CliSemanticModel::new("dataforge", "1.4.0", "CLI", root)
        .with_concurrency(CliConcurrencyPattern::MultiProcessing)
        .with_concurrency(CliConcurrencyPattern::ParallelThreads)
        .with_performance_contract(CliPerformanceContract::new(2.0));

    assert!(model.is_concurrent());
    assert_eq!(model.concurrency_patterns.len(), 2);
    assert!(model.performance_contract.is_some());
    assert_eq!(
        model
            .performance_contract
            .as_ref()
            .unwrap()
            .min_speedup_ratio(),
        2.0
    );

    model
        .detected_gaps
        .push(CliGapReasonCode::PythonGilContention);
    assert!(model.has_gap(&CliGapReasonCode::PythonGilContention));
    assert_eq!(
        CliGapReasonCode::PythonGilContention.as_str(),
        "CLI_PYTHON_GIL_CONTENTION"
    );
}

#[test]
fn test_cli_pipeline_capabilities_and_stage_exit_codes() {
    use mae_domain::semantic_cli::CliPipelineCapability;

    let root = CliCommandNode::new("lead_enrich", "Lead enrichment pipeline CLI")
        .with_subcommand(CliCommandNode::new("ingest", "Normalize raw leads"))
        .with_subcommand(CliCommandNode::new(
            "enrich",
            "Enrich leads with cache and retry",
        ))
        .with_subcommand(CliCommandNode::new("report", "Emit pipeline stage report"));

    let base_model = CliSemanticModel::new(
        "lead_enrich",
        "1.0.0",
        "Lead enrichment pipeline CLI",
        root.clone(),
    );
    assert_eq!(base_model.exit_codes.stage_error, 4);
    assert!(base_model.pipeline_capabilities.is_empty());

    let enriched_model = base_model
        .clone()
        .with_pipeline_capability(CliPipelineCapability::StatePersistence)
        .with_pipeline_capability(CliPipelineCapability::CacheLedger)
        .with_pipeline_capability(CliPipelineCapability::RetryPolicy)
        .with_pipeline_capability(CliPipelineCapability::CheckpointResume);

    assert!(enriched_model.has_pipeline_capability(CliPipelineCapability::StatePersistence));
    assert!(enriched_model.has_pipeline_capability(CliPipelineCapability::CacheLedger));
    assert!(enriched_model.has_pipeline_capability(CliPipelineCapability::RetryPolicy));
    assert!(enriched_model.has_pipeline_capability(CliPipelineCapability::CheckpointResume));
    assert_eq!(enriched_model.pipeline_capabilities.len(), 4);
    assert_ne!(base_model.content_hash(), enriched_model.content_hash());
}
