use std::collections::HashSet;

use mae_domain::*;

#[test]
fn test_plan_hash_determinism_and_sensitivity() {
    let source_fp = "react-src-sha256-abc123";
    let c1 = ContractId::new("C_ROUTE_NAV");
    let c2 = ContractId::new("C_FORM_SUBMIT");
    let contracts = vec![c1.clone(), c2.clone()];
    let cap_digest = "cap-v1-digest";
    let plan_digest = "plan-v1-digest";
    let adapter_versions = "react@0.1.0,slint@0.1.0";

    let hash1 = PlanHash::compute(
        source_fp,
        &contracts,
        cap_digest,
        plan_digest,
        adapter_versions,
    );
    let hash2 = PlanHash::compute(
        source_fp,
        &contracts,
        cap_digest,
        plan_digest,
        adapter_versions,
    );
    assert_eq!(
        hash1, hash2,
        "Identical inputs must yield identical plan hash"
    );

    // Order of contract IDs in input slice must not affect hash (canonical sorting)
    let reversed_contracts = vec![c2.clone(), c1.clone()];
    let hash_reversed = PlanHash::compute(
        source_fp,
        &reversed_contracts,
        cap_digest,
        plan_digest,
        adapter_versions,
    );
    assert_eq!(
        hash1, hash_reversed,
        "Contract ID order must be canonically sorted"
    );

    // Sensitivity to source fingerprint
    let hash_diff_src = PlanHash::compute(
        "diff-fp",
        &contracts,
        cap_digest,
        plan_digest,
        adapter_versions,
    );
    assert_ne!(hash1, hash_diff_src);

    // Sensitivity to contracts
    let c3 = ContractId::new("C_EXTRA");
    let hash_diff_contracts = PlanHash::compute(
        source_fp,
        &[c1.clone(), c3],
        cap_digest,
        plan_digest,
        adapter_versions,
    );
    assert_ne!(hash1, hash_diff_contracts);

    // Sensitivity to capability digest
    let hash_diff_cap = PlanHash::compute(
        source_fp,
        &contracts,
        "cap-v2-digest",
        plan_digest,
        adapter_versions,
    );
    assert_ne!(hash1, hash_diff_cap);

    // Sensitivity to plan digest
    let hash_diff_plan = PlanHash::compute(
        source_fp,
        &contracts,
        cap_digest,
        "plan-v2-digest",
        adapter_versions,
    );
    assert_ne!(hash1, hash_diff_plan);

    // Sensitivity to adapter versions
    let hash_diff_adapters = PlanHash::compute(
        source_fp,
        &contracts,
        cap_digest,
        plan_digest,
        "react@0.2.0",
    );
    assert_ne!(hash1, hash_diff_adapters);
}

#[test]
fn test_content_hash_determinism_and_artifact_integrity() {
    let data1 = b"component Dashboard() { return <div />; }";
    let data2 = b"component Dashboard() { return <div />; }";
    let data3 = b"component Settings() { return <div />; }";

    let h1 = ContentHash::compute(data1);
    let h2 = ContentHash::compute(data2);
    let h3 = ContentHash::compute(data3);

    assert_eq!(h1, h2);
    assert_ne!(h1, h3);
    assert_eq!(
        h1.as_str().len(),
        32,
        "FNV-1a 128-bit hex string must be 32 chars"
    );

    let prov =
        ArtifactProvenance::static_deterministic("test-analyzer", ArtifactRevision::initial());
    let artifact = VersionedArtifact::new(
        ArtifactId::new("art-1"),
        ArtifactKind::ComponentGraph,
        ArtifactRevision::initial(),
        prov,
        data1.to_vec(),
    );

    assert!(artifact.verify_integrity());
}

#[test]
fn test_artifact_revision_progression() {
    let r1 = ArtifactRevision::initial();
    assert_eq!(r1.as_u64(), 1);
    assert!(r1.is_initial());
    assert_eq!(r1.to_string(), "r1");

    let r2 = r1.next();
    assert_eq!(r2.as_u64(), 2);
    assert!(!r2.is_initial());
    assert_eq!(r2.to_string(), "r2");
}

#[test]
fn test_gap_reason_codes_exact_strings() {
    let codes = [
        (GapReasonCode::UiBrowserOnlyApi, "UI_BROWSER_ONLY_API"),
        (GapReasonCode::UiUnsupportedCss, "UI_UNSUPPORTED_CSS"),
        (GapReasonCode::UiDynamicStyle, "UI_DYNAMIC_STYLE"),
        (GapReasonCode::UiThirdPartyWidget, "UI_THIRD_PARTY_WIDGET"),
        (GapReasonCode::UiCanvas, "UI_CANVAS"),
        (GapReasonCode::UiComplexAnimation, "UI_COMPLEX_ANIMATION"),
        (
            GapReasonCode::UiUnresolvedStateLibrary,
            "UI_UNRESOLVED_STATE_LIBRARY",
        ),
        (GapReasonCode::UiUnresolvedRouting, "UI_UNRESOLVED_ROUTING"),
        (GapReasonCode::UiUnresolvedEffect, "UI_UNRESOLVED_EFFECT"),
        (GapReasonCode::UiManualAsset, "UI_MANUAL_ASSET"),
        (GapReasonCode::UiAccessibilityGap, "UI_ACCESSIBILITY_GAP"),
    ];

    for (code, expected_str) in codes {
        assert_eq!(code.as_str(), expected_str);
        assert_eq!(code.to_string(), expected_str);
        assert_eq!(GapReasonCode::parse(expected_str), Some(code));
    }
}

#[test]
fn test_coverage_disclosure_completeness() {
    let scenarios = vec!["s1".to_string(), "s2".to_string()];
    let full = CoverageDisclosure::full(scenarios.clone());
    assert!(full.is_complete());

    let unexecuted = CoverageDisclosure::with_unexecuted(
        scenarios.clone(),
        vec!["s1".to_string()],
        vec!["s1".to_string()],
        vec!["s2".to_string()],
    );
    assert!(!unexecuted.is_complete());

    let failed = CoverageDisclosure::with_unexecuted(
        scenarios.clone(),
        scenarios.clone(),
        vec!["s1".to_string()],
        vec![],
    );
    assert!(!failed.is_complete());
}

#[test]
fn test_task_state_machine_valid_happy_path() {
    let mut task = MigrationTask::new(
        "T1",
        "Migrate dashboard",
        MigrationStrategy::DirectNativeMapping,
    );
    assert_eq!(task.state, TaskState::Pending);

    task.mark_ready().expect("Pending -> Ready must succeed");
    assert_eq!(task.state, TaskState::Ready);

    task.start_running().expect("Ready -> Running must succeed");
    assert_eq!(task.state, TaskState::Running);

    task.start_verifying()
        .expect("Running -> Verifying must succeed");
    assert_eq!(task.state, TaskState::Verifying);
}

#[test]
fn test_task_state_machine_invalid_transitions() {
    let mut task = MigrationTask::new(
        "T1",
        "Migrate dashboard",
        MigrationStrategy::DirectNativeMapping,
    );

    // Cannot start running directly from Pending
    let err = task.start_running().unwrap_err();
    assert!(matches!(
        err,
        DomainError::InvalidStateTransition {
            from: TaskState::Pending,
            to: TaskState::Running,
            ..
        }
    ));

    // Cannot start verifying from Pending
    let err = task.start_verifying().unwrap_err();
    assert!(matches!(
        err,
        DomainError::InvalidStateTransition {
            from: TaskState::Pending,
            to: TaskState::Verifying,
            ..
        }
    ));

    // Move to ready
    task.mark_ready().unwrap();

    // Cannot verify directly from Ready
    let err = task.start_verifying().unwrap_err();
    assert!(matches!(
        err,
        DomainError::InvalidStateTransition {
            from: TaskState::Ready,
            to: TaskState::Verifying,
            ..
        }
    ));
}

#[test]
fn test_task_state_machine_repair_and_retry_limit() {
    let mut task = MigrationTask::new(
        "T1",
        "Migrate component",
        MigrationStrategy::DirectNativeMapping,
    )
    .with_max_retries(2);

    task.mark_ready().unwrap();
    task.start_running().unwrap();
    task.start_verifying().unwrap();

    // Retry 1
    task.mark_needs_repair("visual mismatch")
        .expect("First retry must succeed");
    assert_eq!(task.state, TaskState::NeedsRepair);
    assert_eq!(task.retry_count, 1);

    // Re-enter pipeline
    task.mark_ready().unwrap();
    task.start_running().unwrap();
    task.start_verifying().unwrap();

    // Retry 2
    task.mark_needs_repair("timing error")
        .expect("Second retry must succeed");
    assert_eq!(task.state, TaskState::NeedsRepair);
    assert_eq!(task.retry_count, 2);

    task.mark_ready().unwrap();
    task.start_running().unwrap();
    task.start_verifying().unwrap();

    // Retry 3 (exceeds max_retries = 2)
    let err = task.mark_needs_repair("still failing").unwrap_err();
    assert_eq!(task.state, TaskState::Failed);
    assert!(matches!(
        err,
        DomainError::MaxRetriesExceeded {
            retry_count: 3,
            max_retries: 2,
            ..
        }
    ));
}

#[test]
fn test_task_state_machine_blocking_and_cancellation() {
    let mut task = MigrationTask::new(
        "T1",
        "Migrate component",
        MigrationStrategy::DirectNativeMapping,
    );
    task.mark_blocked("Waiting on design spec").unwrap();
    assert_eq!(task.state, TaskState::Blocked);

    let mut task2 = MigrationTask::new(
        "T2",
        "Cancel component",
        MigrationStrategy::DirectNativeMapping,
    );
    task2.cancel("Requirement dropped").unwrap();
    assert_eq!(task2.state, TaskState::Cancelled);

    // Terminal state cannot be cancelled or failed again
    let err = task2.mark_failed("Already cancelled").unwrap_err();
    assert!(matches!(err, DomainError::InvalidStateTransition { .. }));
}

#[test]
fn test_task_cannot_transition_to_accepted_directly() {
    let mut agg = MigrationAggregate::new("MIG_1", "src-fp");
    let task_id = TaskId::new("T1");
    let task = MigrationTask::new(
        task_id.clone(),
        "Migrate component",
        MigrationStrategy::DirectNativeMapping,
    );
    agg.add_task(task);

    agg.mark_task_ready(&task_id).unwrap();
    agg.start_task_running(&task_id).unwrap();

    // Still running: accept_task must reject because task is not in Verifying state
    let err = agg.accept_task(&task_id).unwrap_err();
    assert!(
        err.contains(&AcceptanceRejectionReason::TaskNotInVerifyingState(
            TaskState::Running
        ))
    );
}

#[test]
fn test_acceptance_policy_rejects_unaccepted_dependency() {
    let dep_id = TaskId::new("DEP_TASK");
    let task = MigrationTask::new(
        "T1",
        "Migrate details view",
        MigrationStrategy::DirectNativeMapping,
    )
    .with_dependency(dep_id.clone());

    let mut task_verifying = task;
    task_verifying.mark_ready().unwrap();
    task_verifying.start_running().unwrap();
    task_verifying.start_verifying().unwrap();

    let accepted_tasks = HashSet::new(); // Empty: dependency not accepted
    let cap_report = CapabilityReport::new();

    let verdict = AcceptancePolicy::evaluate(
        &task_verifying,
        &accepted_tasks,
        ArtifactRevision::initial(),
        &[],
        &cap_report,
        &[],
    );

    assert!(!verdict.is_accepted());
    assert!(verdict
        .reasons()
        .contains(&AcceptanceRejectionReason::DependencyNotAccepted(dep_id)));
}

#[test]
fn test_acceptance_policy_rejects_missing_contract_evidence() {
    let c_id = ContractId::new("C_FORM_SUBMIT");
    let mut task = MigrationTask::new("T1", "Migrate form", MigrationStrategy::DirectNativeMapping)
        .with_required_contract(c_id.clone());
    task.mark_ready().unwrap();
    task.start_running().unwrap();
    task.start_verifying().unwrap();

    let accepted_tasks = HashSet::new();
    let cap_report = CapabilityReport::new();

    let verdict = AcceptancePolicy::evaluate(
        &task,
        &accepted_tasks,
        ArtifactRevision::initial(),
        &[],
        &cap_report,
        &[], // No evidence
    );

    assert!(!verdict.is_accepted());
    assert!(verdict.reasons().contains(
        &AcceptanceRejectionReason::MissingRequiredContractEvidence(c_id)
    ));
}

#[test]
fn test_acceptance_policy_rejects_failed_contract() {
    let c_id = ContractId::new("C_DATA_LOAD");
    let mut task = MigrationTask::new(
        "T1",
        "Migrate data load",
        MigrationStrategy::DirectNativeMapping,
    )
    .with_required_contract(c_id.clone());
    task.mark_ready().unwrap();
    task.start_running().unwrap();
    task.start_verifying().unwrap();

    let evidence = EvidenceRecord::new(
        c_id.clone(),
        "differential-verifier",
        VerificationCategory::Behavioral,
        ArtifactRevision::initial(),
        ArtifactRevision::initial(),
        VerificationStatus::Fail,
        CoverageDisclosure::full(vec!["s1".into()]),
        "Data load returned 500 error simulation",
    );

    let verdict = AcceptancePolicy::evaluate(
        &task,
        &HashSet::new(),
        ArtifactRevision::initial(),
        &[],
        &CapabilityReport::new(),
        &[evidence],
    );

    assert!(!verdict.is_accepted());
    assert!(verdict
        .reasons()
        .contains(&AcceptanceRejectionReason::ContractVerificationFailed(c_id)));
}

#[test]
fn test_acceptance_policy_rejects_inconclusive_or_not_run_contract() {
    let c_id = ContractId::new("C_ANIM");
    let mut task = MigrationTask::new(
        "T1",
        "Migrate animation",
        MigrationStrategy::DirectNativeMapping,
    )
    .with_required_contract(c_id.clone());
    task.mark_ready().unwrap();
    task.start_running().unwrap();
    task.start_verifying().unwrap();

    let evidence = EvidenceRecord::new(
        c_id.clone(),
        "visual-verifier",
        VerificationCategory::Visual,
        ArtifactRevision::initial(),
        ArtifactRevision::initial(),
        VerificationStatus::Inconclusive,
        CoverageDisclosure::full(vec!["s1".into()]),
        "Rendering timed out before frame capture",
    );

    let verdict = AcceptancePolicy::evaluate(
        &task,
        &HashSet::new(),
        ArtifactRevision::initial(),
        &[],
        &CapabilityReport::new(),
        &[evidence],
    );

    assert!(!verdict.is_accepted());
    assert!(verdict.reasons().contains(
        &AcceptanceRejectionReason::ContractVerificationInconclusiveOrNotRun(
            c_id,
            VerificationStatus::Inconclusive,
        )
    ));
}

#[test]
fn test_acceptance_policy_rejects_stale_target_revision_evidence() {
    let c_id = ContractId::new("C_STATE_COUNTER");
    let mut task = MigrationTask::new(
        "T1",
        "Migrate counter",
        MigrationStrategy::DirectNativeMapping,
    )
    .with_required_contract(c_id.clone());
    task.mark_ready().unwrap();
    task.start_running().unwrap();
    task.start_verifying().unwrap();

    let r1 = ArtifactRevision::new(1);
    let r2 = ArtifactRevision::new(2); // Current target is r2, but evidence is for r1!

    let stale_evidence = EvidenceRecord::new(
        c_id.clone(),
        "differential-verifier",
        VerificationCategory::Behavioral,
        ArtifactRevision::initial(),
        r1,
        VerificationStatus::Pass,
        CoverageDisclosure::full(vec!["s1".into()]),
        "Counter passes on r1",
    );

    let verdict = AcceptancePolicy::evaluate(
        &task,
        &HashSet::new(),
        r2,
        &[],
        &CapabilityReport::new(),
        &[stale_evidence],
    );

    assert!(!verdict.is_accepted());
    assert!(verdict.reasons().contains(
        &AcceptanceRejectionReason::StaleEvidenceForTargetRevision {
            contract_id: c_id,
            evidence_rev: r1,
            current_rev: r2,
        }
    ));
}

#[test]
fn test_acceptance_policy_rejects_partial_scenario_coverage() {
    let c_id = ContractId::new("C_EDGE_CASES");
    let mut task = MigrationTask::new(
        "T1",
        "Migrate edge cases",
        MigrationStrategy::DirectNativeMapping,
    )
    .with_required_contract(c_id.clone());
    task.mark_ready().unwrap();
    task.start_running().unwrap();
    task.start_verifying().unwrap();

    let partial_coverage = CoverageDisclosure::with_unexecuted(
        vec!["scenario_a".into(), "scenario_b".into()],
        vec!["scenario_a".into()],
        vec!["scenario_a".into()],
        vec!["scenario_b".into()], // scenario_b was skipped!
    );

    let evidence = EvidenceRecord::new(
        c_id.clone(),
        "differential-verifier",
        VerificationCategory::Behavioral,
        ArtifactRevision::initial(),
        ArtifactRevision::initial(),
        VerificationStatus::Pass,
        partial_coverage,
        "Ran 1 of 2 scenarios",
    );

    let verdict = AcceptancePolicy::evaluate(
        &task,
        &HashSet::new(),
        ArtifactRevision::initial(),
        &[],
        &CapabilityReport::new(),
        &[evidence],
    );

    assert!(!verdict.is_accepted());
    assert!(verdict
        .reasons()
        .contains(&AcceptanceRejectionReason::PartialScenarioCoverage(c_id)));
}

#[test]
fn test_acceptance_policy_rejects_native_compliance_failure() {
    let mut task = MigrationTask::new("T1", "Migrate view", MigrationStrategy::DirectNativeMapping);
    task.mark_ready().unwrap();
    task.start_running().unwrap();
    task.start_verifying().unwrap();

    // Native compliance verifier caught a WebView wrapper cheating!
    let native_fail = EvidenceRecord::new(
        ContractId::new("C_NATIVE_WINDOW"),
        "native-compliance-verifier",
        VerificationCategory::NativeCompliance,
        ArtifactRevision::initial(),
        ArtifactRevision::initial(),
        VerificationStatus::Fail,
        CoverageDisclosure::full(vec!["inspect_binary".into()]),
        "Cheating detected: wry WebView2 wrapper detected in cargo dependencies",
    );

    let verdict = AcceptancePolicy::evaluate(
        &task,
        &HashSet::new(),
        ArtifactRevision::initial(),
        &[],
        &CapabilityReport::new(),
        &[native_fail],
    );

    assert!(!verdict.is_accepted());
    assert!(verdict
        .reasons()
        .contains(&AcceptanceRejectionReason::NativeComplianceFailed));
}

#[test]
fn test_acceptance_policy_rejects_critical_capability_gap() {
    let mut task = MigrationTask::new(
        "T1",
        "Migrate canvas component",
        MigrationStrategy::DirectNativeMapping,
    );
    task.mark_ready().unwrap();
    task.start_running().unwrap();
    task.start_verifying().unwrap();

    let mut cap_report = CapabilityReport::new();
    let critical_gap = MigrationGap::new(
        "src/Chart.tsx:24",
        GapReasonCode::UiCanvas,
        ContractSeverity::Critical,
        MigrationStrategy::Unsupported,
        "HTML5 canvas requires custom Slint rendering backend",
    );
    cap_report.add_gap(critical_gap);

    let verdict = AcceptancePolicy::evaluate(
        &task,
        &HashSet::new(),
        ArtifactRevision::initial(),
        &[],
        &cap_report,
        &[],
    );

    assert!(!verdict.is_accepted());
    assert!(verdict
        .reasons()
        .contains(&AcceptanceRejectionReason::CriticalCapabilityGapExists));
}

#[test]
fn test_acceptance_policy_rejects_task_not_in_verifying_state() {
    let mut task = MigrationTask::new(
        "T1",
        "Migrate component",
        MigrationStrategy::DirectNativeMapping,
    );
    task.mark_ready().unwrap();
    task.start_running().unwrap();
    // Still running, not yet verifying

    let verdict = AcceptancePolicy::evaluate(
        &task,
        &HashSet::new(),
        ArtifactRevision::initial(),
        &[],
        &CapabilityReport::new(),
        &[],
    );

    assert!(!verdict.is_accepted());
    assert!(verdict
        .reasons()
        .contains(&AcceptanceRejectionReason::TaskNotInVerifyingState(
            TaskState::Running
        )));
}

#[test]
fn test_acceptance_policy_succeeds_when_all_invariants_met() {
    let dep_id = TaskId::new("DEP_TASK");
    let c_id = ContractId::new("C_BUTTON_CLICK");

    let mut task = MigrationTask::new(
        "T1",
        "Migrate button",
        MigrationStrategy::DirectNativeMapping,
    )
    .with_dependency(dep_id.clone())
    .with_required_contract(c_id.clone());
    task.mark_ready().unwrap();
    task.start_running().unwrap();
    task.start_verifying().unwrap();

    let mut accepted_tasks = HashSet::new();
    accepted_tasks.insert(dep_id);

    let evidence = EvidenceRecord::new(
        c_id,
        "behavioral-differential",
        VerificationCategory::Behavioral,
        ArtifactRevision::initial(),
        ArtifactRevision::initial(),
        VerificationStatus::Pass,
        CoverageDisclosure::full(vec!["click_inc".into()]),
        "Behavior perfectly matches React source",
    );

    let verdict = AcceptancePolicy::evaluate(
        &task,
        &accepted_tasks,
        ArtifactRevision::initial(),
        &[],
        &CapabilityReport::new(),
        &[evidence],
    );

    assert!(verdict.is_accepted());
    assert!(verdict.reasons().is_empty());
}

#[test]
fn test_aggregate_advancing_target_revision_invalidates_prior_evidence() {
    let mut agg = MigrationAggregate::new("MIG_1", "src-fp");
    let c_id = ContractId::new("C_1");
    let task_id = TaskId::new("T_1");

    let mut task = MigrationTask::new(
        task_id.clone(),
        "Task 1",
        MigrationStrategy::DirectNativeMapping,
    )
    .with_required_contract(c_id.clone());
    task.mark_ready().unwrap();
    task.start_running().unwrap();
    task.start_verifying().unwrap();
    agg.add_task(task);

    // Record evidence on initial revision r1
    let evidence_r1 = EvidenceRecord::new(
        c_id.clone(),
        "verifier",
        VerificationCategory::Behavioral,
        ArtifactRevision::initial(),
        ArtifactRevision::initial(),
        VerificationStatus::Pass,
        CoverageDisclosure::full(vec!["s1".into()]),
        "Pass on r1",
    );
    agg.record_evidence(evidence_r1);

    // Advance revision to r2 (target code changed!)
    let r2 = agg.advance_target_revision();
    assert_eq!(r2.as_u64(), 2);

    // Trying to accept task with r1 evidence must fail with StaleEvidence
    let result = agg.accept_task(&task_id);
    assert!(result.is_err());
    let reasons = result.unwrap_err();
    assert!(
        reasons.contains(&AcceptanceRejectionReason::StaleEvidenceForTargetRevision {
            contract_id: c_id.clone(),
            evidence_rev: ArtifactRevision::initial(),
            current_rev: r2,
        })
    );

    // Record fresh evidence for r2
    let evidence_r2 = EvidenceRecord::new(
        c_id,
        "verifier",
        VerificationCategory::Behavioral,
        ArtifactRevision::initial(),
        r2,
        VerificationStatus::Pass,
        CoverageDisclosure::full(vec!["s1".into()]),
        "Pass on r2",
    );
    agg.record_evidence(evidence_r2);

    // Now acceptance succeeds
    let ok = agg.accept_task(&task_id);
    assert!(ok.is_ok());
    assert_eq!(agg.get_task(&task_id).unwrap().state, TaskState::Accepted);
}

#[test]
fn test_aggregate_events_emission() {
    let mut agg = MigrationAggregate::new("MIG_1", "src-fp");
    let plan_hash = PlanHash::new("canonical-hash-123");
    agg.lock_plan_hash(plan_hash.clone());

    let task_id = TaskId::new("T_1");
    let task = MigrationTask::new(
        task_id.clone(),
        "Task 1",
        MigrationStrategy::DirectNativeMapping,
    );
    agg.add_task(task);

    agg.mark_task_ready(&task_id).unwrap();
    agg.start_task_running(&task_id).unwrap();
    agg.start_task_verifying(&task_id).unwrap();

    let events = agg.take_events();
    assert_eq!(events.len(), 4);
    assert_eq!(events[0], DomainEvent::PlanHashLocked { plan_hash });
    assert_eq!(
        events[1],
        DomainEvent::TaskStateChanged {
            task_id: task_id.clone(),
            from: TaskState::Pending,
            to: TaskState::Ready
        }
    );
    assert_eq!(
        events[2],
        DomainEvent::TaskStateChanged {
            task_id: task_id.clone(),
            from: TaskState::Ready,
            to: TaskState::Running
        }
    );
    assert_eq!(
        events[3],
        DomainEvent::TaskStateChanged {
            task_id,
            from: TaskState::Running,
            to: TaskState::Verifying
        }
    );
}

#[test]
fn test_health_score_progress_and_correctness_gate() {
    let c_crit = Contract::new(
        "C_CRIT",
        "Critical",
        "",
        VerificationCategory::Behavioral,
        ContractSeverity::Critical,
    );
    let mut t1 = MigrationTask::new("T1", "Task 1", MigrationStrategy::DirectNativeMapping);
    let mut t2 = MigrationTask::new("T2", "Task 2", MigrationStrategy::DirectNativeMapping);
    t1.state = TaskState::Accepted;
    t2.state = TaskState::Pending;

    // Progress is 50%, no failures -> health is 0.5
    let hs = HealthScorePolicy::compute(
        &[t1.clone(), t2.clone()],
        std::slice::from_ref(&c_crit),
        &[],
        ArtifactRevision::initial(),
    );
    assert_eq!(hs.progress, 0.5);
    assert_eq!(hs.correctness_gate, 1);
    assert_eq!(hs.score, 0.5);

    // Critical contract has Fail on current revision -> correctness gate drops to 0!
    let fail_ev = EvidenceRecord::new(
        c_crit.id.clone(),
        "verifier",
        VerificationCategory::Behavioral,
        ArtifactRevision::initial(),
        ArtifactRevision::initial(),
        VerificationStatus::Fail,
        CoverageDisclosure::full(vec!["s1".into()]),
        "Failure",
    );

    let hs_fail = HealthScorePolicy::compute(
        &[t1.clone(), t2.clone()],
        std::slice::from_ref(&c_crit),
        &[fail_ev],
        ArtifactRevision::initial(),
    );
    assert_eq!(hs_fail.progress, 0.5);
    assert_eq!(hs_fail.correctness_gate, 0);
    assert_eq!(
        hs_fail.score, 0.0,
        "Score must drop to 0 when critical gate fails"
    );

    // Native compliance failure also drops correctness gate to 0!
    let native_fail = EvidenceRecord::new(
        ContractId::new("C_NAT"),
        "verifier",
        VerificationCategory::NativeCompliance,
        ArtifactRevision::initial(),
        ArtifactRevision::initial(),
        VerificationStatus::Fail,
        CoverageDisclosure::full(vec!["s1".into()]),
        "Cheating",
    );
    let hs_native_fail = HealthScorePolicy::compute(
        &[t1, t2],
        std::slice::from_ref(&c_crit),
        &[native_fail],
        ArtifactRevision::initial(),
    );
    assert_eq!(hs_native_fail.correctness_gate, 0);
    assert_eq!(hs_native_fail.score, 0.0);
}

#[test]
fn test_completion_policy_gates() {
    let mut t1 = MigrationTask::new("T1", "Task 1", MigrationStrategy::DirectNativeMapping);
    let c_crit = Contract::new(
        "C_CRIT",
        "Crit",
        "",
        VerificationCategory::Behavioral,
        ContractSeverity::Critical,
    );
    let cap_report = CapabilityReport::new();

    // 1. Task not accepted -> fails
    let res = CompletionPolicy::evaluate(
        &[t1.clone()],
        std::slice::from_ref(&c_crit),
        &[],
        ArtifactRevision::initial(),
        &cap_report,
    );
    assert!(res.is_err());
    assert!(res.unwrap_err()[0].contains("Unaccepted tasks remain"));

    // Accept task
    t1.state = TaskState::Accepted;

    // 2. Critical contract lacking Pass evidence -> fails
    let res = CompletionPolicy::evaluate(
        &[t1.clone()],
        std::slice::from_ref(&c_crit),
        &[],
        ArtifactRevision::initial(),
        &cap_report,
    );
    assert!(res.is_err());

    // Add critical contract pass evidence
    let crit_pass = EvidenceRecord::new(
        c_crit.id.clone(),
        "verifier",
        VerificationCategory::Behavioral,
        ArtifactRevision::initial(),
        ArtifactRevision::initial(),
        VerificationStatus::Pass,
        CoverageDisclosure::full(vec!["s1".into()]),
        "Passed",
    );

    // 3. Build gate missing -> fails
    let res = CompletionPolicy::evaluate(
        &[t1.clone()],
        std::slice::from_ref(&c_crit),
        std::slice::from_ref(&crit_pass),
        ArtifactRevision::initial(),
        &cap_report,
    );
    assert!(res.is_err());

    let build_pass = EvidenceRecord::new(
        ContractId::new("C_BUILD"),
        "build-verifier",
        VerificationCategory::Build,
        ArtifactRevision::initial(),
        ArtifactRevision::initial(),
        VerificationStatus::Pass,
        CoverageDisclosure::full(vec!["build".into()]),
        "Build clean",
    );

    let native_pass = EvidenceRecord::new(
        ContractId::new("C_NATIVE"),
        "native-verifier",
        VerificationCategory::NativeCompliance,
        ArtifactRevision::initial(),
        ArtifactRevision::initial(),
        VerificationStatus::Pass,
        CoverageDisclosure::full(vec!["native".into()]),
        "Native clean",
    );

    // All gates satisfied!
    let all_evidence = vec![crit_pass, build_pass, native_pass];
    let res = CompletionPolicy::evaluate(
        &[t1],
        &[c_crit],
        &all_evidence,
        ArtifactRevision::initial(),
        &cap_report,
    );
    assert!(
        res.is_ok(),
        "CompletionPolicy must pass when all gates are satisfied"
    );
}

#[test]
fn test_recovery_policy_decisions() {
    // Implementation defect within retry limit -> Repair
    assert_eq!(
        RecoveryPolicy::decide(FailureKind::ImplementationDefect, 1, 3),
        RecoveryDecision::Repair
    );
    // Implementation defect at retry limit -> Escalate
    assert_eq!(
        RecoveryPolicy::decide(FailureKind::ImplementationDefect, 3, 3),
        RecoveryDecision::Escalate
    );
    // Stale evidence -> Repair
    assert_eq!(
        RecoveryPolicy::decide(FailureKind::StaleEvidence, 0, 3),
        RecoveryDecision::Repair
    );
    // Unsupported capability -> Replan
    assert_eq!(
        RecoveryPolicy::decide(FailureKind::UnsupportedCapability, 0, 3),
        RecoveryDecision::Replan
    );
    // Ambiguous semantics -> Escalate
    assert_eq!(
        RecoveryPolicy::decide(FailureKind::AmbiguousSemantics, 0, 3),
        RecoveryDecision::Escalate
    );
    // Verifier defect -> Escalate
    assert_eq!(
        RecoveryPolicy::decide(FailureKind::VerifierDefect, 0, 3),
        RecoveryDecision::Escalate
    );
    // Infrastructure failure -> Block
    assert_eq!(
        RecoveryPolicy::decide(FailureKind::InfrastructureFailure, 0, 3),
        RecoveryDecision::Block
    );
}

#[test]
fn test_semantic_ui_model_composition() {
    let mut model = SemanticUiModel::new(TargetPlatform::WindowsDesktop);

    let comp = ComponentNode::new("comp_1", "DashboardView", "src/Dashboard.tsx")
        .with_prop(PropDefinition::new("title", "string", true))
        .with_child("comp_2");
    model.add_component(comp);

    let state = StateField::new("counter", "i32", "0", "useState");
    model.add_state_field(state);

    let effect = EffectBinding::new("eff_1", "useEffect", true).with_dependency("counter");
    model.add_effect(effect);

    let interaction = InteractionBinding::new("comp_1", "click", "incrementCounter")
        .with_transition("counter += 1");
    model.add_interaction(interaction);

    let route = RouteDefinition::new("/dashboard", "DashboardView").with_param("tab");
    model.add_route(route);

    let layout =
        LayoutConstraint::new("column", 8.0, 16.0).with_responsive_rule("min-width: 600px");
    model.add_layout_constraint(layout);

    let token = StyleToken::new("primary-color", "#0078D4");
    model.add_style_token(token);

    let asset = AssetEntry::new("logo", "assets/logo.png", "image/png");
    model.add_asset(asset);

    assert_eq!(model.platform, TargetPlatform::WindowsDesktop);
    assert_eq!(model.components.len(), 1);
    assert_eq!(model.state_fields.len(), 1);
    assert_eq!(model.effects.len(), 1);
    assert_eq!(model.interactions.len(), 1);
    assert_eq!(model.routes.len(), 1);
    assert_eq!(model.layout_constraints.len(), 1);
    assert_eq!(model.style_tokens.len(), 1);
    assert_eq!(model.assets.len(), 1);
}

#[test]
fn test_ports_clean_architecture_mock_implementations() {
    struct MockSourceAdapter;
    impl SourceAdapterPort for MockSourceAdapter {
        fn discover(&self, _path: &str) -> Result<VersionedArtifact, DomainError> {
            Ok(VersionedArtifact::from_string(
                "source-inv",
                ArtifactKind::SourceInventory,
                ArtifactRevision::initial(),
                ArtifactProvenance::static_deterministic("mock", ArtifactRevision::initial()),
                "{}",
            ))
        }

        fn extract_semantics(&self, _path: &str) -> Result<SemanticUiModel, DomainError> {
            Ok(SemanticUiModel::new(TargetPlatform::WindowsDesktop))
        }

        fn assess_capabilities(
            &self,
            _model: &SemanticUiModel,
        ) -> Result<CapabilityReport, DomainError> {
            Ok(CapabilityReport::new())
        }
    }

    let adapter = MockSourceAdapter;
    let artifact = adapter.discover("dummy/path").unwrap();
    assert_eq!(artifact.kind, ArtifactKind::SourceInventory);
}

#[test]
fn test_migration_eval_metrics_and_token_estimation() {
    assert_eq!(estimate_token_count(""), 0);
    assert!(estimate_token_count("Migrate React component to Slint UI") >= 6);

    let mut metrics = MigrationEvalMetrics::new();
    metrics.record_agent_invocation("Prompt text here with words", "Completion output summary");
    assert_eq!(metrics.agent_invocations_count, 1);
    assert!(metrics.prompt_tokens_estimated > 0);
    assert!(metrics.completion_tokens_estimated > 0);
    assert_eq!(
        metrics.total_tokens_estimated,
        metrics.prompt_tokens_estimated + metrics.completion_tokens_estimated
    );

    metrics.component_name_consistency_ratio = 1.0;
    metrics.route_consistency_ratio = 1.0;
    metrics.state_field_consistency_ratio = 1.0;
    metrics.ui_element_parity_ratio = 1.0;
    metrics.architecture_consistency_score = 1.0;
    assert_eq!(metrics.overall_consistency_score(), 1.0);

    let json = metrics.to_json_report();
    assert!(json.contains("\"total_tokens_estimated\":"));
    assert!(json.contains("\"overall_consistency_score\": 1.0000"));
}
