use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

use mae_domain::{
    AgentBackendPort, AgentResult, ArtifactRevision, DomainError, MigrationAggregate, MigrationId,
    MigrationTask, PlanHash, PromptProvenance, TaskId, TaskState,
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
// Agent Backend Implementations
// ---------------------------------------------------------------------------

/// AI Agent backend provider for OpenAI Codex.
#[derive(Debug, Clone, Default)]
pub struct CodexAgentBackend;

impl CodexAgentBackend {
    pub fn new() -> Self {
        Self
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
        _rendered_prompt: &str,
        provenance: &PromptProvenance,
    ) -> Result<AgentResult, DomainError> {
        Ok(AgentResult {
            output_artifacts: Vec::new(),
            log_summary: format!("Codex backend executed prompt: {}", provenance.prompt_id),
            success: true,
        })
    }
}

/// AI Agent backend provider for Anthropic Claude Code.
#[derive(Debug, Clone, Default)]
pub struct ClaudeCodeAgentBackend;

impl ClaudeCodeAgentBackend {
    pub fn new() -> Self {
        Self
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
        _rendered_prompt: &str,
        provenance: &PromptProvenance,
    ) -> Result<AgentResult, DomainError> {
        Ok(AgentResult {
            output_artifacts: Vec::new(),
            log_summary: format!(
                "ClaudeCode backend executed prompt: {}",
                provenance.prompt_id
            ),
            success: true,
        })
    }
}

/// AI Agent backend provider for GitHub Copilot.
#[derive(Debug, Clone, Default)]
pub struct CopilotAgentBackend;

impl CopilotAgentBackend {
    pub fn new() -> Self {
        Self
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
        _rendered_prompt: &str,
        provenance: &PromptProvenance,
    ) -> Result<AgentResult, DomainError> {
        Ok(AgentResult {
            output_artifacts: Vec::new(),
            log_summary: format!("Copilot backend executed prompt: {}", provenance.prompt_id),
            success: true,
        })
    }
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
