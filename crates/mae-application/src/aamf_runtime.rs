use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use mae_domain::{
    AgentBackendPort, AgentResult, ArtifactRevision, ContentHash, DomainError, MigrationAggregate,
    MigrationId, MigrationTask, PlanHash, PromptProvenance, TaskId, TaskState,
};

/// Execution mode for task scheduling in the AAMF runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    /// Execute each task immediately as its dependencies complete.
    PerTask,
    /// Execute tasks in synchronized topological wave barriers.
    WaveBarrier,
}

/// Topological wave scheduler grouping tasks into parallel dependency frontiers.
pub struct WaveSchedule;

impl WaveSchedule {
    /// Computes parallel waves of task IDs based on their dependency relationships.
    ///
    /// Tasks in Wave 0 have no dependencies.
    /// Tasks in Wave k depend only on tasks in Waves 0..k-1.
    ///
    /// Returns an error if a dependency cycle or unresolvable dependency is detected.
    pub fn compute_waves(tasks: &[MigrationTask]) -> Result<Vec<Vec<TaskId>>, DomainError> {
        if tasks.is_empty() {
            return Ok(Vec::new());
        }

        let all_task_ids: HashSet<TaskId> = tasks.iter().map(|t| t.id.clone()).collect();
        let mut remaining: BTreeMap<TaskId, &MigrationTask> =
            tasks.iter().map(|t| (t.id.clone(), t)).collect();
        let mut scheduled: HashSet<TaskId> = HashSet::new();
        let mut waves: Vec<Vec<TaskId>> = Vec::new();

        while !remaining.is_empty() {
            let mut current_wave: Vec<TaskId> = Vec::new();

            for (task_id, task) in &remaining {
                let deps_met = task.dependencies.iter().all(|dep| {
                    if all_task_ids.contains(dep) {
                        scheduled.contains(dep)
                    } else {
                        // External or pre-existing dependencies are considered satisfied
                        true
                    }
                });

                if deps_met {
                    current_wave.push(task_id.clone());
                }
            }

            if current_wave.is_empty() {
                let remaining_ids: Vec<String> =
                    remaining.keys().map(|k| k.as_str().to_string()).collect();
                return Err(DomainError::PortError(format!(
                    "Dependency cycle or unresolvable dependency detected among tasks: {}",
                    remaining_ids.join(", ")
                )));
            }

            // Sort wave deterministically by TaskId
            current_wave.sort();

            for tid in &current_wave {
                scheduled.insert(tid.clone());
                remaining.remove(tid);
            }

            waves.push(current_wave);
        }

        Ok(waves)
    }
}

/// Serializable snapshot capturing migration aggregate state for checkpoint/resume.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointSnapshot {
    pub migration_id: MigrationId,
    pub plan_hash: Option<PlanHash>,
    pub current_target_revision: ArtifactRevision,
    pub task_states: Vec<(TaskId, TaskState, u32)>,
    pub accepted_tasks: Vec<TaskId>,
}

impl CheckpointSnapshot {
    /// Captures a snapshot from a live `MigrationAggregate`.
    pub fn from_aggregate(aggregate: &MigrationAggregate) -> Self {
        let mut task_states: Vec<(TaskId, TaskState, u32)> = aggregate
            .tasks
            .values()
            .map(|t| (t.id.clone(), t.state, t.retry_count))
            .collect();
        task_states.sort_by(|a, b| a.0.cmp(&b.0));

        let mut accepted_tasks: Vec<TaskId> = aggregate
            .tasks
            .values()
            .filter(|t| t.state.is_accepted())
            .map(|t| t.id.clone())
            .collect();
        accepted_tasks.sort();

        Self {
            migration_id: aggregate.id.clone(),
            plan_hash: aggregate.plan_hash.clone(),
            current_target_revision: aggregate.current_target_revision,
            task_states,
            accepted_tasks,
        }
    }

    /// Resumes and updates a `MigrationAggregate` from this snapshot.
    pub fn apply_to_aggregate(
        &self,
        aggregate: &mut MigrationAggregate,
    ) -> Result<(), DomainError> {
        aggregate.current_target_revision = self.current_target_revision;
        if let Some(ref ph) = self.plan_hash {
            aggregate.plan_hash = Some(ph.clone());
        }

        for (task_id, state, retry_count) in &self.task_states {
            if let Some(task) = aggregate.get_task_mut(task_id) {
                task.state = *state;
                task.retry_count = *retry_count;
            }
        }

        Ok(())
    }
}

/// JSON parser and serializer for CheckpointSnapshots using pure `std`.
pub struct CheckpointManager;

impl CheckpointManager {
    /// Serializes a `CheckpointSnapshot` to deterministic JSON string.
    pub fn serialize(snapshot: &CheckpointSnapshot) -> String {
        let plan_hash_str = match &snapshot.plan_hash {
            Some(ph) => format!("\"{}\"", ph.as_str()),
            None => "null".to_string(),
        };

        let mut tasks_json = Vec::new();
        for (id, state, retries) in &snapshot.task_states {
            tasks_json.push(format!(
                "    {{\"task_id\": \"{}\", \"state\": \"{}\", \"retry_count\": {}}}",
                id.as_str(),
                state.as_str(),
                retries
            ));
        }

        let mut accepted_json = Vec::new();
        for id in &snapshot.accepted_tasks {
            accepted_json.push(format!("    \"{}\"", id.as_str()));
        }

        format!(
            "{{\n  \"migration_id\": \"{}\",\n  \"plan_hash\": {},\n  \"current_target_revision\": {},\n  \"task_states\": [\n{}\n  ],\n  \"accepted_tasks\": [\n{}\n  ]\n}}",
            snapshot.migration_id.as_str(),
            plan_hash_str,
            snapshot.current_target_revision.as_u64(),
            tasks_json.join(",\n"),
            accepted_json.join(",\n"),
        )
    }

    /// Deserializes a `CheckpointSnapshot` from a JSON string.
    pub fn deserialize(json_str: &str) -> Result<CheckpointSnapshot, DomainError> {
        let val = JsonValue::parse(json_str)?;

        let migration_id_str = val
            .get_string("migration_id")
            .ok_or_else(|| DomainError::PortError("Missing migration_id in checkpoint".into()))?;

        let plan_hash = val.get_optional_string("plan_hash").map(PlanHash::new);

        let target_rev_num = val.get_u64("current_target_revision").ok_or_else(|| {
            DomainError::PortError("Missing current_target_revision in checkpoint".into())
        })?;

        let task_states_arr = val
            .get_array("task_states")
            .ok_or_else(|| DomainError::PortError("Missing task_states in checkpoint".into()))?;

        let mut task_states = Vec::new();
        for item in task_states_arr {
            let task_id_str = item
                .get_string("task_id")
                .ok_or_else(|| DomainError::PortError("Missing task_id in task_state".into()))?;
            let state_str = item
                .get_string("state")
                .ok_or_else(|| DomainError::PortError("Missing state in task_state".into()))?;
            let retry_count = item.get_u64("retry_count").unwrap_or(0) as u32;

            let state = parse_task_state(&state_str)?;
            task_states.push((TaskId::new(task_id_str), state, retry_count));
        }

        let accepted_arr = val.get_array("accepted_tasks").unwrap_or(&[]);

        let mut accepted_tasks = Vec::new();
        for item in accepted_arr {
            if let Some(s) = item.as_str() {
                accepted_tasks.push(TaskId::new(s));
            }
        }

        Ok(CheckpointSnapshot {
            migration_id: MigrationId::new(migration_id_str),
            plan_hash,
            current_target_revision: ArtifactRevision::new(target_rev_num),
            task_states,
            accepted_tasks,
        })
    }

    /// Persists a `CheckpointSnapshot` to disk.
    pub fn save_checkpoint(path: &Path, snapshot: &CheckpointSnapshot) -> Result<(), DomainError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                DomainError::PortError(format!("Failed to create checkpoint directory: {}", e))
            })?;
        }

        let serialized = Self::serialize(snapshot);
        fs::write(path, serialized).map_err(|e| {
            DomainError::PortError(format!(
                "Failed to write checkpoint file {}: {}",
                path.display(),
                e
            ))
        })?;

        Ok(())
    }

    /// Loads a `CheckpointSnapshot` from disk.
    pub fn load_checkpoint(path: &Path) -> Result<CheckpointSnapshot, DomainError> {
        let content = fs::read_to_string(path).map_err(|e| {
            DomainError::PortError(format!(
                "Failed to read checkpoint file {}: {}",
                path.display(),
                e
            ))
        })?;

        Self::deserialize(&content)
    }
}

fn parse_task_state(s: &str) -> Result<TaskState, DomainError> {
    match s {
        "PENDING" => Ok(TaskState::Pending),
        "READY" => Ok(TaskState::Ready),
        "RUNNING" => Ok(TaskState::Running),
        "VERIFYING" => Ok(TaskState::Verifying),
        "ACCEPTED" => Ok(TaskState::Accepted),
        "NEEDS_REPAIR" => Ok(TaskState::NeedsRepair),
        "BLOCKED" => Ok(TaskState::Blocked),
        "FAILED" => Ok(TaskState::Failed),
        "CANCELLED" => Ok(TaskState::Cancelled),
        other => Err(DomainError::PortError(format!(
            "Unknown task state: {}",
            other
        ))),
    }
}

// ---------------------------------------------------------------------------
// Pure std lightweight JSON Parser
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum JsonValue {
    Null,
    String(String),
    Number(f64),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    fn parse(input: &str) -> Result<Self, DomainError> {
        let mut chars = input.chars().peekable();
        skip_ws(&mut chars);
        parse_json_value(&mut chars)
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            Self::Object(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    fn get_string(&self, key: &str) -> Option<String> {
        self.get(key).and_then(|v| match v {
            Self::String(s) => Some(s.clone()),
            _ => None,
        })
    }

    fn get_optional_string(&self, key: &str) -> Option<String> {
        self.get(key).and_then(|v| match v {
            Self::String(s) => Some(s.clone()),
            Self::Null => None,
            _ => None,
        })
    }

    fn get_u64(&self, key: &str) -> Option<u64> {
        self.get(key).and_then(|v| match v {
            Self::Number(n) => Some(*n as u64),
            _ => None,
        })
    }

    fn get_array(&self, key: &str) -> Option<&[JsonValue]> {
        self.get(key).and_then(|v| match v {
            Self::Array(arr) => Some(arr.as_slice()),
            _ => None,
        })
    }
}

fn skip_ws<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) {
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }
}

fn parse_json_value<I: Iterator<Item = char>>(
    chars: &mut std::iter::Peekable<I>,
) -> Result<JsonValue, DomainError> {
    skip_ws(chars);
    let next_char = chars
        .peek()
        .copied()
        .ok_or_else(|| DomainError::PortError("Unexpected EOF in JSON".into()))?;

    match next_char {
        '{' => parse_json_object(chars),
        '[' => parse_json_array(chars),
        '"' => parse_json_string(chars).map(JsonValue::String),
        'n' => parse_json_null(chars),
        '-' | '0'..='9' => parse_json_number(chars),
        other => Err(DomainError::PortError(format!(
            "Unexpected character '{}' in JSON",
            other
        ))),
    }
}

fn parse_json_string<I: Iterator<Item = char>>(
    chars: &mut std::iter::Peekable<I>,
) -> Result<String, DomainError> {
    if chars.next() != Some('"') {
        return Err(DomainError::PortError("Expected '\"' for string".into()));
    }
    let mut s = String::new();
    while let Some(c) = chars.next() {
        if c == '"' {
            return Ok(s);
        } else if c == '\\' {
            match chars.next() {
                Some('"') => s.push('"'),
                Some('\\') => s.push('\\'),
                Some('n') => s.push('\n'),
                Some('r') => s.push('\r'),
                Some('t') => s.push('\t'),
                Some(other) => s.push(other),
                None => return Err(DomainError::PortError("Unterminated escape in JSON".into())),
            }
        } else {
            s.push(c);
        }
    }
    Err(DomainError::PortError("Unterminated string in JSON".into()))
}

fn parse_json_null<I: Iterator<Item = char>>(
    chars: &mut std::iter::Peekable<I>,
) -> Result<JsonValue, DomainError> {
    let mut keyword = String::new();
    for _ in 0..4 {
        if let Some(c) = chars.next() {
            keyword.push(c);
        }
    }
    if keyword == "null" {
        Ok(JsonValue::Null)
    } else {
        Err(DomainError::PortError(format!(
            "Expected null, got '{}'",
            keyword
        )))
    }
}

fn parse_json_number<I: Iterator<Item = char>>(
    chars: &mut std::iter::Peekable<I>,
) -> Result<JsonValue, DomainError> {
    let mut num_str = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e' || c == 'E' {
            num_str.push(c);
            chars.next();
        } else {
            break;
        }
    }
    let num = num_str
        .parse::<f64>()
        .map_err(|e| DomainError::PortError(format!("Invalid number '{}': {}", num_str, e)))?;
    Ok(JsonValue::Number(num))
}

fn parse_json_object<I: Iterator<Item = char>>(
    chars: &mut std::iter::Peekable<I>,
) -> Result<JsonValue, DomainError> {
    chars.next(); // consume '{'
    skip_ws(chars);

    let mut entries = Vec::new();
    if chars.peek() == Some(&'}') {
        chars.next();
        return Ok(JsonValue::Object(entries));
    }

    loop {
        skip_ws(chars);
        let key = parse_json_string(chars)?;
        skip_ws(chars);
        if chars.next() != Some(':') {
            return Err(DomainError::PortError("Expected ':' in JSON object".into()));
        }
        skip_ws(chars);
        let val = parse_json_value(chars)?;
        entries.push((key, val));
        skip_ws(chars);

        match chars.next() {
            Some(',') => continue,
            Some('}') => break,
            other => {
                return Err(DomainError::PortError(format!(
                    "Expected ',' or '}}' in JSON object, got {:?}",
                    other
                )))
            }
        }
    }

    Ok(JsonValue::Object(entries))
}

fn parse_json_array<I: Iterator<Item = char>>(
    chars: &mut std::iter::Peekable<I>,
) -> Result<JsonValue, DomainError> {
    chars.next(); // consume '['
    skip_ws(chars);

    let mut elements = Vec::new();
    if chars.peek() == Some(&']') {
        chars.next();
        return Ok(JsonValue::Array(elements));
    }

    loop {
        skip_ws(chars);
        let val = parse_json_value(chars)?;
        elements.push(val);
        skip_ws(chars);

        match chars.next() {
            Some(',') => continue,
            Some(']') => break,
            other => {
                return Err(DomainError::PortError(format!(
                    "Expected ',' or ']' in JSON array, got {:?}",
                    other
                )))
            }
        }
    }

    Ok(JsonValue::Array(elements))
}

// ---------------------------------------------------------------------------
// Reproducibility Gate & OS-Level Agent Containment
// ---------------------------------------------------------------------------

/// Captures and verifies the immutable git commit, toolchain, and read-only fixture digests
/// required before running a controlled migration probe or pilot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReproducibilityGate {
    pub mae_commit_sha: String,
    pub rustc_version: String,
    pub python_version: String,
    pub fixture_hashes: BTreeMap<String, ContentHash>,
}

impl ReproducibilityGate {
    /// Captures the current workspace git commit SHA, toolchain versions, and fixture hashes.
    pub fn capture(repo_root: &Path, fixtures: &[PathBuf]) -> Result<Self, DomainError> {
        let sha_out = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo_root)
            .output()
            .map_err(|e| DomainError::PortError(format!("Failed to execute git rev-parse: {e}")))?;

        if !sha_out.status.success() {
            return Err(DomainError::PortError(
                "Reproducibility gate failed: unable to resolve git HEAD SHA".to_string(),
            ));
        }

        let mae_commit_sha = String::from_utf8_lossy(&sha_out.stdout).trim().to_string();
        if mae_commit_sha.len() != 40 || !mae_commit_sha.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(DomainError::PortError(format!(
                "Reproducibility gate failed: invalid git commit SHA '{mae_commit_sha}'"
            )));
        }

        let rustc_out = Command::new("rustc")
            .arg("--version")
            .output()
            .map_err(|e| DomainError::PortError(format!("Failed to query rustc version: {e}")))?;
        let rustc_version = String::from_utf8_lossy(&rustc_out.stdout)
            .trim()
            .to_string();

        let py_out = Command::new("python3")
            .arg("--version")
            .output()
            .or_else(|_| Command::new("python").arg("--version").output())
            .map_err(|e| DomainError::PortError(format!("Failed to query python version: {e}")))?;
        let python_version = {
            let stdout = String::from_utf8_lossy(&py_out.stdout).trim().to_string();
            if stdout.is_empty() {
                String::from_utf8_lossy(&py_out.stderr).trim().to_string()
            } else {
                stdout
            }
        };

        let mut fixture_hashes = BTreeMap::new();
        for path in fixtures {
            let bytes = fs::read(path).map_err(|e| {
                DomainError::PortError(format!(
                    "Reproducibility gate failed: missing fixture {}: {}",
                    path.display(),
                    e
                ))
            })?;
            let key = path
                .strip_prefix(repo_root)
                .unwrap_or(path.as_path())
                .to_string_lossy()
                .replace('\\', "/");
            fixture_hashes.insert(key, ContentHash::compute(&bytes));
        }

        Ok(Self {
            mae_commit_sha,
            rustc_version,
            python_version,
            fixture_hashes,
        })
    }

    /// Verifies that the workspace matches the recorded commit SHA and all fixture hashes remain intact.
    pub fn verify(&self, repo_root: &Path) -> Result<(), DomainError> {
        let sha_out = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo_root)
            .output()
            .map_err(|e| DomainError::PortError(format!("Failed to execute git rev-parse: {e}")))?;

        let current_sha = String::from_utf8_lossy(&sha_out.stdout).trim().to_string();
        if current_sha != self.mae_commit_sha {
            return Err(DomainError::PortError(format!(
                "Reproducibility gate failed: expected commit '{}', found '{}'",
                self.mae_commit_sha, current_sha
            )));
        }

        for (rel_path, expected_hash) in &self.fixture_hashes {
            let full_path = repo_root.join(rel_path);
            let bytes = fs::read(&full_path).map_err(|e| {
                DomainError::PortError(format!(
                    "Reproducibility gate failed: cannot read fixture {}: {}",
                    full_path.display(),
                    e
                ))
            })?;
            let actual_hash = ContentHash::compute(&bytes);
            if &actual_hash != expected_hash {
                return Err(DomainError::PortError(format!(
                    "Reproducibility gate failed: fixture '{}' hash mismatch (expected {}, got {})",
                    rel_path, expected_hash, actual_hash
                )));
            }
        }

        Ok(())
    }
}

/// OS-level process, filesystem, network, and timeout containment policy for real coding-model runs.
#[derive(Debug, Clone)]
pub struct AgentContainmentPolicy {
    pub working_dir: PathBuf,
    pub read_only_fixtures: Vec<PathBuf>,
    pub network_disabled: bool,
    pub timeout: Duration,
    pub max_output_bytes: usize,
}

impl AgentContainmentPolicy {
    pub fn new(working_dir: impl Into<PathBuf>) -> Self {
        Self {
            working_dir: working_dir.into(),
            read_only_fixtures: Vec::new(),
            network_disabled: true,
            timeout: Duration::from_secs(60),
            max_output_bytes: 256 * 1024,
        }
    }

    pub fn with_read_only_fixture(mut self, path: impl Into<PathBuf>) -> Self {
        self.read_only_fixtures.push(path.into());
        self
    }

    pub fn with_network_disabled(mut self, disabled: bool) -> Self {
        self.network_disabled = disabled;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_max_output_bytes(mut self, max_bytes: usize) -> Self {
        self.max_output_bytes = max_bytes;
        self
    }
}

#[derive(Debug, Clone)]
struct SubprocessRunnerConfig {
    executable: String,
    base_args: Vec<String>,
    containment: AgentContainmentPolicy,
}

fn execute_contained_subprocess(
    provider: &str,
    runner: &SubprocessRunnerConfig,
    rendered_prompt: &str,
    provenance: &PromptProvenance,
) -> Result<AgentResult, DomainError> {
    fs::create_dir_all(&runner.containment.working_dir).map_err(|e| {
        DomainError::PortError(format!(
            "Failed to create agent containment working directory: {e}"
        ))
    })?;

    // 1. Snapshot read-only fixtures prior to agent execution
    let mut fixture_snapshots: Vec<(PathBuf, Vec<u8>, ContentHash)> = Vec::new();
    for path in &runner.containment.read_only_fixtures {
        let bytes = fs::read(path).map_err(|e| {
            DomainError::PortError(format!(
                "Failed to snapshot read-only fixture {}: {}",
                path.display(),
                e
            ))
        })?;
        let hash = ContentHash::compute(&bytes);
        fixture_snapshots.push((path.clone(), bytes, hash));
    }

    // 2. Prepare temporary stdout/stderr capture files so descendant processes cannot hang pipes
    let stdout_file_path = runner
        .containment
        .working_dir
        .join(format!(".mae_agent_stdout_{}.log", std::process::id()));
    let stderr_file_path = runner
        .containment
        .working_dir
        .join(format!(".mae_agent_stderr_{}.log", std::process::id()));

    let stdout_file = fs::File::create(&stdout_file_path)
        .map_err(|e| DomainError::PortError(format!("Failed to create agent stdout log: {e}")))?;
    let stderr_file = fs::File::create(&stderr_file_path)
        .map_err(|e| DomainError::PortError(format!("Failed to create agent stderr log: {e}")))?;

    let mut cmd = Command::new(&runner.executable);
    cmd.args(&runner.base_args)
        .current_dir(&runner.containment.working_dir)
        .env("MAE_PROMPT_ID", &provenance.prompt_id)
        .env("MAE_PROMPT_DIGEST", &provenance.content_digest)
        .env("MAE_RENDERED_PROMPT", rendered_prompt)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));

    if runner.containment.network_disabled {
        cmd.env("MAE_NETWORK_DISABLED", "1")
            .env("CARGO_NET_OFFLINE", "true")
            .env("PIP_NO_INDEX", "1")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("HTTP_PROXY", "http://127.0.0.1:1")
            .env("HTTPS_PROXY", "http://127.0.0.1:1")
            .env("ALL_PROXY", "http://127.0.0.1:1")
            .env("NO_PROXY", "");
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    let mut child = cmd.spawn().map_err(|e| {
        let _ = fs::remove_file(&stdout_file_path);
        let _ = fs::remove_file(&stderr_file_path);
        DomainError::PortError(format!(
            "{} agent backend failed to spawn '{}': {}",
            provider, runner.executable, e
        ))
    })?;

    let child_pid = child.id();
    let start = Instant::now();
    let poll_interval = Duration::from_millis(15);
    let mut exit_status = None;
    let mut timed_out = false;

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                exit_status = Some(status);
                break;
            }
            Ok(None) => {
                if start.elapsed() >= runner.containment.timeout {
                    timed_out = true;
                    break;
                }
                std::thread::sleep(poll_interval);
            }
            Err(e) => {
                terminate_process_tree(&mut child, child_pid);
                let _ = fs::remove_file(&stdout_file_path);
                let _ = fs::remove_file(&stderr_file_path);
                return Err(DomainError::PortError(format!(
                    "{} agent process wait failed: {}",
                    provider, e
                )));
            }
        }
    }

    // Always terminate the entire process group so no background child survives the run
    terminate_process_tree(&mut child, child_pid);

    let stdout_bytes = fs::read(&stdout_file_path).unwrap_or_default();
    let stderr_bytes = fs::read(&stderr_file_path).unwrap_or_default();
    let _ = fs::remove_file(&stdout_file_path);
    let _ = fs::remove_file(&stderr_file_path);

    // 3. Verify read-only fixtures were not mutated; restore and fail if violated
    let mut violation_msg = None;
    for (path, original_bytes, expected_hash) in &fixture_snapshots {
        let current_ok = fs::read(path)
            .map(|b| &ContentHash::compute(&b) == expected_hash)
            .unwrap_or(false);
        if !current_ok {
            let _ = fs::write(path, original_bytes);
            if violation_msg.is_none() {
                violation_msg = Some(format!(
                    "Containment violation: read-only fixture '{}' was mutated during {} agent execution",
                    path.display(),
                    provider
                ));
            }
        }
    }

    if let Some(msg) = violation_msg {
        return Err(DomainError::PortError(msg));
    }

    if timed_out {
        return Err(DomainError::PortError(format!(
            "{} agent process timed out after {:?} on prompt '{}'",
            provider, runner.containment.timeout, provenance.prompt_id
        )));
    }

    let status = exit_status.unwrap();
    let max_len = runner.containment.max_output_bytes;
    let stdout_trunc = &stdout_bytes[..stdout_bytes.len().min(max_len)];
    let stderr_trunc = &stderr_bytes[..stderr_bytes.len().min(max_len)];
    let stdout_str = String::from_utf8_lossy(stdout_trunc).trim().to_string();
    let stderr_str = String::from_utf8_lossy(stderr_trunc).trim().to_string();

    let log_summary = if stderr_str.is_empty() {
        format!(
            "{} executed prompt '{}': {}",
            provider, provenance.prompt_id, stdout_str
        )
    } else {
        format!(
            "{} executed prompt '{}': stdout={} stderr={}",
            provider, provenance.prompt_id, stdout_str, stderr_str
        )
    };

    Ok(AgentResult {
        output_artifacts: Vec::new(),
        log_summary,
        success: status.success(),
    })
}

fn terminate_process_tree(child: &mut std::process::Child, child_pid: u32) {
    #[cfg(unix)]
    {
        let neg_pgid = format!("-{}", child_pid);
        let _ = Command::new("kill")
            .args(["-9", "--", &neg_pgid])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(windows)]
    {
        let pid_str = child_pid.to_string();
        let _ = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid_str])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

// ---------------------------------------------------------------------------
// Agent Backend Implementations
// ---------------------------------------------------------------------------

/// AI Agent backend provider for OpenAI Codex.
#[derive(Debug, Clone, Default)]
pub struct CodexAgentBackend {
    runner: Option<SubprocessRunnerConfig>,
}

impl CodexAgentBackend {
    pub fn new() -> Self {
        Self { runner: None }
    }

    pub fn with_process_runner(
        mut self,
        executable: impl Into<String>,
        base_args: Vec<String>,
        containment: AgentContainmentPolicy,
    ) -> Self {
        self.runner = Some(SubprocessRunnerConfig {
            executable: executable.into(),
            base_args,
            containment,
        });
        self
    }

    /// Builds command-line arguments for Codex CLI.
    pub fn build_cli_args(&self, prompt: &str, mcp_servers: &[String]) -> Vec<String> {
        let mut args = vec![
            "codex".to_string(),
            "exec".to_string(),
            "--prompt".to_string(),
            prompt.to_string(),
        ];
        for server in mcp_servers {
            args.push("--mcp-server".to_string());
            args.push(server.clone());
        }
        args
    }
}

impl AgentBackendPort for CodexAgentBackend {
    fn execute_prompt(
        &self,
        rendered_prompt: &str,
        provenance: &PromptProvenance,
    ) -> Result<AgentResult, DomainError> {
        match &self.runner {
            Some(runner) => {
                execute_contained_subprocess("Codex", runner, rendered_prompt, provenance)
            }
            None => Err(unconfigured_agent_backend("Codex", provenance)),
        }
    }
}

/// AI Agent backend provider for Anthropic Claude Code.
#[derive(Debug, Clone, Default)]
pub struct ClaudeCodeAgentBackend {
    runner: Option<SubprocessRunnerConfig>,
}

impl ClaudeCodeAgentBackend {
    pub fn new() -> Self {
        Self { runner: None }
    }

    pub fn with_process_runner(
        mut self,
        executable: impl Into<String>,
        base_args: Vec<String>,
        containment: AgentContainmentPolicy,
    ) -> Self {
        self.runner = Some(SubprocessRunnerConfig {
            executable: executable.into(),
            base_args,
            containment,
        });
        self
    }

    /// Builds command-line arguments for Claude Code CLI.
    pub fn build_cli_args(&self, prompt: &str, mcp_servers: &[String]) -> Vec<String> {
        let mut args = vec!["claude".to_string(), "-p".to_string(), prompt.to_string()];
        for server in mcp_servers {
            args.push("--mcp".to_string());
            args.push(server.clone());
        }
        args
    }
}

impl AgentBackendPort for ClaudeCodeAgentBackend {
    fn execute_prompt(
        &self,
        rendered_prompt: &str,
        provenance: &PromptProvenance,
    ) -> Result<AgentResult, DomainError> {
        match &self.runner {
            Some(runner) => {
                execute_contained_subprocess("Claude Code", runner, rendered_prompt, provenance)
            }
            None => Err(unconfigured_agent_backend("Claude Code", provenance)),
        }
    }
}

/// AI Agent backend provider for GitHub Copilot.
#[derive(Debug, Clone, Default)]
pub struct CopilotAgentBackend {
    runner: Option<SubprocessRunnerConfig>,
}

impl CopilotAgentBackend {
    pub fn new() -> Self {
        Self { runner: None }
    }

    pub fn with_process_runner(
        mut self,
        executable: impl Into<String>,
        base_args: Vec<String>,
        containment: AgentContainmentPolicy,
    ) -> Self {
        self.runner = Some(SubprocessRunnerConfig {
            executable: executable.into(),
            base_args,
            containment,
        });
        self
    }

    /// Builds command-line arguments for Copilot CLI.
    pub fn build_cli_args(&self, prompt: &str, mcp_servers: &[String]) -> Vec<String> {
        let mut args = vec![
            "copilot".to_string(),
            "chat".to_string(),
            "--prompt".to_string(),
            prompt.to_string(),
        ];
        for server in mcp_servers {
            args.push("--server".to_string());
            args.push(server.clone());
        }
        args
    }
}

impl AgentBackendPort for CopilotAgentBackend {
    fn execute_prompt(
        &self,
        rendered_prompt: &str,
        provenance: &PromptProvenance,
    ) -> Result<AgentResult, DomainError> {
        match &self.runner {
            Some(runner) => {
                execute_contained_subprocess("Copilot", runner, rendered_prompt, provenance)
            }
            None => Err(unconfigured_agent_backend("Copilot", provenance)),
        }
    }
}

fn unconfigured_agent_backend(provider: &str, provenance: &PromptProvenance) -> DomainError {
    DomainError::PortError(format!(
        "{} agent backend cannot execute prompt '{}' because process execution is not configured",
        provider, provenance.prompt_id
    ))
}

/// Deterministic mock AI agent backend for fast, reproducible testing.
#[derive(Debug, Clone)]
pub struct DeterministicMockAgentBackend {
    pub custom_responses: BTreeMap<String, AgentResult>,
    pub default_success: bool,
}

impl DeterministicMockAgentBackend {
    pub fn new() -> Self {
        Self {
            custom_responses: BTreeMap::new(),
            default_success: true,
        }
    }

    pub fn with_response(mut self, prompt_id: &str, result: AgentResult) -> Self {
        self.custom_responses.insert(prompt_id.to_string(), result);
        self
    }

    pub fn with_default_success(mut self, success: bool) -> Self {
        self.default_success = success;
        self
    }

    /// Builds command-line arguments for Mock CLI.
    pub fn build_cli_args(&self, prompt: &str, mcp_servers: &[String]) -> Vec<String> {
        let mut args = vec![
            "mock-agent".to_string(),
            "--prompt".to_string(),
            prompt.to_string(),
        ];
        for server in mcp_servers {
            args.push("--mcp".to_string());
            args.push(server.clone());
        }
        args
    }
}

impl Default for DeterministicMockAgentBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentBackendPort for DeterministicMockAgentBackend {
    fn execute_prompt(
        &self,
        _rendered_prompt: &str,
        provenance: &PromptProvenance,
    ) -> Result<AgentResult, DomainError> {
        if let Some(res) = self.custom_responses.get(&provenance.prompt_id) {
            return Ok(res.clone());
        }

        Ok(AgentResult {
            output_artifacts: Vec::new(),
            log_summary: format!(
                "DeterministicMock executed prompt: {}",
                provenance.prompt_id
            ),
            success: self.default_success,
        })
    }
}
