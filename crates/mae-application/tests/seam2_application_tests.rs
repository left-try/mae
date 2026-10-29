use std::fs;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use mae_application::{
    CheckpointManager, CheckpointSnapshot, ClaudeCodeAgentBackend, CodexAgentBackend,
    CopilotAgentBackend, DeterministicMockAgentBackend, ExecutionMode,
    FileSystemArtifactRepository, InMemoryArtifactRepository, LegacyPromptProvider,
    MigrationOrchestrator, OkfKnowledgeRepository, OpfPrompt, OpfPromptProvider, WaveSchedule,
};
use mae_domain::{
    AgentBackendPort, ArtifactId, ArtifactKind, ArtifactProvenance, ArtifactRepositoryPort,
    ArtifactRevision, CapabilityReport, ContentHash, Contract, ContractSeverity,
    CoverageDisclosure, DomainError, EvidenceRecord, KnowledgeRepositoryPort, MigrationAggregate,
    MigrationId, MigrationStrategy, MigrationTask, PlanHash, PromptProviderPort, SemanticUiModel,
    SourceAdapterPort, TargetAdapterPort, TargetPlatform, TargetScaffoldSummary, TaskId, TaskState,
    VerificationCategory, VerificationStatus, VerifierPluginPort, VersionedArtifact,
};

fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("{}_{}", prefix, now));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn workspace_root() -> std::path::PathBuf {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidate = manifest_dir.join("../..");
    if candidate.join("opf.yaml").exists() {
        candidate
    } else {
        std::path::PathBuf::from(".")
    }
}

// ---------------------------------------------------------------------------
// Mock Adapters for Orchestrator Tests
// ---------------------------------------------------------------------------

struct MockSourceAdapter;

impl SourceAdapterPort for MockSourceAdapter {
    fn discover(&self, _source_path: &str) -> Result<VersionedArtifact, DomainError> {
        Ok(VersionedArtifact::new(
            ArtifactId::new("source-inventory"),
            ArtifactKind::SourceInventory,
            ArtifactRevision::initial(),
            ArtifactProvenance::static_deterministic("mock-source", ArtifactRevision::initial()),
            b"mock-source-inventory-content".to_vec(),
        ))
    }

    fn extract_semantics(&self, _source_path: &str) -> Result<SemanticUiModel, DomainError> {
        Ok(SemanticUiModel::new(TargetPlatform::WindowsDesktop))
    }

    fn assess_capabilities(
        &self,
        _model: &SemanticUiModel,
    ) -> Result<CapabilityReport, DomainError> {
        Ok(CapabilityReport::new())
    }
}

struct MockTargetAdapter;

impl TargetAdapterPort for MockTargetAdapter {
    fn scaffold_target(
        &self,
        _model: &SemanticUiModel,
        _target_dir: &str,
    ) -> Result<TargetScaffoldSummary, DomainError> {
        Ok(TargetScaffoldSummary {
            files_generated: vec![
                "Cargo.toml".into(),
                "src/main.rs".into(),
                "ui/app.slint".into(),
            ],
            target_crate_name: "native_app".into(),
            has_windows_subsystem: true,
        })
    }

    fn emit_target_code(
        &self,
        task: &MigrationTask,
        _model: &SemanticUiModel,
        _target_dir: &str,
    ) -> Result<ArtifactRevision, DomainError> {
        Ok(ArtifactRevision::new((task.retry_count + 1) as u64))
    }
}

struct AlwaysPassVerifier {
    category: VerificationCategory,
}

impl AlwaysPassVerifier {
    fn new(category: VerificationCategory) -> Self {
        Self { category }
    }
}

impl VerifierPluginPort for AlwaysPassVerifier {
    fn category(&self) -> VerificationCategory {
        self.category
    }

    fn verify(
        &self,
        contract: &Contract,
        target_rev: ArtifactRevision,
    ) -> Result<EvidenceRecord, DomainError> {
        Ok(EvidenceRecord::new(
            contract.id.clone(),
            format!("verifier-{}", self.category.as_str()),
            self.category,
            ArtifactRevision::initial(),
            target_rev,
            VerificationStatus::Pass,
            CoverageDisclosure::full(vec!["scenario-1".into()]),
            "Passed successfully",
        ))
    }
}

struct FlakyThenPassVerifier {
    category: VerificationCategory,
    attempt: AtomicU32,
}

impl FlakyThenPassVerifier {
    fn new(category: VerificationCategory) -> Self {
        Self {
            category,
            attempt: AtomicU32::new(0),
        }
    }
}

impl VerifierPluginPort for FlakyThenPassVerifier {
    fn category(&self) -> VerificationCategory {
        self.category
    }

    fn verify(
        &self,
        contract: &Contract,
        target_rev: ArtifactRevision,
    ) -> Result<EvidenceRecord, DomainError> {
        let count = self.attempt.fetch_add(1, Ordering::SeqCst);
        if count == 0 {
            Ok(EvidenceRecord::new(
                contract.id.clone(),
                "flaky-verifier",
                self.category,
                ArtifactRevision::initial(),
                target_rev,
                VerificationStatus::Fail,
                CoverageDisclosure::full(vec!["scenario-1".into()]),
                "Deliberate test failure on first attempt",
            ))
        } else {
            Ok(EvidenceRecord::new(
                contract.id.clone(),
                "flaky-verifier",
                self.category,
                ArtifactRevision::initial(),
                target_rev,
                VerificationStatus::Pass,
                CoverageDisclosure::full(vec!["scenario-1".into()]),
                "Passed on repair attempt",
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// 1. OPF Prompt Provider Tests
// ---------------------------------------------------------------------------

#[test]
fn test_opf_prompt_provider_loads_real_files_from_root() {
    let provider =
        OpfPromptProvider::from_root(workspace_root()).expect("Failed to load real opf.yaml");

    let required_prompts = [
        "migration.planner",
        "migration.code-migrator",
        "migration.parity-verifier",
        "migration.failure-resolver",
        "migration.ui-semantic-analyzer",
        "migration.implementation-style",
    ];

    for name in &required_prompts {
        let prompt_source = provider
            .get_prompt(name)
            .unwrap_or_else(|_| panic!("Expected prompt '{}' to exist", name));
        assert_eq!(prompt_source.id, *name);
        assert!(!prompt_source.template.is_empty());
        assert!(
            prompt_source
                .variables
                .contains(&"context_path".to_string()),
            "Prompt {} missing context_path",
            name
        );
    }

    // Render migration.planner with context_path
    let (rendered, provenance) = provider
        .render_prompt(
            "migration.planner",
            &[("context_path", "test/context/planner.json")],
        )
        .expect("Rendering planner prompt should succeed");

    assert!(rendered.contains("test/context/planner.json"));
    assert_eq!(provenance.prompt_id, "migration.planner");
    assert!(!provenance.content_digest.is_empty());
    assert_eq!(
        provenance.rendered_vars,
        vec![(
            "context_path".to_string(),
            "test/context/planner.json".to_string()
        )]
    );

    // Missing required variable fails deterministically
    let missing_err = provider.render_prompt("migration.planner", &[]);
    assert!(missing_err.is_err());
    match missing_err.unwrap_err() {
        DomainError::PortError(msg) => {
            assert!(msg.contains("Missing required input variable 'context_path'"));
        }
        other => panic!("Expected PortError, got {:?}", other),
    }

    // Non-existent prompt fails
    let not_found = provider.get_prompt("non.existent.prompt");
    assert!(not_found.is_err());
}

#[test]
fn test_opf_prompt_frontmatter_validation() {
    // Missing opening delimiter
    let no_delim = "format: opf/0.2\nid: test\n---\nbody";
    assert!(OpfPrompt::parse(no_delim).is_err());

    // Invalid format
    let invalid_format = "---\nformat: opf/0.1\nid: test\n---\nbody";
    let err = OpfPrompt::parse(invalid_format);
    assert!(err.is_err());
    assert!(err.unwrap_err().to_string().contains("expected 'opf/0.2'"));

    // Missing id
    let missing_id = "---\nformat: opf/0.2\ndescription: test\n---\nbody";
    assert!(OpfPrompt::parse(missing_id).is_err());

    // Valid prompt with multiple inputs
    let valid_opf = r#"---
format: opf/0.2
id: custom.prompt
description: A custom prompt test
tags: [test, opf]
inputs:
  target_file:
    type: string
    required: true
  optional_flag:
    type: string
    required: false
---
Hello {{ target_file }}! Flag: {{ optional_flag }}
"#;
    let prompt = OpfPrompt::parse(valid_opf).expect("Should parse valid OPF");
    assert_eq!(prompt.id, "custom.prompt");
    assert_eq!(prompt.required_inputs, vec!["target_file"]);
    assert_eq!(prompt.optional_inputs, vec!["optional_flag"]);

    let (rendered, _) = prompt
        .render(&[("target_file", "app.rs"), ("optional_flag", "true")])
        .unwrap();
    assert_eq!(rendered.trim(), "Hello app.rs! Flag: true");
}

#[test]
fn test_legacy_prompt_provider_compatibility() {
    let mut provider = LegacyPromptProvider::new();
    provider.register_template("legacy.task", "Migrate {{ module_name }} to Slint.");

    let (rendered, prov) = provider
        .render_prompt("legacy.task", &[("module_name", "Header")])
        .unwrap();

    assert_eq!(rendered, "Migrate Header to Slint.");
    assert_eq!(prov.prompt_id, "legacy.task");
    assert!(!prov.content_digest.is_empty());
}

// ---------------------------------------------------------------------------
// 2. OKF Knowledge Repository Tests
// ---------------------------------------------------------------------------

#[test]
fn test_okf_repository_bundle_validation_and_search() {
    let repo = OkfKnowledgeRepository::new(workspace_root());

    // Validate real repository OKF bundle
    let is_valid = repo
        .validate_bundle()
        .expect("validate_bundle should succeed");
    assert!(is_valid, "Repository .okf bundle should be valid");

    // Search query on real curated bundle markdown files
    let results = repo.search("AAMF").expect("search should succeed");
    assert!(!results.is_empty(), "Expected search matches for 'AAMF'");

    for match_line in &results {
        assert!(match_line.contains(".md:"));
    }
}

#[test]
fn test_okf_repository_bundle_rejection_invariants() {
    let temp_dir = unique_temp_dir("okf_test");
    let bundle_dir = temp_dir.join(".okf");
    fs::create_dir_all(&bundle_dir).unwrap();

    let repo = OkfKnowledgeRepository::new(&temp_dir);

    // 1. Missing files fails
    assert!(!repo.validate_bundle().unwrap());

    // Create required files with valid manifest
    fs::write(
        bundle_dir.join("manifest.yaml"),
        "schema_version: 1\nname: test",
    )
    .unwrap();
    fs::write(bundle_dir.join("index.md"), "# Knowledge").unwrap();
    fs::create_dir_all(bundle_dir.join("architecture")).unwrap();
    fs::write(bundle_dir.join("architecture/index.md"), "# Arch").unwrap();
    fs::create_dir_all(bundle_dir.join("application")).unwrap();
    fs::write(bundle_dir.join("application/index.md"), "# App").unwrap();
    fs::create_dir_all(bundle_dir.join("operations")).unwrap();
    fs::write(bundle_dir.join("operations/index.md"), "# Ops").unwrap();
    fs::create_dir_all(bundle_dir.join("changes")).unwrap();
    fs::write(bundle_dir.join("changes/index.md"), "# Changes").unwrap();

    // Now valid!
    assert!(repo.validate_bundle().unwrap());

    // 2. Leaked .okf-index/ artifact must fail validation!
    let prohibited_index = bundle_dir.join(".okf-index");
    fs::create_dir_all(&prohibited_index).unwrap();
    assert!(
        !repo.validate_bundle().unwrap(),
        "Bundle containing .okf-index must fail validation"
    );
}

// ---------------------------------------------------------------------------
// 3. Artifact Repository Tests
// ---------------------------------------------------------------------------

#[test]
fn test_in_memory_artifact_repository_integrity() {
    let mut repo = InMemoryArtifactRepository::new();
    let provenance =
        ArtifactProvenance::static_deterministic("test-gen", ArtifactRevision::initial());

    let artifact = VersionedArtifact::from_string(
        ArtifactId::new("comp-1"),
        ArtifactKind::ComponentGraph,
        ArtifactRevision::initial(),
        provenance,
        "valid artifact content",
    );

    // Store valid artifact
    repo.store(artifact.clone()).unwrap();

    // List revisions
    let revs = repo.list_revisions(&ArtifactId::new("comp-1")).unwrap();
    assert_eq!(revs, vec![ArtifactRevision::initial()]);

    // Get artifact
    let retrieved = repo
        .get(&ArtifactId::new("comp-1"), ArtifactRevision::initial())
        .unwrap();
    assert_eq!(retrieved, Some(artifact));

    // Tampered artifact with corrupt hash fails on store
    let tampered = VersionedArtifact {
        id: ArtifactId::new("tampered"),
        kind: ArtifactKind::StateModel,
        revision: ArtifactRevision::initial(),
        content_hash: ContentHash::new("fake_hash_12345"),
        provenance: ArtifactProvenance::static_deterministic("bad", ArtifactRevision::initial()),
        content: b"corrupted".to_vec(),
    };
    assert!(repo.store(tampered).is_err());
}

#[test]
fn test_filesystem_artifact_repository_persistence_and_integrity() {
    let temp_dir = unique_temp_dir("artifact_fs_test");
    let mut repo = FileSystemArtifactRepository::new(&temp_dir);

    let provenance =
        ArtifactProvenance::static_deterministic("test-gen", ArtifactRevision::initial());

    let art1 = VersionedArtifact::from_string(
        ArtifactId::new("ast-model"),
        ArtifactKind::SemanticUiModel,
        ArtifactRevision::new(1),
        provenance.clone(),
        "revision 1 payload",
    );

    let art2 = VersionedArtifact::from_string(
        ArtifactId::new("ast-model"),
        ArtifactKind::SemanticUiModel,
        ArtifactRevision::new(2),
        provenance,
        "revision 2 payload",
    );

    repo.store(art1.clone()).unwrap();
    repo.store(art2.clone()).unwrap();

    let revs = repo.list_revisions(&ArtifactId::new("ast-model")).unwrap();
    assert_eq!(
        revs,
        vec![ArtifactRevision::new(1), ArtifactRevision::new(2)]
    );

    let fetched = repo
        .get(&ArtifactId::new("ast-model"), ArtifactRevision::new(2))
        .unwrap();
    assert_eq!(fetched, Some(art2));
}

// ---------------------------------------------------------------------------
// 4. Wave Schedule & Cycle Detection Tests
// ---------------------------------------------------------------------------

#[test]
fn test_wave_schedule_topological_diamond_dag() {
    let t1 = MigrationTask::new("t1", "Init", MigrationStrategy::DirectNativeMapping);
    let t2 = MigrationTask::new("t2", "Left", MigrationStrategy::DirectNativeMapping)
        .with_dependency("t1");
    let t3 = MigrationTask::new("t3", "Right", MigrationStrategy::DirectNativeMapping)
        .with_dependency("t1");
    let t4 = MigrationTask::new("t4", "Merge", MigrationStrategy::DirectNativeMapping)
        .with_dependencies(vec![TaskId::new("t2"), TaskId::new("t3")]);

    let tasks = vec![t4, t2, t1, t3];
    let waves = WaveSchedule::compute_waves(&tasks).expect("Topological sort should succeed");

    assert_eq!(waves.len(), 3);
    assert_eq!(waves[0], vec![TaskId::new("t1")]);
    assert_eq!(waves[1], vec![TaskId::new("t2"), TaskId::new("t3")]);
    assert_eq!(waves[2], vec![TaskId::new("t4")]);
}

#[test]
fn test_wave_schedule_detects_cycles() {
    let t1 = MigrationTask::new("t1", "Cycle A", MigrationStrategy::DirectNativeMapping)
        .with_dependency("t2");
    let t2 = MigrationTask::new("t2", "Cycle B", MigrationStrategy::DirectNativeMapping)
        .with_dependency("t1");

    let result = WaveSchedule::compute_waves(&[t1, t2]);
    assert!(result.is_err());
    match result.unwrap_err() {
        DomainError::PortError(msg) => {
            assert!(msg.contains("Dependency cycle"));
        }
        other => panic!("Expected PortError with cycle, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// 5. Checkpoint Manager Tests
// ---------------------------------------------------------------------------

#[test]
fn test_checkpoint_manager_roundtrip_and_persistence() {
    let temp_dir = unique_temp_dir("checkpoint_test");
    let cp_file = temp_dir.join("checkpoint.json");

    let snapshot = CheckpointSnapshot {
        migration_id: MigrationId::new("mig-42"),
        plan_hash: Some(PlanHash::new("hash-abc-123")),
        current_target_revision: ArtifactRevision::new(5),
        task_states: vec![
            (TaskId::new("task-1"), TaskState::Accepted, 0),
            (TaskId::new("task-2"), TaskState::NeedsRepair, 2),
            (TaskId::new("task-3"), TaskState::Pending, 0),
        ],
        accepted_tasks: vec![TaskId::new("task-1")],
    };

    CheckpointManager::save_checkpoint(&cp_file, &snapshot)
        .expect("Failed to save checkpoint to disk");

    let loaded =
        CheckpointManager::load_checkpoint(&cp_file).expect("Failed to load checkpoint from disk");

    assert_eq!(loaded, snapshot);

    // Apply to aggregate
    let mut aggregate = MigrationAggregate::new("mig-42", "source-fp");
    let t1 = MigrationTask::new("task-1", "T1", MigrationStrategy::DirectNativeMapping);
    let t2 = MigrationTask::new("task-2", "T2", MigrationStrategy::DirectNativeMapping);
    let t3 = MigrationTask::new("task-3", "T3", MigrationStrategy::DirectNativeMapping);
    aggregate.add_task(t1);
    aggregate.add_task(t2);
    aggregate.add_task(t3);

    loaded.apply_to_aggregate(&mut aggregate).unwrap();

    assert_eq!(aggregate.current_target_revision, ArtifactRevision::new(5));
    assert_eq!(aggregate.plan_hash, Some(PlanHash::new("hash-abc-123")));
    assert_eq!(
        aggregate.get_task(&TaskId::new("task-1")).unwrap().state,
        TaskState::Accepted
    );
    assert_eq!(
        aggregate.get_task(&TaskId::new("task-2")).unwrap().state,
        TaskState::NeedsRepair
    );
    assert_eq!(
        aggregate
            .get_task(&TaskId::new("task-2"))
            .unwrap()
            .retry_count,
        2
    );
}

// ---------------------------------------------------------------------------
// 6. Agent Backend CLI Arguments Mapping
// ---------------------------------------------------------------------------

#[test]
fn test_agent_backend_cli_flag_mappings() {
    let mcp = vec!["server1".to_string(), "server2".to_string()];

    // Codex
    let codex = CodexAgentBackend::new();
    let codex_args = codex.build_cli_args("implement prompt", &mcp);
    assert_eq!(
        codex_args,
        vec![
            "codex",
            "exec",
            "--prompt",
            "implement prompt",
            "--mcp-server",
            "server1",
            "--mcp-server",
            "server2"
        ]
    );

    // Claude Code
    let claude = ClaudeCodeAgentBackend::new();
    let claude_args = claude.build_cli_args("implement prompt", &mcp);
    assert_eq!(
        claude_args,
        vec![
            "claude",
            "-p",
            "implement prompt",
            "--mcp",
            "server1",
            "--mcp",
            "server2"
        ]
    );

    // Copilot
    let copilot = CopilotAgentBackend::new();
    let copilot_args = copilot.build_cli_args("implement prompt", &mcp);
    assert_eq!(
        copilot_args,
        vec![
            "copilot",
            "chat",
            "--prompt",
            "implement prompt",
            "--server",
            "server1",
            "--server",
            "server2"
        ]
    );

    // Mock
    let mock = DeterministicMockAgentBackend::new();
    let mock_args = mock.build_cli_args("implement prompt", &mcp);
    assert_eq!(
        mock_args,
        vec![
            "mock-agent",
            "--prompt",
            "implement prompt",
            "--mcp",
            "server1",
            "--mcp",
            "server2"
        ]
    );
}

#[test]
fn test_unconfigured_provider_backends_do_not_report_prompt_execution_success() {
    let (_, provenance) = OpfPromptProvider::from_root(workspace_root())
        .unwrap()
        .render_prompt(
            "migration.code-migrator",
            &[("context_path", "target/tasks/task-1")],
        )
        .unwrap();

    assert!(CodexAgentBackend::new()
        .execute_prompt("implement", &provenance)
        .is_err());
    assert!(ClaudeCodeAgentBackend::new()
        .execute_prompt("implement", &provenance)
        .is_err());
    assert!(CopilotAgentBackend::new()
        .execute_prompt("implement", &provenance)
        .is_err());
}

// ---------------------------------------------------------------------------
// 7. Full Orchestrator Execution — Happy Path
// ---------------------------------------------------------------------------

#[test]
fn test_orchestrator_does_not_accept_task_when_agent_reports_failure() {
    let mut orchestrator = MigrationOrchestrator::new(
        Box::new(OpfPromptProvider::from_root(workspace_root()).unwrap()),
        Box::new(OkfKnowledgeRepository::new(workspace_root())),
        Box::new(DeterministicMockAgentBackend::new().with_default_success(false)),
        Box::new(MockSourceAdapter),
        Box::new(MockTargetAdapter),
        Box::new(InMemoryArtifactRepository::new()),
        vec![
            Box::new(AlwaysPassVerifier::new(VerificationCategory::Build)),
            Box::new(AlwaysPassVerifier::new(
                VerificationCategory::NativeCompliance,
            )),
            Box::new(AlwaysPassVerifier::new(VerificationCategory::Behavioral)),
        ],
    );
    let mut aggregate = MigrationAggregate::new("mig-agent-failure", "source-fingerprint");
    aggregate.add_contract(Contract::new(
        "contract-build",
        "Target build gate",
        "Must compile cleanly",
        VerificationCategory::Build,
        ContractSeverity::Critical,
    ));
    aggregate.add_contract(Contract::new(
        "contract-native",
        "Native compliance gate",
        "Must be a native target",
        VerificationCategory::NativeCompliance,
        ContractSeverity::Critical,
    ));
    aggregate.add_contract(Contract::new(
        "contract-behavior",
        "Behavioral parity",
        "Must preserve observable behavior",
        VerificationCategory::Behavioral,
        ContractSeverity::Critical,
    ));
    aggregate.add_task(
        MigrationTask::new(
            "task-agent-failure",
            "Generate target",
            MigrationStrategy::DirectNativeMapping,
        )
        .with_required_contract("contract-build")
        .with_required_contract("contract-native")
        .with_required_contract("contract-behavior"),
    );

    let result = orchestrator.run(&mut aggregate, "source", "target");

    assert!(result.is_err(), "agent failure must stop the migration run");
    assert_eq!(
        aggregate
            .get_task(&TaskId::new("task-agent-failure"))
            .unwrap()
            .state,
        TaskState::Failed
    );
    assert!(aggregate.evidence_ledger.is_empty());
}

#[test]
fn test_orchestrator_full_run_happy_path() {
    let prompt_provider = Box::new(
        OpfPromptProvider::from_root(workspace_root()).expect("Failed to load real prompt catalog"),
    );
    let knowledge_repo = Box::new(OkfKnowledgeRepository::new(workspace_root()));
    let agent_backend = Box::new(DeterministicMockAgentBackend::new());
    let source_adapter = Box::new(MockSourceAdapter);
    let target_adapter = Box::new(MockTargetAdapter);
    let artifact_repo = Box::new(InMemoryArtifactRepository::new());

    let verifiers: Vec<Box<dyn VerifierPluginPort>> = vec![
        Box::new(AlwaysPassVerifier::new(VerificationCategory::Build)),
        Box::new(AlwaysPassVerifier::new(
            VerificationCategory::NativeCompliance,
        )),
        Box::new(AlwaysPassVerifier::new(VerificationCategory::Behavioral)),
    ];

    let mut orchestrator = MigrationOrchestrator::new(
        prompt_provider,
        knowledge_repo,
        agent_backend,
        source_adapter,
        target_adapter,
        artifact_repo,
        verifiers,
    )
    .with_execution_mode(ExecutionMode::WaveBarrier);

    let mut aggregate = MigrationAggregate::new("mig-happy-path", "fingerprint-react-123");

    // Add Build, NativeCompliance, and Behavioral contracts
    let build_contract = Contract::new(
        "contract-build",
        "Target build gate",
        "Must compile cleanly",
        VerificationCategory::Build,
        ContractSeverity::Critical,
    );
    let native_contract = Contract::new(
        "contract-native",
        "Native compliance gate",
        "Must not use web wrappers",
        VerificationCategory::NativeCompliance,
        ContractSeverity::Critical,
    );
    let behavior_contract = Contract::new(
        "contract-behavior",
        "Behavioral equivalence",
        "Clicking counter updates display",
        VerificationCategory::Behavioral,
        ContractSeverity::Critical,
    );
    aggregate.add_contract(build_contract);
    aggregate.add_contract(native_contract);
    aggregate.add_contract(behavior_contract.clone());

    // Task 1: foundation
    let t1 = MigrationTask::new(
        "task-1",
        "Foundation",
        MigrationStrategy::DirectNativeMapping,
    )
    .with_required_contract("contract-build")
    .with_required_contract("contract-native");

    // Task 2: interactive component (depends on task-1)
    let t2 = MigrationTask::new(
        "task-2",
        "Counter UI",
        MigrationStrategy::DirectNativeMapping,
    )
    .with_dependency("task-1")
    .with_required_contract("contract-behavior")
    .with_required_contract("contract-build")
    .with_required_contract("contract-native");

    aggregate.add_task(t1);
    aggregate.add_task(t2);

    let temp_target = unique_temp_dir("target_happy");
    let report = orchestrator
        .run(
            &mut aggregate,
            "fixtures/source",
            temp_target.to_str().unwrap(),
        )
        .expect("Orchestrator run should succeed");

    assert!(report.is_successful());
    assert_eq!(report.waves_executed, 2);
    assert_eq!(report.repairs_triggered, 0);
    assert!(report.plan_hash.is_some());
    assert_eq!(report.health_score.score, 1.0);
    assert_eq!(report.health_score.correctness_gate, 1);
    assert_eq!(
        aggregate.get_task(&TaskId::new("task-1")).unwrap().state,
        TaskState::Accepted
    );
    assert_eq!(
        aggregate.get_task(&TaskId::new("task-2")).unwrap().state,
        TaskState::Accepted
    );
}

// ---------------------------------------------------------------------------
// 8. Deliberate Verification Failure -> Repair -> Fresh Evidence -> Accept Cycle
// ---------------------------------------------------------------------------

#[test]
fn test_orchestrator_deliberate_failure_repair_and_acceptance_cycle() {
    let prompt_provider = Box::new(
        OpfPromptProvider::from_root(workspace_root()).expect("Failed to load real prompt catalog"),
    );
    let knowledge_repo = Box::new(OkfKnowledgeRepository::new(workspace_root()));
    let agent_backend = Box::new(DeterministicMockAgentBackend::new());
    let source_adapter = Box::new(MockSourceAdapter);
    let target_adapter = Box::new(MockTargetAdapter);
    let artifact_repo = Box::new(InMemoryArtifactRepository::new());

    // Inject FlakyThenPassVerifier on Behavioral category!
    let verifiers: Vec<Box<dyn VerifierPluginPort>> = vec![
        Box::new(AlwaysPassVerifier::new(VerificationCategory::Build)),
        Box::new(AlwaysPassVerifier::new(
            VerificationCategory::NativeCompliance,
        )),
        Box::new(FlakyThenPassVerifier::new(VerificationCategory::Behavioral)),
    ];

    let temp_checkpoint = unique_temp_dir("cp_repair").join("checkpoint.json");

    let mut orchestrator = MigrationOrchestrator::new(
        prompt_provider,
        knowledge_repo,
        agent_backend,
        source_adapter,
        target_adapter,
        artifact_repo,
        verifiers,
    )
    .with_checkpoint_path(&temp_checkpoint);

    let mut aggregate = MigrationAggregate::new("mig-repair-cycle", "fp-repair-123");

    let build_contract = Contract::new(
        "contract-build",
        "Target build gate",
        "Clean build",
        VerificationCategory::Build,
        ContractSeverity::Critical,
    );
    let native_contract = Contract::new(
        "contract-native",
        "Native compliance gate",
        "No webview wrappers",
        VerificationCategory::NativeCompliance,
        ContractSeverity::Critical,
    );
    let behavior_contract = Contract::new(
        "contract-behavior",
        "Behavioral parity",
        "State matches expectation",
        VerificationCategory::Behavioral,
        ContractSeverity::Critical,
    );

    aggregate.add_contract(build_contract);
    aggregate.add_contract(native_contract);
    aggregate.add_contract(behavior_contract);

    let task = MigrationTask::new(
        "task-interactive",
        "Interactive Screen",
        MigrationStrategy::DirectNativeMapping,
    )
    .with_required_contract("contract-behavior")
    .with_required_contract("contract-build")
    .with_required_contract("contract-native");

    aggregate.add_task(task);

    let temp_target = unique_temp_dir("target_repair");
    let report = orchestrator
        .run(
            &mut aggregate,
            "fixtures/source",
            temp_target.to_str().unwrap(),
        )
        .expect("Orchestrator run should recover and succeed");

    // Must have triggered repair!
    assert_eq!(
        report.repairs_triggered, 1,
        "Expected exactly 1 repair trigger"
    );
    assert!(
        report.final_target_revision.as_u64() >= 3,
        "Target revision should advance on initial attempt and repair attempt"
    );
    assert!(report.is_successful());
    assert_eq!(
        aggregate
            .get_task(&TaskId::new("task-interactive"))
            .unwrap()
            .state,
        TaskState::Accepted
    );
    assert_eq!(
        aggregate
            .get_task(&TaskId::new("task-interactive"))
            .unwrap()
            .retry_count,
        1
    );
    assert!(temp_checkpoint.exists(), "Checkpoint must have been saved");
}

// ---------------------------------------------------------------------------
// 9. Interrupted Migration Checkpoint Save & Resume Test
// ---------------------------------------------------------------------------

#[test]
fn test_orchestrator_checkpoint_resume_after_interruption() {
    let temp_dir = unique_temp_dir("interruption_test");
    let cp_file = temp_dir.join("checkpoint.json");

    // Phase A: Simulate first run where task-1 is completed and saved
    {
        let mut aggregate = MigrationAggregate::new("mig-interrupt", "fp-123");
        let t1 = MigrationTask::new(
            "task-1",
            "Done Task",
            MigrationStrategy::DirectNativeMapping,
        );
        let t2 = MigrationTask::new(
            "task-2",
            "Pending Task",
            MigrationStrategy::DirectNativeMapping,
        )
        .with_dependency("task-1");
        aggregate.add_task(t1);
        aggregate.add_task(t2);

        // Mark task-1 as accepted in aggregate
        let task1 = aggregate.get_task_mut(&TaskId::new("task-1")).unwrap();
        task1.state = TaskState::Accepted;

        let snapshot = CheckpointSnapshot::from_aggregate(&aggregate);
        CheckpointManager::save_checkpoint(&cp_file, &snapshot).unwrap();
    }

    // Phase B: Resume from checkpoint
    {
        let prompt_provider = Box::new(
            OpfPromptProvider::from_root(workspace_root())
                .expect("Failed to load real prompt catalog"),
        );
        let knowledge_repo = Box::new(OkfKnowledgeRepository::new(workspace_root()));
        let agent_backend = Box::new(DeterministicMockAgentBackend::new());
        let source_adapter = Box::new(MockSourceAdapter);
        let target_adapter = Box::new(MockTargetAdapter);
        let artifact_repo = Box::new(InMemoryArtifactRepository::new());

        let verifiers: Vec<Box<dyn VerifierPluginPort>> = vec![
            Box::new(AlwaysPassVerifier::new(VerificationCategory::Build)),
            Box::new(AlwaysPassVerifier::new(
                VerificationCategory::NativeCompliance,
            )),
            Box::new(AlwaysPassVerifier::new(VerificationCategory::Behavioral)),
        ];

        let mut orchestrator = MigrationOrchestrator::new(
            prompt_provider,
            knowledge_repo,
            agent_backend,
            source_adapter,
            target_adapter,
            artifact_repo,
            verifiers,
        )
        .with_checkpoint_path(&cp_file);

        let mut aggregate = MigrationAggregate::new("mig-interrupt", "fp-123");
        let build_contract = Contract::new(
            "contract-build",
            "Build",
            "Build pass",
            VerificationCategory::Build,
            ContractSeverity::Critical,
        );
        let native_contract = Contract::new(
            "contract-native",
            "Native",
            "Native pass",
            VerificationCategory::NativeCompliance,
            ContractSeverity::Critical,
        );
        let behavior_contract = Contract::new(
            "contract-behavior",
            "Behavior",
            "Behavior pass",
            VerificationCategory::Behavioral,
            ContractSeverity::Critical,
        );
        aggregate.add_contract(build_contract);
        aggregate.add_contract(native_contract);
        aggregate.add_contract(behavior_contract);

        let t1 = MigrationTask::new(
            "task-1",
            "Done Task",
            MigrationStrategy::DirectNativeMapping,
        );
        let t2 = MigrationTask::new(
            "task-2",
            "Pending Task",
            MigrationStrategy::DirectNativeMapping,
        )
        .with_dependency("task-1")
        .with_required_contract("contract-behavior");
        aggregate.add_task(t1);
        aggregate.add_task(t2);

        // Resume from checkpoint!
        let resumed = orchestrator
            .resume_from_checkpoint(&mut aggregate)
            .expect("Resume from checkpoint should succeed");
        assert!(resumed);
        assert_eq!(
            aggregate.get_task(&TaskId::new("task-1")).unwrap().state,
            TaskState::Accepted
        );

        // Run orchestrator: should skip task-1 and complete task-2!
        let temp_target = unique_temp_dir("target_resumed");
        let report = orchestrator
            .run(
                &mut aggregate,
                "fixtures/source",
                temp_target.to_str().unwrap(),
            )
            .expect("Resumed run should succeed");

        assert!(report.is_successful());
        assert_eq!(
            aggregate.get_task(&TaskId::new("task-1")).unwrap().state,
            TaskState::Accepted
        );
        assert_eq!(
            aggregate.get_task(&TaskId::new("task-2")).unwrap().state,
            TaskState::Accepted
        );
    }
}

// ---------------------------------------------------------------------------
// 11. Reproducibility Gate & OS-Level Subprocess Agent Containment Tests
// ---------------------------------------------------------------------------

#[test]
fn test_reproducibility_gate_validates_pinned_commit_and_fixture_hashes() {
    use mae_application::ReproducibilityGate;

    let root = workspace_root();
    let fixture = root.join("fixtures/python-rust-cli/data/employees.json");

    let manifest = ReproducibilityGate::capture(&root, std::slice::from_ref(&fixture))
        .expect("Should capture reproducibility manifest from workspace");

    assert_eq!(manifest.mae_commit_sha.len(), 40);
    assert!(manifest.rustc_version.contains("rustc"));
    assert_eq!(manifest.fixture_hashes.len(), 1);

    // Verification succeeds against the exact captured commit and unmodified fixture
    manifest
        .verify(&root)
        .expect("Verification against matching commit and fixture should pass");

    // Verification fails if expected commit SHA does not match
    let mut wrong_sha_manifest = manifest.clone();
    wrong_sha_manifest.mae_commit_sha = "0000000000000000000000000000000000000000".to_string();
    let err = wrong_sha_manifest.verify(&root).unwrap_err();
    assert!(err.to_string().contains("Reproducibility gate failed"));
}

#[test]
fn test_configured_subprocess_agent_executes_under_containment_and_enforces_network_policy() {
    use mae_application::AgentContainmentPolicy;
    use std::time::Duration;

    let temp_dir = unique_temp_dir("agent_containment_ok");
    let ro_fixture = temp_dir.join("readonly_input.json");
    fs::write(&ro_fixture, r#"{"status":"baseline"}"#).unwrap();

    let script = temp_dir.join("fake_codex.py");
    fs::write(
        &script,
        r#"#!/usr/bin/env python3
import os, sys
assert os.environ.get("MAE_NETWORK_DISABLED") == "1", "Network must be disabled"
assert os.environ.get("CARGO_NET_OFFLINE") == "true", "Cargo offline must be set"
print("agent-executed-ok")
sys.exit(0)
"#,
    )
    .unwrap();

    let policy = AgentContainmentPolicy::new(&temp_dir)
        .with_read_only_fixture(&ro_fixture)
        .with_network_disabled(true)
        .with_timeout(Duration::from_secs(5));

    let backend = CodexAgentBackend::new().with_process_runner(
        "python3",
        vec![script.to_str().unwrap().to_string()],
        policy,
    );

    let (_, provenance) = OpfPromptProvider::from_root(workspace_root())
        .unwrap()
        .render_prompt(
            "migration.code-migrator",
            &[("context_path", "target/tasks/task-1")],
        )
        .unwrap();

    let res = backend
        .execute_prompt("migrate slice", &provenance)
        .expect("Subprocess execution should succeed");
    assert!(res.success);
    assert!(res.log_summary.contains("agent-executed-ok"));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_subprocess_agent_containment_rejects_read_only_fixture_mutation() {
    use mae_application::AgentContainmentPolicy;
    use std::time::Duration;

    let temp_dir = unique_temp_dir("agent_containment_ro");
    let ro_fixture = temp_dir.join("readonly_input.json");
    fs::write(&ro_fixture, r#"{"status":"baseline"}"#).unwrap();

    let script = temp_dir.join("rogue_agent.py");
    fs::write(
        &script,
        format!(
            r#"import os, stat
p = {:?}
try:
    os.chmod(p, stat.S_IWRITE | stat.S_IREAD)
    with open(p, "w") as f:
        f.write("mutated!")
except PermissionError:
    pass
"#,
            ro_fixture.to_str().unwrap()
        ),
    )
    .unwrap();

    let policy = AgentContainmentPolicy::new(&temp_dir)
        .with_read_only_fixture(&ro_fixture)
        .with_timeout(Duration::from_secs(5));

    let backend = ClaudeCodeAgentBackend::new().with_process_runner(
        "python3",
        vec![script.to_str().unwrap().to_string()],
        policy,
    );

    let (_, provenance) = OpfPromptProvider::from_root(workspace_root())
        .unwrap()
        .render_prompt(
            "migration.code-migrator",
            &[("context_path", "target/tasks/task-1")],
        )
        .unwrap();

    let res = backend.execute_prompt("migrate slice", &provenance);
    assert!(
        res.is_err(),
        "Mutating a read-only fixture must fail the containment gate"
    );
    assert!(res
        .unwrap_err()
        .to_string()
        .contains("Containment violation"));

    // Verify original fixture content was restored by containment guard
    let restored = fs::read_to_string(&ro_fixture).unwrap();
    assert_eq!(restored, r#"{"status":"baseline"}"#);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_subprocess_agent_timeout_terminates_descendant_processes() {
    use mae_application::AgentContainmentPolicy;
    use std::time::Duration;

    let temp_dir = unique_temp_dir("agent_containment_timeout");
    let late_mutation_file = temp_dir.join("late_mutation.txt");

    // Parent spawns a child process that sleeps 1.0s then writes late_mutation_file, while parent sleeps 10s
    let script = temp_dir.join("spawn_descendant.py");
    fs::write(
        &script,
        format!(
            r#"import subprocess, sys, time
target = {:?}
child_code = "import sys, time; time.sleep(1.0); open(sys.argv[1], 'w').write('escaped')"
subprocess.Popen([sys.executable, "-c", child_code, target])
time.sleep(10)
"#,
            late_mutation_file.to_str().unwrap()
        ),
    )
    .unwrap();

    let policy = AgentContainmentPolicy::new(&temp_dir).with_timeout(Duration::from_millis(300));

    let backend = CopilotAgentBackend::new().with_process_runner(
        "python3",
        vec![script.to_str().unwrap().to_string()],
        policy,
    );

    let (_, provenance) = OpfPromptProvider::from_root(workspace_root())
        .unwrap()
        .render_prompt(
            "migration.code-migrator",
            &[("context_path", "target/tasks/task-1")],
        )
        .unwrap();

    let res = backend.execute_prompt("migrate slice", &provenance);
    assert!(res.is_err(), "Timed-out agent must return error");
    assert!(res.unwrap_err().to_string().contains("timed out"));

    // Wait past the descendant's 1.2s timer and confirm the descendant was killed with the process group
    std::thread::sleep(Duration::from_millis(1400));
    assert!(
        !late_mutation_file.exists(),
        "Descendant process must be terminated on timeout and unable to mutate files after run"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}
