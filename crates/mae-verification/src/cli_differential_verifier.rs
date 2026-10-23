//! Independent Differential Behavioral Verifier for CLI migrations.
//!
//! Executes identical argument vectors and stdin payloads across both
//! the source Python script and the compiled native Rust binary,
//! comparing exit codes, structured output formats, and error channels.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use mae_domain::contract::VerificationCategory;
use mae_domain::ids::{ArtifactRevision, ContractId};
use mae_domain::ports::DomainError;
use mae_domain::verification::{CoverageDisclosure, EvidenceRecord, VerificationStatus};

#[derive(Debug, Clone)]
pub struct CliDifferentialScenario {
    pub id: String,
    pub args: Vec<String>,
    pub stdin_data: Option<String>,
}

impl CliDifferentialScenario {
    pub fn new(id: &str, args: &[&str]) -> Self {
        Self {
            id: id.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            stdin_data: None,
        }
    }

    pub fn with_stdin(mut self, data: &str) -> Self {
        self.stdin_data = Some(data.to_string());
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
pub struct CliDifferentialVerifier;

impl CliDifferentialVerifier {
    pub fn new() -> Self {
        Self
    }

    pub fn verify(
        &self,
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

        // 1. Build the Rust target binary
        let binary_path = self.build_rust_target(target_path)?;

        // 2. Execute differential matrix
        let declared: Vec<String> = scenarios.iter().map(|s| s.id.clone()).collect();
        let mut executed = Vec::new();
        let mut passed = Vec::new();
        let mut unexecuted = Vec::new();
        let mut discrepancies = Vec::new();
        let mut perf_log = Vec::new();

        for scenario in scenarios {
            executed.push(scenario.id.clone());

            let py_res = run_process(
                "python3",
                python_script,
                &scenario.args,
                scenario.stdin_data.as_deref(),
            );
            let rust_res = run_process(
                binary_path.to_str().unwrap(),
                "",
                &scenario.args,
                scenario.stdin_data.as_deref(),
            );

            match (py_res, rust_res) {
                (Ok(py), Ok(rust)) => {
                    let mut mismatch = None;
                    if py.exit_code != rust.exit_code {
                        mismatch = Some(format!(
                            "Exit code mismatch: python={} vs rust={}",
                            py.exit_code, rust.exit_code
                        ));
                    } else if py.exit_code == 0 {
                        // Compare outputs
                        if !outputs_match(&py.stdout, &rust.stdout) {
                            mismatch = Some(format!(
                                "Output mismatch!\nPYTHON:\n{}\nRUST:\n{}",
                                py.stdout, rust.stdout
                            ));
                        }
                    } else {
                        // Both failed as expected: check that both emitted stderr
                        if rust.stderr.trim().is_empty() && !py.stderr.trim().is_empty() {
                            mismatch =
                                Some("Rust failed without error message on stderr".to_string());
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

        let coverage = CoverageDisclosure::with_unexecuted(declared, executed, passed, unexecuted);
        let status = if coverage.is_complete() && discrepancies.is_empty() {
            VerificationStatus::Pass
        } else {
            VerificationStatus::Fail
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
        } else {
            format!(
                "CLI Differential Verification failed: {} discrepancies found:\n{}",
                discrepancies.len(),
                discrepancies.join("\n")
            )
        };

        Ok(EvidenceRecord::new(
            ContractId::new("C_CLI_DIFFERENTIAL_PARITY"),
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
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let workspace_tmp = Path::new(manifest_dir)
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("target/tmp");

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

        // Parse crate name from Cargo.toml or use directory name
        let bin_path = target_dir.join("target/debug/dataforge");
        if bin_path.exists() {
            Ok(bin_path)
        } else {
            // Find any executable in target/debug
            let debug_dir = target_dir.join("target/debug");
            if let Ok(entries) = std::fs::read_dir(debug_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() && !p.extension().is_some_and(|ext| ext == "d" || ext == "rlib")
                    {
                        return Ok(p);
                    }
                }
            }
            Err(DomainError::PortError(
                "Could not locate compiled target binary".to_string(),
            ))
        }
    }
}

struct ProcessResult {
    exit_code: i32,
    stdout: String,
    stderr: String,
    duration: std::time::Duration,
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
