use std::path::PathBuf;
use std::time::Instant;

use mae_domain::{
    AcceptanceRejectionReason, AgentBackendPort, ArtifactRepositoryPort, ArtifactRevision,
    ContentHash, ContractId, DomainError, FailureKind, HealthScore, KnowledgeRepositoryPort,
    MigrationAggregate, MigrationEvalMetrics, MigrationId, MigrationTask, PlanHash,
    PromptProviderPort, RecoveryDecision, RecoveryPolicy, SourceAdapterPort, TargetAdapterPort,
    TaskId, TaskState, VerificationCategory, VerifierPluginPort,
};

use crate::aamf_runtime::{CheckpointManager, CheckpointSnapshot, ExecutionMode, WaveSchedule};

/// Comprehensive outcome report produced at the end of a migration orchestrator execution run.
#[derive(Debug, Clone, PartialEq)]
pub struct MigrationRunReport {
    pub migration_id: MigrationId,
    pub plan_hash: Option<PlanHash>,
    pub final_target_revision: ArtifactRevision,
    pub health_score: HealthScore,
    pub completion_status: Result<(), Vec<String>>,
    pub task_states: Vec<(TaskId, TaskState)>,
    pub total_evidence_count: usize,
    pub waves_executed: usize,
    pub repairs_triggered: u32,
    pub checkpoint_saved: bool,
    pub eval_metrics: MigrationEvalMetrics,
}

impl MigrationRunReport {
    pub fn is_successful(&self) -> bool {
        self.completion_status.is_ok()
            && self.task_states.iter().all(|(_, s)| s.is_accepted())
            && self.health_score.score >= 1.0
    }
}

/// Application service coordinating a complete migration lifecycle across injected ports.
pub struct MigrationOrchestrator {
    pub prompt_provider: Box<dyn PromptProviderPort>,
    pub knowledge_repo: Box<dyn KnowledgeRepositoryPort>,
    pub agent_backend: Box<dyn AgentBackendPort>,
    pub source_adapter: Box<dyn SourceAdapterPort>,
    pub target_adapter: Box<dyn TargetAdapterPort>,
    pub artifact_repo: Box<dyn ArtifactRepositoryPort>,
    pub verifiers: Vec<Box<dyn VerifierPluginPort>>,
    pub checkpoint_path: Option<PathBuf>,
    pub execution_mode: ExecutionMode,
}

impl MigrationOrchestrator {
    pub fn new(
        prompt_provider: Box<dyn PromptProviderPort>,
        knowledge_repo: Box<dyn KnowledgeRepositoryPort>,
        agent_backend: Box<dyn AgentBackendPort>,
        source_adapter: Box<dyn SourceAdapterPort>,
        target_adapter: Box<dyn TargetAdapterPort>,
        artifact_repo: Box<dyn ArtifactRepositoryPort>,
        verifiers: Vec<Box<dyn VerifierPluginPort>>,
    ) -> Self {
        Self {
            prompt_provider,
            knowledge_repo,
            agent_backend,
            source_adapter,
            target_adapter,
            artifact_repo,
            verifiers,
            checkpoint_path: None,
            execution_mode: ExecutionMode::WaveBarrier,
        }
    }

    pub fn with_checkpoint_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.checkpoint_path = Some(path.into());
        self
    }

    pub fn with_execution_mode(mut self, mode: ExecutionMode) -> Self {
        self.execution_mode = mode;
        self
    }

    /// Resumes the aggregate state from disk if a checkpoint exists at `checkpoint_path`.
    pub fn resume_from_checkpoint(
        &mut self,
        aggregate: &mut MigrationAggregate,
    ) -> Result<bool, DomainError> {
        if let Some(ref cp_path) = self.checkpoint_path {
            if cp_path.exists() {
                let snapshot = CheckpointManager::load_checkpoint(cp_path)?;
                snapshot.apply_to_aggregate(aggregate)?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Executes the complete migration pipeline.
    pub fn run(
        &mut self,
        aggregate: &mut MigrationAggregate,
        source_path: &str,
        target_dir: &str,
    ) -> Result<MigrationRunReport, DomainError> {
        let wall_clock_start = Instant::now();
        let mut eval_metrics = MigrationEvalMetrics::new();

        // 1. Validate OKF curated repository bundle
        let okf_valid = self.knowledge_repo.validate_bundle()?;
        if !okf_valid {
            return Err(DomainError::PortError(
                "OKF repository bundle validation failed".to_string(),
            ));
        }

        // Verify required OPF prompt catalog entries
        let required_prompts = [
            "migration.planner",
            "migration.code-migrator",
            "migration.implementation-style",
            "migration.parity-verifier",
            "migration.failure-resolver",
        ];
        for prompt_name in &required_prompts {
            self.prompt_provider.get_prompt(prompt_name)?;
        }

        // 2. Discover source inventory, extract UI semantics, assess capabilities
        let discovery_start = Instant::now();
        let source_inventory_artifact = self.source_adapter.discover(source_path)?;
        self.artifact_repo.store(source_inventory_artifact)?;
        eval_metrics.discovery_duration_us = discovery_start.elapsed().as_micros() as u64;

        let semantics_start = Instant::now();
        let semantic_model = self.source_adapter.extract_semantics(source_path)?;
        if aggregate.capability_report.assessments.is_empty()
            && aggregate.capability_report.gaps.is_empty()
        {
            let capability_report = self.source_adapter.assess_capabilities(&semantic_model)?;
            aggregate.set_capability_report(capability_report);
        }
        eval_metrics.semantics_duration_us = semantics_start.elapsed().as_micros() as u64;

        // 3. Compute and lock canonical reviewed PlanHash
        if aggregate.plan_hash.is_none() {
            let contract_ids: Vec<ContractId> = aggregate.contracts.keys().cloned().collect();
            let cap_digest =
                ContentHash::compute(format!("{:?}", aggregate.capability_report).as_bytes()).0;
            let plan_digest = ContentHash::compute(
                format!("{:?}", aggregate.tasks.keys().collect::<Vec<_>>()).as_bytes(),
            )
            .0;
            let plan_hash = PlanHash::compute(
                &aggregate.source_fingerprint,
                &contract_ids,
                &cap_digest,
                &plan_digest,
                "react:v0.1,rust-slint:v0.1",
            );
            aggregate.lock_plan_hash(plan_hash);
        }

        // 4. Scaffold native target project
        let scaffold_start = Instant::now();
        let _scaffold_summary = self
            .target_adapter
            .scaffold_target(&semantic_model, target_dir)?;
        eval_metrics.scaffold_duration_us = scaffold_start.elapsed().as_micros() as u64;

        // 5. Execute tasks in topological wave order
        let tasks: Vec<MigrationTask> = aggregate.tasks.values().cloned().collect();
        let waves = WaveSchedule::compute_waves(&tasks)?;

        let mut waves_executed = 0usize;
        let mut repairs_triggered = 0u32;
        let mut checkpoint_saved = false;

        for wave in &waves {
            waves_executed += 1;
            for task_id in wave {
                let task = aggregate
                    .get_task(task_id)
                    .ok_or_else(|| DomainError::TaskNotFound(task_id.clone()))?;
                if task.state.is_accepted() {
                    // Already accepted (e.g. from restored checkpoint)
                    continue;
                }

                // Execution & repair loop
                loop {
                    let exec_step_start = Instant::now();
                    // Pending / NeedsRepair -> Ready
                    aggregate.mark_task_ready(task_id)?;
                    // Ready -> Running
                    aggregate.start_task_running(task_id)?;

                    let context_path = format!("{}/tasks/{}", target_dir, task_id.as_str());

                    // Render code-migrator prompt
                    let (code_prompt, code_prov) = self.prompt_provider.render_prompt(
                        "migration.code-migrator",
                        &[("context_path", &context_path)],
                    )?;

                    // Render implementation-style prompt
                    let (style_prompt, _style_prov) = self.prompt_provider.render_prompt(
                        "migration.implementation-style",
                        &[("context_path", &context_path)],
                    )?;

                    let combined_prompt = format!("{}\n\n{}", code_prompt, style_prompt);
                    let agent_result = self
                        .agent_backend
                        .execute_prompt(&combined_prompt, &code_prov)?;
                    eval_metrics
                        .record_agent_invocation(&combined_prompt, &agent_result.log_summary);
                    if !agent_result.success {
                        let reason = format!("Agent backend failed: {}", agent_result.log_summary);
                        aggregate.mark_task_failed(task_id, &reason)?;
                        return Err(DomainError::PortError(reason));
                    }

                    // Emit target code and advance target revision
                    let current_task = aggregate.get_task(task_id).unwrap().clone();
                    let _emitted_rev = self.target_adapter.emit_target_code(
                        &current_task,
                        &semantic_model,
                        target_dir,
                    )?;
                    let current_target_rev = aggregate.advance_target_revision();
                    eval_metrics.execution_duration_us +=
                        exec_step_start.elapsed().as_micros() as u64;

                    // Transition Running -> Verifying
                    let verify_step_start = Instant::now();
                    aggregate.start_task_verifying(task_id)?;

                    // Run all verifiers for task's required contracts + global Build and NativeCompliance
                    let mut contracts_to_verify = current_task.required_contracts.clone();
                    for (cid, contract) in &aggregate.contracts {
                        if matches!(
                            contract.category,
                            VerificationCategory::Build | VerificationCategory::NativeCompliance
                        ) && !contracts_to_verify.contains(cid)
                        {
                            contracts_to_verify.push(cid.clone());
                        }
                    }

                    for cid in &contracts_to_verify {
                        if let Some(contract) = aggregate.contracts.get(cid).cloned() {
                            for verifier in &self.verifiers {
                                if verifier.category() == contract.category {
                                    let evidence =
                                        verifier.verify(&contract, current_target_rev)?;
                                    aggregate.record_evidence(evidence);
                                }
                            }
                        }
                    }
                    eval_metrics.verification_duration_us +=
                        verify_step_start.elapsed().as_micros() as u64;

                    // Attempt to accept task via pure AcceptancePolicy
                    match aggregate.accept_task(task_id) {
                        Ok(()) => {
                            // Task successfully accepted!
                            if let Some(ref cp_path) = self.checkpoint_path {
                                let snapshot = CheckpointSnapshot::from_aggregate(aggregate);
                                CheckpointManager::save_checkpoint(cp_path, &snapshot)?;
                                checkpoint_saved = true;
                            }
                            break;
                        }
                        Err(reasons) => {
                            repairs_triggered += 1;
                            let task_now = aggregate.get_task(task_id).unwrap().clone();

                            let failure_kind = if reasons.iter().any(|r| {
                                matches!(r, AcceptanceRejectionReason::NativeComplianceFailed)
                            }) {
                                FailureKind::ImplementationDefect
                            } else if reasons.iter().any(|r| {
                                matches!(
                                    r,
                                    AcceptanceRejectionReason::StaleEvidenceForTargetRevision { .. }
                                )
                            }) {
                                FailureKind::StaleEvidence
                            } else if reasons.iter().any(|r| {
                                matches!(r, AcceptanceRejectionReason::CriticalCapabilityGapExists)
                            }) {
                                FailureKind::UnsupportedCapability
                            } else {
                                FailureKind::ImplementationDefect
                            };

                            let decision = RecoveryPolicy::decide(
                                failure_kind,
                                task_now.retry_count,
                                task_now.max_retries,
                            );

                            match decision {
                                RecoveryDecision::Repair => {
                                    let reasons_summary = reasons
                                        .iter()
                                        .map(|r| r.to_string())
                                        .collect::<Vec<_>>()
                                        .join("; ");
                                    aggregate.mark_task_needs_repair(task_id, &reasons_summary)?;

                                    // Invoke failure resolver prompt
                                    let (fail_prompt, fail_prov) =
                                        self.prompt_provider.render_prompt(
                                            "migration.failure-resolver",
                                            &[("context_path", &context_path)],
                                        )?;
                                    let fail_res = self
                                        .agent_backend
                                        .execute_prompt(&fail_prompt, &fail_prov)?;
                                    eval_metrics.record_agent_invocation(
                                        &fail_prompt,
                                        &fail_res.log_summary,
                                    );
                                    if !fail_res.success {
                                        let reason = format!(
                                            "Failure resolver agent failed: {}",
                                            fail_res.log_summary
                                        );
                                        aggregate.mark_task_failed(task_id, &reason)?;
                                        return Err(DomainError::PortError(reason));
                                    }
                                    // Retry loop continues to attempt repair
                                }
                                RecoveryDecision::Block => {
                                    let reasons_summary = reasons
                                        .iter()
                                        .map(|r| r.to_string())
                                        .collect::<Vec<_>>()
                                        .join("; ");
                                    let task_mut = aggregate.get_task_mut(task_id).unwrap();
                                    let _ = task_mut.mark_blocked(&reasons_summary);
                                    break;
                                }
                                RecoveryDecision::Escalate | RecoveryDecision::Replan => {
                                    let reasons_summary = reasons
                                        .iter()
                                        .map(|r| r.to_string())
                                        .collect::<Vec<_>>()
                                        .join("; ");
                                    let task_mut = aggregate.get_task_mut(task_id).unwrap();
                                    let _ = task_mut.mark_failed(&reasons_summary);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        // 6. Final completion evaluation and health score calculation
        let completion_status = aggregate.check_completion();
        let health_score = aggregate.health_score();
        let task_states: Vec<(TaskId, TaskState)> = aggregate
            .tasks
            .values()
            .map(|t| (t.id.clone(), t.state))
            .collect();
        eval_metrics.total_wall_clock_duration_us = wall_clock_start.elapsed().as_micros() as u64;

        Ok(MigrationRunReport {
            migration_id: aggregate.id.clone(),
            plan_hash: aggregate.plan_hash.clone(),
            final_target_revision: aggregate.current_target_revision,
            health_score,
            completion_status,
            task_states,
            total_evidence_count: aggregate.evidence_ledger.len(),
            waves_executed,
            repairs_triggered,
            checkpoint_saved,
            eval_metrics,
        })
    }
}
