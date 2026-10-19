//! Evaluation, telemetry, token accounting, timing, and architectural consistency metrics.

/// Estimates token count deterministically from text (~4 characters per token, minimum 1 for non-empty).
pub fn estimate_token_count(text: &str) -> u64 {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return 0;
    }
    let char_count = trimmed.chars().count() as u64;
    let word_count = trimmed.split_whitespace().count() as u64;
    let by_chars = char_count.div_ceil(4);
    by_chars.max(word_count)
}

/// Comprehensive telemetry and consistency evaluation metrics for a single migration run.
#[derive(Debug, Clone, PartialEq)]
pub struct MigrationEvalMetrics {
    pub prompt_tokens_estimated: u64,
    pub completion_tokens_estimated: u64,
    pub total_tokens_estimated: u64,
    pub agent_invocations_count: u32,
    pub discovery_duration_us: u64,
    pub semantics_duration_us: u64,
    pub scaffold_duration_us: u64,
    pub execution_duration_us: u64,
    pub verification_duration_us: u64,
    pub total_wall_clock_duration_us: u64,
    pub component_name_consistency_ratio: f64,
    pub route_consistency_ratio: f64,
    pub state_field_consistency_ratio: f64,
    pub ui_element_parity_ratio: f64,
    pub architecture_consistency_score: f64,
    pub matched_components: Vec<String>,
    pub missing_components: Vec<String>,
    pub matched_routes: Vec<String>,
    pub missing_routes: Vec<String>,
    pub matched_state_fields: Vec<String>,
    pub missing_state_fields: Vec<String>,
    pub source_ui_elements_count: usize,
    pub target_ui_elements_count: usize,
}

impl Default for MigrationEvalMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl MigrationEvalMetrics {
    pub fn new() -> Self {
        Self {
            prompt_tokens_estimated: 0,
            completion_tokens_estimated: 0,
            total_tokens_estimated: 0,
            agent_invocations_count: 0,
            discovery_duration_us: 0,
            semantics_duration_us: 0,
            scaffold_duration_us: 0,
            execution_duration_us: 0,
            verification_duration_us: 0,
            total_wall_clock_duration_us: 0,
            component_name_consistency_ratio: 1.0,
            route_consistency_ratio: 1.0,
            state_field_consistency_ratio: 1.0,
            ui_element_parity_ratio: 1.0,
            architecture_consistency_score: 1.0,
            matched_components: Vec::new(),
            missing_components: Vec::new(),
            matched_routes: Vec::new(),
            missing_routes: Vec::new(),
            matched_state_fields: Vec::new(),
            missing_state_fields: Vec::new(),
            source_ui_elements_count: 0,
            target_ui_elements_count: 0,
        }
    }

    pub fn record_agent_invocation(&mut self, prompt_text: &str, completion_text: &str) {
        let p_tokens = estimate_token_count(prompt_text);
        let c_tokens = estimate_token_count(completion_text);
        self.prompt_tokens_estimated += p_tokens;
        self.completion_tokens_estimated += c_tokens;
        self.total_tokens_estimated =
            self.prompt_tokens_estimated + self.completion_tokens_estimated;
        self.agent_invocations_count += 1;
    }

    pub fn total_duration_ms(&self) -> f64 {
        self.total_wall_clock_duration_us as f64 / 1000.0
    }

    pub fn overall_consistency_score(&self) -> f64 {
        (self.component_name_consistency_ratio
            + self.route_consistency_ratio
            + self.state_field_consistency_ratio
            + self.ui_element_parity_ratio
            + self.architecture_consistency_score)
            / 5.0
    }

    pub fn to_json_report(&self) -> String {
        format!(
            concat!(
                "{{\n",
                "  \"prompt_tokens_estimated\": {},\n",
                "  \"completion_tokens_estimated\": {},\n",
                "  \"total_tokens_estimated\": {},\n",
                "  \"agent_invocations_count\": {},\n",
                "  \"discovery_duration_us\": {},\n",
                "  \"semantics_duration_us\": {},\n",
                "  \"scaffold_duration_us\": {},\n",
                "  \"execution_duration_us\": {},\n",
                "  \"verification_duration_us\": {},\n",
                "  \"total_wall_clock_duration_us\": {},\n",
                "  \"total_duration_ms\": {:.3},\n",
                "  \"component_name_consistency_ratio\": {:.4},\n",
                "  \"route_consistency_ratio\": {:.4},\n",
                "  \"state_field_consistency_ratio\": {:.4},\n",
                "  \"ui_element_parity_ratio\": {:.4},\n",
                "  \"architecture_consistency_score\": {:.4},\n",
                "  \"overall_consistency_score\": {:.4},\n",
                "  \"matched_components_count\": {},\n",
                "  \"missing_components_count\": {},\n",
                "  \"matched_routes_count\": {},\n",
                "  \"missing_routes_count\": {},\n",
                "  \"matched_state_fields_count\": {},\n",
                "  \"missing_state_fields_count\": {},\n",
                "  \"source_ui_elements_count\": {},\n",
                "  \"target_ui_elements_count\": {}\n",
                "}}\n"
            ),
            self.prompt_tokens_estimated,
            self.completion_tokens_estimated,
            self.total_tokens_estimated,
            self.agent_invocations_count,
            self.discovery_duration_us,
            self.semantics_duration_us,
            self.scaffold_duration_us,
            self.execution_duration_us,
            self.verification_duration_us,
            self.total_wall_clock_duration_us,
            self.total_duration_ms(),
            self.component_name_consistency_ratio,
            self.route_consistency_ratio,
            self.state_field_consistency_ratio,
            self.ui_element_parity_ratio,
            self.architecture_consistency_score,
            self.overall_consistency_score(),
            self.matched_components.len(),
            self.missing_components.len(),
            self.matched_routes.len(),
            self.missing_routes.len(),
            self.matched_state_fields.len(),
            self.missing_state_fields.len(),
            self.source_ui_elements_count,
            self.target_ui_elements_count,
        )
    }
}
