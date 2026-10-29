//! Independent Differential Behavioral Verifier for CLI migrations.
//!
//! Executes identical argument vectors and stdin payloads across both
//! the source Python script and the compiled native Rust binary,
//! comparing exit codes, structured output formats, and error channels.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use mae_domain::contract::{Contract, VerificationCategory};
use mae_domain::ids::{ArtifactRevision, ContractId};
use mae_domain::ports::{DomainError, VerifierPluginPort};
use mae_domain::verification::{CoverageDisclosure, EvidenceRecord, VerificationStatus};

#[derive(Debug, Clone)]
pub struct CliDifferentialScenario {
    pub id: String,
    pub args: Vec<String>,
    pub stdin_data: Option<String>,
    pub expected_exit_code: Option<i32>,
    pub expected_stderr_substrings: Vec<String>,
    pub compared_files: Vec<String>,
    pub excluded_reason: Option<String>,
}

impl CliDifferentialScenario {
    pub fn new(id: &str, args: &[&str]) -> Self {
        Self {
            id: id.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            stdin_data: None,
            expected_exit_code: None,
            expected_stderr_substrings: Vec::new(),
            compared_files: Vec::new(),
            excluded_reason: None,
        }
    }

    pub fn with_stdin(mut self, data: &str) -> Self {
        self.stdin_data = Some(data.to_string());
        self
    }

    pub fn with_expected_exit_code(mut self, code: i32) -> Self {
        self.expected_exit_code = Some(code);
        self
    }

    pub fn with_expected_stderr_contains(mut self, substring: &str) -> Self {
        self.expected_stderr_substrings.push(substring.to_string());
        self
    }

    pub fn with_compared_file(mut self, relative_path: &str) -> Self {
        self.compared_files.push(relative_path.to_string());
        self
    }

    pub fn with_excluded_reason(mut self, reason: &str) -> Self {
        self.excluded_reason = Some(reason.to_string());
        self
    }
}

#[derive(Debug, Clone)]
pub struct CliScenarioResult {
    pub scenario_id: String,
    pub passed: bool,
    pub python_exit_code: i32,
    pub rust_exit_code: i32,
    pub discrepancy: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct CliDifferentialVerifier {
    configured_python_script: Option<String>,
    configured_rust_target_dir: Option<String>,
    configured_scenarios: Vec<CliDifferentialScenario>,
}

impl CliDifferentialVerifier {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_pipeline(
        python_script: impl Into<String>,
        rust_target_dir: impl Into<String>,
        scenarios: Vec<CliDifferentialScenario>,
    ) -> Self {
        Self {
            configured_python_script: Some(python_script.into()),
            configured_rust_target_dir: Some(rust_target_dir.into()),
            configured_scenarios: scenarios,
        }
    }

    pub fn verify(
        &self,
        python_script: &str,
        rust_target_dir: &str,
        scenarios: &[CliDifferentialScenario],
        target_rev: ArtifactRevision,
    ) -> Result<EvidenceRecord, DomainError> {
        self.verify_for_contract(
            ContractId::new("C_CLI_DIFFERENTIAL_PARITY"),
            python_script,
            rust_target_dir,
            scenarios,
            target_rev,
        )
    }

    pub fn verify_for_contract(
        &self,
        contract_id: ContractId,
        python_script: &str,
        rust_target_dir: &str,
        scenarios: &[CliDifferentialScenario],
        target_rev: ArtifactRevision,
    ) -> Result<EvidenceRecord, DomainError> {
        let target_path = Path::new(rust_target_dir);
        let cargo_toml = target_path.join("Cargo.toml");
        if !cargo_toml.exists() {
            return Err(DomainError::PortError(format!(
                "Cargo.toml not found in target dir: {}",
                rust_target_dir
            )));
        }

        // 1. Build the Rust target binary (removing any stale binary first)
        let binary_path = self.build_rust_target(target_path)?;

        // Prepare isolated per-run filesystem output directories for {OUT_DIR} placeholders
        let py_out_dir = target_path.join(".mae_diff_py_out");
        let rust_out_dir = target_path.join(".mae_diff_rust_out");
        let _ = fs::remove_dir_all(&py_out_dir);
        let _ = fs::remove_dir_all(&rust_out_dir);
        fs::create_dir_all(&py_out_dir).map_err(|e| DomainError::PortError(e.to_string()))?;
        fs::create_dir_all(&rust_out_dir).map_err(|e| DomainError::PortError(e.to_string()))?;

        // 2. Execute differential matrix
        let declared: Vec<String> = scenarios.iter().map(|s| s.id.clone()).collect();
        let mut executed = Vec::new();
        let mut passed = Vec::new();
        let mut unexecuted = Vec::new();
        let mut discrepancies = Vec::new();
        let mut perf_log = Vec::new();

        for scenario in scenarios {
            if let Some(ref reason) = scenario.excluded_reason {
                unexecuted.push(scenario.id.clone());
                perf_log.push(format!("{}: EXCLUDED ({})", scenario.id, reason));
                continue;
            }

            executed.push(scenario.id.clone());

            let py_args: Vec<String> = scenario
                .args
                .iter()
                .map(|a| a.replace("{OUT_DIR}", py_out_dir.to_str().unwrap()))
                .collect();
            let rust_args: Vec<String> = scenario
                .args
                .iter()
                .map(|a| a.replace("{OUT_DIR}", rust_out_dir.to_str().unwrap()))
                .collect();

            let py_res =
                run_python_process(python_script, &py_args, scenario.stdin_data.as_deref());
            let rust_res = run_process(
                binary_path.to_str().unwrap(),
                "",
                &rust_args,
                scenario.stdin_data.as_deref(),
            );

            match (py_res, rust_res) {
                (Ok(py), Ok(rust)) => {
                    let mut mismatch = None;
                    if py.exit_code != rust.exit_code {
                        mismatch = Some(format!(
                            "Exit code mismatch: python={} vs rust={}\nPY STDERR: {}\nRUST STDERR: {}",
                            py.exit_code, rust.exit_code, py.stderr, rust.stderr
                        ));
                    } else if let Some(exp_code) = scenario.expected_exit_code {
                        if rust.exit_code != exp_code {
                            mismatch = Some(format!(
                                "Expected exit code {}, but both exited with {}",
                                exp_code, rust.exit_code
                            ));
                        }
                    }

                    if mismatch.is_none() && py.exit_code == 0 {
                        if !outputs_match(&py.stdout, &rust.stdout) {
                            mismatch = Some(format!(
                                "Output mismatch!\nPYTHON:\n{}\nRUST:\n{}",
                                py.stdout, rust.stdout
                            ));
                        }
                    } else if mismatch.is_none()
                        && py.exit_code != 0
                        && rust.stderr.trim().is_empty()
                        && !py.stderr.trim().is_empty()
                    {
                        mismatch = Some("Rust failed without error message on stderr".to_string());
                    }

                    if mismatch.is_none() {
                        for expected_sub in &scenario.expected_stderr_substrings {
                            if !py.stderr.contains(expected_sub)
                                || !rust.stderr.contains(expected_sub)
                            {
                                mismatch = Some(format!(
                                    "Missing expected stderr diagnostic '{}'\nPY STDERR: {}\nRUST STDERR: {}",
                                    expected_sub, py.stderr, rust.stderr
                                ));
                                break;
                            }
                        }
                    }

                    if mismatch.is_none() {
                        for rel_file in &scenario.compared_files {
                            let py_file = py_out_dir.join(rel_file);
                            let rust_file = rust_out_dir.join(rel_file);
                            match (fs::read_to_string(&py_file), fs::read_to_string(&rust_file)) {
                                (Ok(py_content), Ok(rust_content)) => {
                                    if !outputs_match(&py_content, &rust_content) {
                                        mismatch = Some(format!(
                                            "Filesystem artifact '{}' mismatch!\nPYTHON:\n{}\nRUST:\n{}",
                                            rel_file, py_content, rust_content
                                        ));
                                        break;
                                    }
                                }
                                (Err(e), _) => {
                                    mismatch = Some(format!(
                                        "Python did not produce expected file '{}': {}",
                                        rel_file, e
                                    ));
                                    break;
                                }
                                (_, Err(e)) => {
                                    mismatch = Some(format!(
                                        "Rust did not produce expected file '{}': {}",
                                        rel_file, e
                                    ));
                                    break;
                                }
                            }
                        }
                    }

                    if let Some(err) = mismatch {
                        discrepancies.push(format!("[{}] {}", scenario.id, err));
                    } else {
                        passed.push(scenario.id.clone());
                        let speedup =
                            py.duration.as_secs_f64() / rust.duration.as_secs_f64().max(0.0001);
                        perf_log.push(format!(
                            "{}: py={:.1}ms rust={:.1}ms ({:.1}x)",
                            scenario.id,
                            py.duration.as_secs_f64() * 1000.0,
                            rust.duration.as_secs_f64() * 1000.0,
                            speedup
                        ));
                    }
                }
                (Err(e), _) => {
                    unexecuted.push(scenario.id.clone());
                    discrepancies.push(format!("[{}] Python execution failed: {}", scenario.id, e));
                }
                (_, Err(e)) => {
                    discrepancies.push(format!("[{}] Rust execution failed: {}", scenario.id, e));
                }
            }
        }

        let _ = fs::remove_dir_all(&py_out_dir);
        let _ = fs::remove_dir_all(&rust_out_dir);

        let coverage = CoverageDisclosure::with_unexecuted(declared, executed, passed, unexecuted);
        let status = if !discrepancies.is_empty() {
            VerificationStatus::Fail
        } else if !coverage.is_complete() {
            VerificationStatus::Inconclusive
        } else {
            VerificationStatus::Pass
        };

        let summary = if status.is_pass() {
            let perf_summary = if !perf_log.is_empty() {
                format!("\nPerformance & Acceleration:\n  {}", perf_log.join("\n  "))
            } else {
                String::new()
            };
            format!(
                "CLI Differential Verification passed: {}/{} scenarios matched exactly{}",
                coverage.passed.len(),
                coverage.declared_scenarios.len(),
                perf_summary
            )
        } else if status == VerificationStatus::Inconclusive {
            format!(
                "CLI Differential Verification inconclusive: {}/{} scenarios passed, {} unexecuted ({:?})",
                coverage.passed.len(),
                coverage.declared_scenarios.len(),
                coverage.unexecuted.len(),
                coverage.unexecuted
            )
        } else {
            format!(
                "CLI Differential Verification failed: {} discrepancies found:\n{}",
                discrepancies.len(),
                discrepancies.join("\n")
            )
        };

        Ok(EvidenceRecord::new(
            contract_id,
            "CliDifferentialVerifier-v1",
            VerificationCategory::Behavioral,
            ArtifactRevision::initial(),
            target_rev,
            status,
            coverage,
            summary,
        ))
    }

    fn build_rust_target(&self, target_dir: &Path) -> Result<PathBuf, DomainError> {
        let cargo_toml_path = target_dir.join("Cargo.toml");
        let crate_name =
            parse_crate_name(&cargo_toml_path).unwrap_or_else(|| "dataforge".to_string());
        let bin_filename = format!("{}{}", crate_name, std::env::consts::EXE_SUFFIX);
        let expected_bin_path = target_dir.join("target/debug").join(&bin_filename);

        // Remove any prior compiled binary before building so a broken or partial generation
        // can never fall back to a stale executable from an earlier ArtifactRevision.
        if expected_bin_path.exists() {
            let _ = fs::remove_file(&expected_bin_path);
        }

        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let workspace_tmp = Path::new(manifest_dir)
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("target/tmp");
        let _ = fs::create_dir_all(&workspace_tmp);

        let output = Command::new("cargo")
            .arg("build")
            .current_dir(target_dir)
            .env("TMPDIR", workspace_tmp)
            .output()
            .map_err(|e| DomainError::PortError(format!("Failed running cargo build: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DomainError::PortError(format!(
                "Target cargo build failed:\n{stderr}"
            )));
        }

        if expected_bin_path.exists() {
            Ok(expected_bin_path)
        } else {
            Err(DomainError::PortError(format!(
                "Could not locate compiled target binary '{}' in target/debug",
                bin_filename
            )))
        }
    }
}

impl VerifierPluginPort for CliDifferentialVerifier {
    fn category(&self) -> VerificationCategory {
        VerificationCategory::Behavioral
    }

    fn verify(
        &self,
        contract: &Contract,
        target_rev: ArtifactRevision,
    ) -> Result<EvidenceRecord, DomainError> {
        let py_script = self.configured_python_script.as_deref().ok_or_else(|| {
            DomainError::PortError(
                "CliDifferentialVerifier missing configured python_script".into(),
            )
        })?;
        let rust_target_dir = self.configured_rust_target_dir.as_deref().ok_or_else(|| {
            DomainError::PortError(
                "CliDifferentialVerifier missing configured rust_target_dir".into(),
            )
        })?;
        self.verify_for_contract(
            contract.id.clone(),
            py_script,
            rust_target_dir,
            &self.configured_scenarios,
            target_rev,
        )
    }
}

fn parse_crate_name(cargo_toml_path: &Path) -> Option<String> {
    let content = fs::read_to_string(cargo_toml_path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("name") {
            let after_eq = rest.trim_start().strip_prefix('=')?.trim();
            return Some(after_eq.trim_matches('"').to_string());
        }
    }
    None
}

struct ProcessResult {
    exit_code: i32,
    stdout: String,
    stderr: String,
    duration: std::time::Duration,
}

fn run_python_process(
    script_arg: &str,
    args: &[String],
    stdin_data: Option<&str>,
) -> Result<ProcessResult, String> {
    run_process("python3", script_arg, args, stdin_data)
        .or_else(|_| run_process("python", script_arg, args, stdin_data))
}

fn run_process(
    cmd: &str,
    script_arg: &str,
    args: &[String],
    stdin_data: Option<&str>,
) -> Result<ProcessResult, String> {
    let t0 = std::time::Instant::now();
    let mut command = Command::new(cmd);
    if !script_arg.is_empty() {
        command.arg(script_arg);
    }
    for a in args {
        command.arg(a);
    }

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    if stdin_data.is_some() {
        command.stdin(Stdio::piped());
    } else {
        command.stdin(Stdio::null());
    }

    let mut child = command.spawn().map_err(|e| e.to_string())?;

    if let Some(data) = stdin_data {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(data.as_bytes());
        }
    }

    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    let duration = t0.elapsed();
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok(ProcessResult {
        exit_code: code,
        stdout,
        stderr,
        duration,
    })
}

fn outputs_match(out1: &str, out2: &str) -> bool {
    let t1 = out1.trim();
    let t2 = out2.trim();
    if t1 == t2 {
        return true;
    }

    // Strip whitespace differences for JSON output comparison
    let norm1 = t1
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>();
    let norm2 = t2
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>();
    norm1 == norm2
}
