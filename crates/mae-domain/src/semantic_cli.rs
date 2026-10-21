use std::fmt;

use crate::ids::ContentHash;

/// Reason codes identifying why a Python CLI construct cannot be directly mapped or requires manual adaptation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CliGapReasonCode {
    DynamicEvalOrExec,
    AsyncEventLoopUnsupported,
    DynamicModuleImport,
    NativeCExtensionRequired,
    UnmappedPyPiDependency(String),
    InteractiveTerminalPrompt,
    SignalsOrPipesPlatformSpecific,
    PythonGilContention,
}

impl CliGapReasonCode {
    pub fn as_str(&self) -> &str {
        match self {
            Self::DynamicEvalOrExec => "CLI_DYNAMIC_EVAL_OR_EXEC",
            Self::AsyncEventLoopUnsupported => "CLI_ASYNC_EVENT_LOOP_UNSUPPORTED",
            Self::DynamicModuleImport => "CLI_DYNAMIC_MODULE_IMPORT",
            Self::NativeCExtensionRequired => "CLI_NATIVE_C_EXTENSION_REQUIRED",
            Self::UnmappedPyPiDependency(_) => "CLI_UNMAPPED_PYPI_DEPENDENCY",
            Self::InteractiveTerminalPrompt => "CLI_INTERACTIVE_TERMINAL_PROMPT",
            Self::SignalsOrPipesPlatformSpecific => "CLI_SIGNALS_OR_PIPES_PLATFORM_SPECIFIC",
            Self::PythonGilContention => "CLI_PYTHON_GIL_CONTENTION",
        }
    }
}

impl fmt::Display for CliGapReasonCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnmappedPyPiDependency(pkg) => {
                write!(f, "CLI_UNMAPPED_PYPI_DEPENDENCY: {}", pkg)
            }
            other => write!(f, "{}", other.as_str()),
        }
    }
}

/// Category of argument passed to a CLI command.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CliArgumentKind {
    Positional { required: bool },
    NamedOption { short: Option<char>, long: String },
    Flag { short: Option<char>, long: String },
}

/// Primitive type constraints for CLI arguments.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CliValueType {
    String,
    Integer,
    Float,
    Boolean,
    Path,
    EnumChoices(Vec<String>),
}

/// Definition of an argument, flag, or option accepted by a command.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CliArgument {
    pub name: String,
    pub kind: CliArgumentKind,
    pub value_type: CliValueType,
    pub help: String,
    pub default_value: Option<String>,
    pub env_var_fallback: Option<String>,
}

impl CliArgument {
    pub fn positional(name: &str, required: bool, help: &str) -> Self {
        Self {
            name: name.to_string(),
            kind: CliArgumentKind::Positional { required },
            value_type: CliValueType::String,
            help: help.to_string(),
            default_value: None,
            env_var_fallback: None,
        }
    }

    pub fn flag(long: &str, short: Option<char>, help: &str) -> Self {
        Self {
            name: long.to_string(),
            kind: CliArgumentKind::Flag {
                short,
                long: long.to_string(),
            },
            value_type: CliValueType::Boolean,
            help: help.to_string(),
            default_value: Some("false".to_string()),
            env_var_fallback: None,
        }
    }

    pub fn option(
        name: &str,
        long: &str,
        short: Option<char>,
        value_type: CliValueType,
        help: &str,
    ) -> Self {
        Self {
            name: name.to_string(),
            kind: CliArgumentKind::NamedOption {
                short,
                long: long.to_string(),
            },
            value_type,
            help: help.to_string(),
            default_value: None,
            env_var_fallback: None,
        }
    }
}

/// Concurrency pattern employed by a CLI command or subsystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CliConcurrencyPattern {
    Sequential,
    ParallelThreads,
    MultiProcessing,
    AsyncEventLoop,
}

impl CliConcurrencyPattern {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sequential => "SEQUENTIAL",
            Self::ParallelThreads => "PARALLEL_THREADS",
            Self::MultiProcessing => "MULTIPROCESSING",
            Self::AsyncEventLoop => "ASYNC_EVENT_LOOP",
        }
    }
}

/// Target performance contract asserting acceleration requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CliPerformanceContract {
    pub min_speedup_basis_points: u32, // 100 = 1.0x, 150 = 1.5x, 200 = 2.0x
    pub max_duration_ms: Option<u64>,
}

impl CliPerformanceContract {
    pub fn new(speedup_ratio: f64) -> Self {
        Self {
            min_speedup_basis_points: (speedup_ratio * 100.0).round() as u32,
            max_duration_ms: None,
        }
    }

    pub fn min_speedup_ratio(&self) -> f64 {
        self.min_speedup_basis_points as f64 / 100.0
    }
}

impl Default for CliPerformanceContract {
    fn default() -> Self {
        Self {
            min_speedup_basis_points: 150,
            max_duration_ms: None,
        }
    }
}

/// Node representing an executable CLI command or nested subcommand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliCommandNode {
    pub name: String,
    pub description: String,
    pub arguments: Vec<CliArgument>,
    pub subcommands: Vec<CliCommandNode>,
    pub concurrency: CliConcurrencyPattern,
}

impl CliCommandNode {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            arguments: Vec::new(),
            subcommands: Vec::new(),
            concurrency: CliConcurrencyPattern::Sequential,
        }
    }

    pub fn with_concurrency(mut self, pattern: CliConcurrencyPattern) -> Self {
        self.concurrency = pattern;
        self
    }

    pub fn with_argument(mut self, arg: CliArgument) -> Self {
        self.arguments.push(arg);
        self
    }

    pub fn with_subcommand(mut self, sub: CliCommandNode) -> Self {
        self.subcommands.push(sub);
        self
    }

    pub fn find_subcommand(&self, name: &str) -> Option<&CliCommandNode> {
        self.subcommands.iter().find(|s| s.name == name)
    }
}

/// Contract for standard I/O behavior and stream formats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliIoContract {
    pub supports_stdin: bool,
    pub default_stdout_format: String,
    pub supported_formats: Vec<String>,
    pub stderr_is_diagnostic_only: bool,
}

impl Default for CliIoContract {
    fn default() -> Self {
        Self {
            supports_stdin: true,
            default_stdout_format: "json".to_string(),
            supported_formats: vec!["json".to_string(), "csv".to_string()],
            stderr_is_diagnostic_only: true,
        }
    }
}

/// Definition of deterministic exit codes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliExitCodeMapping {
    pub success: i32,
    pub argument_error: i32,
    pub io_error: i32,
    pub data_error: i32,
}

impl Default for CliExitCodeMapping {
    fn default() -> Self {
        Self {
            success: 0,
            argument_error: 1,
            io_error: 2,
            data_error: 3,
        }
    }
}

/// Pure domain IR model capturing complete semantics of a CLI application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliSemanticModel {
    pub program_name: String,
    pub version: String,
    pub description: String,
    pub root_command: CliCommandNode,
    pub io_contract: CliIoContract,
    pub exit_codes: CliExitCodeMapping,
    pub detected_framework: String,
    pub detected_gaps: Vec<CliGapReasonCode>,
    pub concurrency_patterns: Vec<CliConcurrencyPattern>,
    pub performance_contract: Option<CliPerformanceContract>,
}

impl CliSemanticModel {
    pub fn new(
        program_name: &str,
        version: &str,
        description: &str,
        root_command: CliCommandNode,
    ) -> Self {
        Self {
            program_name: program_name.to_string(),
            version: version.to_string(),
            description: description.to_string(),
            root_command,
            io_contract: CliIoContract::default(),
            exit_codes: CliExitCodeMapping::default(),
            detected_framework: "argparse".to_string(),
            detected_gaps: Vec::new(),
            concurrency_patterns: Vec::new(),
            performance_contract: None,
        }
    }

    pub fn with_concurrency(mut self, pattern: CliConcurrencyPattern) -> Self {
        if !self.concurrency_patterns.contains(&pattern) {
            self.concurrency_patterns.push(pattern);
        }
        self
    }

    pub fn with_performance_contract(mut self, contract: CliPerformanceContract) -> Self {
        self.performance_contract = Some(contract);
        self
    }

    pub fn is_concurrent(&self) -> bool {
        self.concurrency_patterns
            .iter()
            .any(|p| *p != CliConcurrencyPattern::Sequential)
    }

    /// Computes a deterministic content hash for this CLI semantic model.
    pub fn content_hash(&self) -> ContentHash {
        let mut canonical_repr = format!(
            "prog={}|ver={}|desc={}|framework={}|stdin={}|stdout_fmt={}|exit=[{},{},{},{}]",
            self.program_name,
            self.version,
            self.description,
            self.detected_framework,
            self.io_contract.supports_stdin,
            self.io_contract.default_stdout_format,
            self.exit_codes.success,
            self.exit_codes.argument_error,
            self.exit_codes.io_error,
            self.exit_codes.data_error
        );

        fn serialize_command(cmd: &CliCommandNode, out: &mut String) {
            out.push_str(&format!(";cmd={}:desc={}", cmd.name, cmd.description));
            for arg in &cmd.arguments {
                out.push_str(&format!(";arg={}:def={:?}", arg.name, arg.default_value));
            }
            for sub in &cmd.subcommands {
                serialize_command(sub, out);
            }
        }

        serialize_command(&self.root_command, &mut canonical_repr);
        ContentHash::compute(canonical_repr.as_bytes())
    }

    pub fn total_subcommands(&self) -> usize {
        self.root_command.subcommands.len()
    }

    pub fn has_gap(&self, gap: &CliGapReasonCode) -> bool {
        self.detected_gaps.contains(gap)
    }
}
