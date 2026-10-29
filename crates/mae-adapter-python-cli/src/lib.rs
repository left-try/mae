//! Python CLI Source Adapter for MAE (Migrate Absolutely Everything).
//!
//! Extracts CLI semantic structure, arguments, commands, exit codes, and capability gaps
//! from Python CLI scripts (argparse, sys.argv, click).

use std::fs;
use std::path::Path;

use mae_domain::artifact::{ArtifactKind, ArtifactProvenance, VersionedArtifact};
use mae_domain::capability::{
    CapabilityAssessment, CapabilityLevel, CapabilityReport, GapReasonCode, MigrationGap,
    MigrationStrategy,
};
use mae_domain::contract::ContractSeverity;
use mae_domain::ids::{ArtifactId, ArtifactRevision};
use mae_domain::ports::{CliSourceAdapterPort, DomainError};
use mae_domain::semantic_cli::{
    CliArgument, CliCommandNode, CliExitCodeMapping, CliGapReasonCode, CliIoContract,
    CliSemanticModel, CliValueType,
};

#[derive(Debug, Clone, Default)]
pub struct PythonCliSourceAdapter;

impl PythonCliSourceAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl CliSourceAdapterPort for PythonCliSourceAdapter {
    fn discover(&self, source_path: &str) -> Result<VersionedArtifact, DomainError> {
        let path = Path::new(source_path);
        if !path.exists() {
            return Err(DomainError::PortError(format!(
                "Python CLI source path not found: {}",
                source_path
            )));
        }

        let content = fs::read(path).map_err(|e| DomainError::PortError(e.to_string()))?;
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("source");

        let artifact = VersionedArtifact::new(
            ArtifactId::new(format!("python-cli-source-{}", stem)),
            ArtifactKind::SourceInventory,
            ArtifactRevision::initial(),
            ArtifactProvenance::static_deterministic(
                "python-cli-adapter",
                ArtifactRevision::initial(),
            ),
            content,
        );

        Ok(artifact)
    }

    fn extract_semantics(&self, source_path: &str) -> Result<CliSemanticModel, DomainError> {
        let content = fs::read_to_string(source_path)
            .map_err(|e| DomainError::PortError(format!("Failed to read source: {}", e)))?;

        // 1. Program name, version, description
        let prog_name = extract_string_param(&content, "prog=")
            .or_else(|| {
                Path::new(source_path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "cli_tool".to_string());

        let version = extract_version(&content).unwrap_or_else(|| "0.1.0".to_string());
        let description = extract_string_param(&content, "description=")
            .or_else(|| extract_module_docstring(&content))
            .unwrap_or_else(|| "Command Line Interface".to_string());

        let mut root_cmd = CliCommandNode::new(&prog_name, &description);

        // 2. Parse commands and arguments
        parse_argparse_commands(&content, &mut root_cmd);

        // 3. Exit codes
        let exit_codes = parse_exit_codes(&content);

        // 4. IO contract
        let io_contract = parse_io_contract(&content);

        // 5. Gap detection
        let detected_gaps = detect_gaps(&content);

        let mut model = CliSemanticModel::new(&prog_name, &version, &description, root_cmd);
        model.exit_codes = exit_codes;
        model.io_contract = io_contract;
        model.detected_gaps = detected_gaps;

        // 6. Concurrency pattern detection
        let mut concurrency_patterns = Vec::new();
        if content.contains("multiprocessing") || content.contains("ProcessPoolExecutor") {
            concurrency_patterns
                .push(mae_domain::semantic_cli::CliConcurrencyPattern::MultiProcessing);
        }
        if content.contains("ThreadPoolExecutor") || content.contains("threading.") {
            concurrency_patterns
                .push(mae_domain::semantic_cli::CliConcurrencyPattern::ParallelThreads);
        }
        if content.contains("asyncio") {
            concurrency_patterns
                .push(mae_domain::semantic_cli::CliConcurrencyPattern::AsyncEventLoop);
        }
        if concurrency_patterns.is_empty() {
            concurrency_patterns.push(mae_domain::semantic_cli::CliConcurrencyPattern::Sequential);
        }
        model.concurrency_patterns = concurrency_patterns;

        if model.is_concurrent() {
            model.performance_contract =
                Some(mae_domain::semantic_cli::CliPerformanceContract::new(1.5));
        }

        // 7. Pipeline operational capabilities (state persistence, cache ledger, retry policy, resume)
        if content.contains("--state-file") || content.contains("state_file") {
            model = model.with_pipeline_capability(
                mae_domain::semantic_cli::CliPipelineCapability::StatePersistence,
            );
        }
        if content.contains("domain_cache") || content.contains("cache_hits") {
            model = model.with_pipeline_capability(
                mae_domain::semantic_cli::CliPipelineCapability::CacheLedger,
            );
        }
        if content.contains("--max-retries") || content.contains("max_retries") {
            model = model.with_pipeline_capability(
                mae_domain::semantic_cli::CliPipelineCapability::RetryPolicy,
            );
        }
        if content.contains("--resume") || content.contains("resumed_skipped") {
            model = model.with_pipeline_capability(
                mae_domain::semantic_cli::CliPipelineCapability::CheckpointResume,
            );
        }

        Ok(model)
    }

    fn assess_capabilities(
        &self,
        model: &CliSemanticModel,
    ) -> Result<CapabilityReport, DomainError> {
        let mut report = CapabilityReport::new();

        report.add_assessment(CapabilityAssessment::new(
            "cli_argument_parsing",
            CapabilityLevel::Supported,
            MigrationStrategy::DirectNativeMapping,
        ));

        report.add_assessment(CapabilityAssessment::new(
            "standard_streams_and_exit_codes",
            CapabilityLevel::Supported,
            MigrationStrategy::DirectNativeMapping,
        ));

        if model.is_concurrent() {
            report.add_assessment(CapabilityAssessment::new(
                "cli_concurrency_scoped_parallelism",
                CapabilityLevel::Supported,
                MigrationStrategy::DirectNativeMapping,
            ));
        }

        for cap in &model.pipeline_capabilities {
            let feature_name = match cap {
                mae_domain::semantic_cli::CliPipelineCapability::StatePersistence => {
                    "cli_pipeline_state_persistence"
                }
                mae_domain::semantic_cli::CliPipelineCapability::CacheLedger => {
                    "cli_pipeline_cache_ledger"
                }
                mae_domain::semantic_cli::CliPipelineCapability::RetryPolicy => {
                    "cli_pipeline_retry_policy"
                }
                mae_domain::semantic_cli::CliPipelineCapability::CheckpointResume => {
                    "cli_pipeline_checkpoint_resume"
                }
            };
            report.add_assessment(CapabilityAssessment::new(
                feature_name,
                CapabilityLevel::Supported,
                MigrationStrategy::DirectNativeMapping,
            ));
        }

        // Report any detected gaps
        for gap in &model.detected_gaps {
            let (level, strategy) = match gap {
                CliGapReasonCode::DynamicEvalOrExec => {
                    (CapabilityLevel::Unsupported, MigrationStrategy::Unsupported)
                }
                CliGapReasonCode::NativeCExtensionRequired => (
                    CapabilityLevel::Partial,
                    MigrationStrategy::ManualAdaptation,
                ),
                _ => (
                    CapabilityLevel::Partial,
                    MigrationStrategy::SemanticReconstruction,
                ),
            };

            report.add_assessment(CapabilityAssessment::new(
                format!("cli_gap_{}", gap.as_str().to_lowercase()),
                level,
                strategy,
            ));

            report.add_gap(MigrationGap::new(
                "source.py",
                GapReasonCode::UiUnresolvedEffect,
                if level == CapabilityLevel::Unsupported {
                    ContractSeverity::Critical
                } else {
                    ContractSeverity::High
                },
                strategy,
                format!("CLI Gap: {}", gap),
            ));
        }

        Ok(report)
    }
}

// ==================== Helper Extractors ====================

fn extract_string_param(text: &str, prefix: &str) -> Option<String> {
    let idx = text.find(prefix)?;
    let rem = text[idx + prefix.len()..].trim_start();
    let quote = rem.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = &rem[1..];
    let end_quote = rest.find(quote)?;
    Some(rest[..end_quote].to_string())
}

fn extract_version(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("VERSION") && trimmed.contains('=') {
            return extract_string_param(trimmed, "=");
        }
    }
    None
}

fn extract_module_docstring(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if let Some(rem) = trimmed.strip_prefix("\"\"\"") {
        if let Some(end) = rem.find("\"\"\"") {
            let doc = rem[..end].trim();
            let first_line = doc.lines().next().unwrap_or(doc);
            return Some(first_line.trim().to_string());
        }
    }
    None
}

fn detect_gaps(text: &str) -> Vec<CliGapReasonCode> {
    let mut gaps = Vec::new();

    if text.contains("eval(") || text.contains("exec(") {
        gaps.push(CliGapReasonCode::DynamicEvalOrExec);
    }
    if text.contains("asyncio") && !text.contains("asyncio.run") {
        gaps.push(CliGapReasonCode::AsyncEventLoopUnsupported);
    }
    if text.contains("importlib") || text.contains("__import__") {
        gaps.push(CliGapReasonCode::DynamicModuleImport);
    }
    if text.contains("ctypes") {
        gaps.push(CliGapReasonCode::NativeCExtensionRequired);
    }
    for line in text.lines() {
        let trimmed = line.trim();
        if (trimmed.starts_with("import ") || trimmed.starts_with("from "))
            && (trimmed.contains("pandas")
                || trimmed.contains("numpy")
                || trimmed.contains("scipy"))
        {
            let pkg = if trimmed.contains("pandas") {
                "pandas"
            } else if trimmed.contains("numpy") {
                "numpy"
            } else {
                "scipy"
            };
            gaps.push(CliGapReasonCode::UnmappedPyPiDependency(pkg.to_string()));
        }
    }

    gaps
}

fn parse_exit_codes(text: &str) -> CliExitCodeMapping {
    let mut mapping = CliExitCodeMapping::default();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("EXIT_SUCCESS") {
            if let Some(val) = parse_int_assignment(trimmed) {
                mapping.success = val;
            }
        } else if trimmed.starts_with("EXIT_ARG_ERROR") {
            if let Some(val) = parse_int_assignment(trimmed) {
                mapping.argument_error = val;
            }
        } else if trimmed.starts_with("EXIT_IO_ERROR") {
            if let Some(val) = parse_int_assignment(trimmed) {
                mapping.io_error = val;
            }
        } else if trimmed.starts_with("EXIT_DATA_ERROR") {
            if let Some(val) = parse_int_assignment(trimmed) {
                mapping.data_error = val;
            }
        } else if trimmed.starts_with("EXIT_STAGE_ERROR") {
            if let Some(val) = parse_int_assignment(trimmed) {
                mapping.stage_error = val;
            }
        }
    }
    mapping
}

fn parse_int_assignment(line: &str) -> Option<i32> {
    let parts: Vec<&str> = line.split('=').collect();
    if parts.len() == 2 {
        parts[1].trim().parse::<i32>().ok()
    } else {
        None
    }
}

fn parse_io_contract(text: &str) -> CliIoContract {
    let supports_stdin = text.contains("stdin") || text.contains("sys.stdin");
    let default_stdout_format = "json".to_string();

    CliIoContract {
        supports_stdin,
        default_stdout_format,
        supported_formats: vec!["json".to_string(), "csv".to_string()],
        stderr_is_diagnostic_only: text.contains("stderr"),
    }
}

fn parse_argparse_commands(content: &str, root_cmd: &mut CliCommandNode) {
    let lines: Vec<&str> = content.lines().collect();
    let mut subcommands_map: Vec<(String, CliCommandNode)> = Vec::new();

    for line in &lines {
        let trimmed = line.trim();

        // 1. Root global flags/arguments on `parser.add_argument(...)`
        if trimmed.starts_with("parser.add_argument(") {
            if let Some(arg) = parse_add_argument(trimmed) {
                root_cmd.arguments.push(arg);
            }
        }

        // 2. Subparser definition e.g. `p_stats = subparsers.add_parser("stats", help="...")`
        if trimmed.contains("subparsers.add_parser(") {
            let var_name = trimmed.split('=').next().unwrap_or("").trim().to_string();
            if let Some(sub_name) = extract_first_quoted_arg(trimmed) {
                let desc = extract_string_param(trimmed, "help=").unwrap_or_default();
                let sub_cmd = CliCommandNode::new(&sub_name, &desc);
                subcommands_map.push((var_name, sub_cmd));
            }
        }

        // 3. Subcommand argument e.g. `p_stats.add_argument(...)`
        for (var_name, sub_cmd) in &mut subcommands_map {
            let prefix = format!("{}.add_argument(", var_name);
            if trimmed.starts_with(&prefix) {
                if let Some(arg) = parse_add_argument(trimmed) {
                    sub_cmd.arguments.push(arg);
                }
            }
        }
    }

    // Attach parsed subcommands to root command
    for (_, sub_cmd) in subcommands_map {
        root_cmd.subcommands.push(sub_cmd);
    }
}

fn extract_first_quoted_arg(line: &str) -> Option<String> {
    let open_paren = line.find('(')?;
    let rem = &line[open_paren + 1..];
    let quote_idx = rem.find(['"', '\''])?;
    let quote = rem.chars().nth(quote_idx)?;
    let after_quote = &rem[quote_idx + 1..];
    let end_quote = after_quote.find(quote)?;
    Some(after_quote[..end_quote].to_string())
}

fn split_args(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quote: Option<char> = None;
    let mut bracket_depth = 0;
    let mut paren_depth = 0;

    for c in s.chars() {
        if let Some(q) = in_quote {
            if c == q {
                in_quote = None;
            }
            current.push(c);
        } else if c == '"' || c == '\'' {
            in_quote = Some(c);
            current.push(c);
        } else if c == '[' {
            bracket_depth += 1;
            current.push(c);
        } else if c == ']' {
            if bracket_depth > 0 {
                bracket_depth -= 1;
            }
            current.push(c);
        } else if c == '(' {
            paren_depth += 1;
            current.push(c);
        } else if c == ')' {
            if paren_depth > 0 {
                paren_depth -= 1;
            }
            current.push(c);
        } else if c == ',' && bracket_depth == 0 && paren_depth == 0 {
            parts.push(current.trim().to_string());
            current.clear();
        } else {
            current.push(c);
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }
    parts
}

fn parse_add_argument(call: &str) -> Option<CliArgument> {
    let mut short_flag: Option<char> = None;
    let mut long_flag: Option<String> = None;
    let mut positional_name: Option<String> = None;

    let open = call.find('(')?;
    let close = call.rfind(')')?;
    let inner = &call[open + 1..close];

    for piece in split_args(inner) {
        let p = piece.trim();
        if p.starts_with("\"--") || p.starts_with("'--") {
            long_flag = Some(p.trim_matches(|c| c == '"' || c == '\'')[2..].to_string());
        } else if p.starts_with("\"-") || p.starts_with("'-") {
            let flag_str = p.trim_matches(|c| c == '"' || c == '\'');
            if flag_str.len() == 2 {
                short_flag = flag_str.chars().nth(1);
            }
        } else if (p.starts_with('"') || p.starts_with('\'')) && !p.contains('=') {
            positional_name = Some(p.trim_matches(|c| c == '"' || c == '\'').to_string());
        }
    }

    let help = extract_string_param(inner, "help=").unwrap_or_default();
    let is_flag = inner.contains("action=\"store_true\"")
        || inner.contains("action='store_true'")
        || inner.contains("store_false");

    let is_version = inner.contains("action=\"version\"") || inner.contains("action='version'");
    if is_version {
        return Some(CliArgument::flag(
            long_flag.as_deref().unwrap_or("version"),
            short_flag,
            "Show version",
        ));
    }

    let value_type = if inner.contains("type=int") {
        CliValueType::Integer
    } else if inner.contains("type=float") {
        CliValueType::Float
    } else if inner.contains("choices=") {
        let mut choices = Vec::new();
        if let Some(choices_idx) = inner.find("choices=") {
            let rem = &inner[choices_idx + 8..];
            if let Some(start_bracket) = rem.find('[') {
                if let Some(end_bracket) = rem.find(']') {
                    let choices_str = &rem[start_bracket + 1..end_bracket];
                    for c in choices_str.split(',') {
                        choices.push(
                            c.trim()
                                .trim_matches(|ch| ch == '"' || ch == '\'')
                                .to_string(),
                        );
                    }
                }
            }
        }
        CliValueType::EnumChoices(choices)
    } else if is_flag {
        CliValueType::Boolean
    } else {
        CliValueType::String
    };

    if let Some(pos) = positional_name {
        let required = !inner.contains("nargs=\"?\"") && !inner.contains("nargs='?'");
        Some(CliArgument::positional(&pos, required, &help))
    } else if let Some(long) = long_flag {
        if is_flag {
            Some(CliArgument::flag(&long, short_flag, &help))
        } else {
            let mut arg = CliArgument::option(&long, &long, short_flag, value_type, &help);
            if let Some(def_val) = extract_default_param(inner) {
                arg.default_value = Some(def_val);
            }
            Some(arg)
        }
    } else {
        None
    }
}

fn extract_default_param(inner: &str) -> Option<String> {
    let idx = inner.find("default=")?;
    let rem = &inner[idx + 8..];
    let end = rem.find(',').unwrap_or(rem.len());
    let def = rem[..end].trim();
    Some(def.trim_matches(|c| c == '"' || c == '\'').to_string())
}
