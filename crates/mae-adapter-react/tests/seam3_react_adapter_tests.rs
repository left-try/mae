use std::fs;
use std::path::PathBuf;

use mae_adapter_react::{ReactSourceAdapter, RuntimeTraceBaseline};
use mae_domain::{
    ArtifactKind, ArtifactRevision, ContractSeverity, GapReasonCode, MigrationStrategy,
    ProvenanceOrigin, SemanticUiModel, SourceAdapterPort, TargetPlatform,
};

/// Helper to create an isolated temporary directory for testing.
struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(test_name: &str) -> Self {
        let temp =
            std::env::temp_dir().join(format!("mae_test_{}_{}", test_name, std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).expect("Failed to create test temp dir");
        Self { path: temp }
    }

    fn write_file(&self, rel_path: &str, content: &str) {
        let full = self.path.join(rel_path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent dir");
        }
        fs::write(full, content).expect("Failed to write file");
    }

    fn as_str(&self) -> &str {
        self.path.to_str().expect("Invalid UTF-8 path")
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

// ---------------------------------------------------------------------------
// Test 1: Project Inventory Discovery
// ---------------------------------------------------------------------------
#[test]
fn test_discover_project_inventory() {
    let td = TestDir::new("discovery");
    td.write_file(
        "package.json",
        r#"{
            "name": "sample-react-app",
            "version": "1.2.3",
            "dependencies": {
                "react": "^18.2.0",
                "react-dom": "^18.2.0"
            },
            "devDependencies": {
                "typescript": "^5.0.0"
            }
        }"#,
    );
    td.write_file(
        "tsconfig.json",
        r#"{"compilerOptions": {"jsx": "react-jsx"}}"#,
    );
    td.write_file(
        "src/main.tsx",
        r#"import React from 'react';
           import { App } from './App';
           export function main() {}"#,
    );
    td.write_file(
        "src/App.tsx",
        r#"import React from 'react';
           import { Dashboard } from './components/Dashboard';
           export function App() { return <Dashboard />; }"#,
    );
    td.write_file(
        "src/components/Dashboard.tsx",
        r#"import React from 'react';
           export function Dashboard() { return <div>Dashboard</div>; }"#,
    );
    td.write_file("src/styles/app.css", "body { margin: 0; }");
    td.write_file("assets/logo.svg", "<svg></svg>");

    let adapter = ReactSourceAdapter::default_windows();
    let inv = adapter
        .discover_project(td.as_str())
        .expect("Discovery failed");

    assert_eq!(inv.project_name, "sample-react-app");
    assert_eq!(inv.version, "1.2.3");
    assert!(inv.has_tsconfig);
    assert_eq!(
        inv.dependencies.get("react").map(|s| s.as_str()),
        Some("^18.2.0")
    );
    assert_eq!(
        inv.dev_dependencies.get("typescript").map(|s| s.as_str()),
        Some("^5.0.0")
    );
    assert!(inv.entry_files.contains(&"src/main.tsx".to_string()));
    assert!(inv.entry_files.contains(&"src/App.tsx".to_string()));
    assert_eq!(inv.tsx_files.len(), 3);
    assert_eq!(inv.style_files.len(), 1);
    assert_eq!(inv.asset_files.len(), 1);
}

// ---------------------------------------------------------------------------
// Test 2: Source Inventory Artifact Determinism & Integrity
// ---------------------------------------------------------------------------
#[test]
fn test_discover_source_inventory_artifact_determinism() {
    let td = TestDir::new("artifact_determinism");
    td.write_file(
        "package.json",
        r#"{"name": "deterministic-app", "version": "0.1.0"}"#,
    );
    td.write_file("src/main.tsx", "export function Main() {}");

    let adapter = ReactSourceAdapter::default_windows();

    let artifact1 = adapter
        .discover(td.as_str())
        .expect("First discovery failed");
    let artifact2 = adapter
        .discover(td.as_str())
        .expect("Second discovery failed");

    assert_eq!(artifact1.kind, ArtifactKind::SourceInventory);
    assert_eq!(artifact1.content_hash, artifact2.content_hash);
    assert!(artifact1.verify_integrity());
    assert_eq!(
        artifact1.provenance.origin,
        ProvenanceOrigin::DeterministicStatic
    );
    assert_eq!(artifact1.revision, ArtifactRevision::new(1));
}

// ---------------------------------------------------------------------------
// Test 3: Module Dependency Graph Extraction
// ---------------------------------------------------------------------------
#[test]
fn test_module_dependency_graph_extraction() {
    let td = TestDir::new("module_graph");
    td.write_file(
        "src/App.tsx",
        r#"import React from 'react';
           import { Dashboard } from './components/Dashboard';
           import './styles/app.css';
           export function App() {}"#,
    );
    td.write_file(
        "src/components/Dashboard.tsx",
        r#"import React from 'react';
           import { TaskCard } from './TaskCard';
           export function Dashboard() {}"#,
    );
    td.write_file(
        "src/components/TaskCard.tsx",
        r#"export function TaskCard() {}"#,
    );

    let adapter = ReactSourceAdapter::default_windows();
    let artifact = adapter
        .extract_module_dependency_graph(td.as_str())
        .expect("Graph extraction failed");

    assert_eq!(artifact.kind, ArtifactKind::ModuleDependencyGraph);
    let json_text = artifact.as_str().unwrap();
    assert!(json_text.contains("./components/Dashboard"));
    assert!(json_text.contains("./TaskCard"));
    assert!(json_text.contains("./styles/app.css"));
}

// ---------------------------------------------------------------------------
// Test 4: TSX Component Nodes and Hierarchy Extraction
// ---------------------------------------------------------------------------
#[test]
fn test_extract_tsx_component_nodes_and_hierarchy() {
    let td = TestDir::new("components");
    td.write_file(
        "src/Dashboard.tsx",
        r#"import React from 'react';
           export function Dashboard() {
               return (
                   <div>
                       <Header title="Overview" />
                       <TaskGrid>
                           <TaskCard id="1" />
                       </TaskGrid>
                       <Footer />
                   </div>
               );
           }
           export const Footer: React.FC = () => <footer>Footer</footer>;"#,
    );

    let adapter = ReactSourceAdapter::default_windows();
    let model = adapter
        .extract_semantics(td.as_str())
        .expect("Semantic extraction failed");

    assert_eq!(model.platform, TargetPlatform::WindowsDesktop);
    assert_eq!(model.components.len(), 2);

    let dashboard = model
        .components
        .iter()
        .find(|c| c.name == "Dashboard")
        .unwrap();
    assert!(dashboard.children.contains(&"Header".to_string()));
    assert!(dashboard.children.contains(&"TaskGrid".to_string()));
    assert!(dashboard.children.contains(&"TaskCard".to_string()));
    assert!(dashboard.children.contains(&"Footer".to_string()));

    let footer = model
        .components
        .iter()
        .find(|c| c.name == "Footer")
        .unwrap();
    assert_eq!(footer.name, "Footer");
}

// ---------------------------------------------------------------------------
// Test 5: Props Interface Fields Extraction
// ---------------------------------------------------------------------------
#[test]
fn test_extract_props_interface_fields() {
    let td = TestDir::new("props");
    td.write_file(
        "src/TaskCard.tsx",
        r#"import React from 'react';
           export interface TaskCardProps {
               id: string;
               title: string;
               completed?: boolean;
               priority?: string = "medium";
               onToggle?: (id: string) => void;
           }
           export function TaskCard(props: TaskCardProps) {
               return <div>{props.title}</div>;
           }"#,
    );

    let adapter = ReactSourceAdapter::default_windows();
    let model = adapter
        .extract_semantics(td.as_str())
        .expect("Semantic extraction failed");

    let card = model
        .components
        .iter()
        .find(|c| c.name == "TaskCard")
        .unwrap();
    assert_eq!(card.props.len(), 5);

    let id_prop = card.props.iter().find(|p| p.name == "id").unwrap();
    assert_eq!(id_prop.prop_type, "string");
    assert!(id_prop.required);

    let completed_prop = card.props.iter().find(|p| p.name == "completed").unwrap();
    assert_eq!(completed_prop.prop_type, "boolean");
    assert!(!completed_prop.required);

    let priority_prop = card.props.iter().find(|p| p.name == "priority").unwrap();
    assert_eq!(priority_prop.default_value.as_deref(), Some("\"medium\""));
}

// ---------------------------------------------------------------------------
// Test 6: `useState` State Fields Extraction
// ---------------------------------------------------------------------------
#[test]
fn test_extract_use_state_fields() {
    let td = TestDir::new("use_state");
    td.write_file(
        "src/Counter.tsx",
        r#"import React, { useState } from 'react';
           export function Counter() {
               const [count, setCount] = useState<number>(0);
               const [label, setLabel] = useState("Default");
               const [isActive, setIsActive] = useState<boolean>(true);
               return <div>{label}: {count}</div>;
           }"#,
    );

    let adapter = ReactSourceAdapter::default_windows();
    let model = adapter
        .extract_semantics(td.as_str())
        .expect("Semantic extraction failed");

    let count_field = model
        .state_fields
        .iter()
        .find(|s| s.name == "count")
        .unwrap();
    assert_eq!(count_field.data_type, "number");
    assert_eq!(count_field.initial_value, "0");
    assert_eq!(count_field.hook_kind, "useState");

    let label_field = model
        .state_fields
        .iter()
        .find(|s| s.name == "label")
        .unwrap();
    assert_eq!(label_field.data_type, "string");
    assert_eq!(label_field.initial_value, "\"Default\"");

    let active_field = model
        .state_fields
        .iter()
        .find(|s| s.name == "isActive")
        .unwrap();
    assert_eq!(active_field.data_type, "boolean");
    assert_eq!(active_field.initial_value, "true");
}

// ---------------------------------------------------------------------------
// Test 7: `useReducer` State Fields Extraction
// ---------------------------------------------------------------------------
#[test]
fn test_extract_use_reducer_fields() {
    let td = TestDir::new("use_reducer");
    td.write_file(
        "src/TaskManager.tsx",
        r#"import React, { useReducer } from 'react';
           export function TaskManager() {
               const [state, dispatch] = useReducer(taskReducer, { tasks: [], count: 0 });
               return <div>Tasks: {state.count}</div>;
           }"#,
    );

    let adapter = ReactSourceAdapter::default_windows();
    let model = adapter
        .extract_semantics(td.as_str())
        .expect("Semantic extraction failed");

    let state_field = model
        .state_fields
        .iter()
        .find(|s| s.name == "state")
        .unwrap();
    assert_eq!(state_field.hook_kind, "useReducer");
    assert_eq!(state_field.initial_value, "{ tasks: [], count: 0 }");
}

// ---------------------------------------------------------------------------
// Test 8: `useEffect` Side-Effect Bindings Extraction
// ---------------------------------------------------------------------------
#[test]
fn test_extract_use_effect_bindings() {
    let td = TestDir::new("use_effect");
    td.write_file(
        "src/DataLoader.tsx",
        r#"import React, { useEffect } from 'react';
           export function DataLoader() {
               useEffect(() => {
                   async function fetchItems() {
                       const res = await fetch("/api/tasks");
                       const data = await res.json();
                   }
                   fetchItems();
               }, [filter, sortOrder]);

               useEffect(() => {
                   const timer = setInterval(() => {}, 1000);
                   return () => clearInterval(timer);
               }, []);
               return <div>Loader</div>;
           }"#,
    );

    let adapter = ReactSourceAdapter::default_windows();
    let model = adapter
        .extract_semantics(td.as_str())
        .expect("Semantic extraction failed");

    assert_eq!(model.effects.len(), 2);

    let fetch_effect = model.effects.iter().find(|e| e.is_async).unwrap();
    assert_eq!(fetch_effect.kind, "data_fetch");
    assert!(fetch_effect.dependencies.contains(&"filter".to_string()));
    assert!(fetch_effect.dependencies.contains(&"sortOrder".to_string()));

    let timer_effect = model.effects.iter().find(|e| !e.is_async).unwrap();
    assert_eq!(timer_effect.kind, "timer");
    assert!(timer_effect.dependencies.is_empty());
}

// ---------------------------------------------------------------------------
// Test 9: JSX Interaction Event Handlers Extraction
// ---------------------------------------------------------------------------
#[test]
fn test_extract_interaction_bindings() {
    let td = TestDir::new("interactions");
    td.write_file(
        "src/InteractiveForm.tsx",
        r#"import React from 'react';
           export function InteractiveForm() {
               return (
                   <form onSubmit={handleFormSubmit}>
                       <input onChange={handleFilterChange} />
                       <button onClick={() => setCount(c => c + 1)}>Increment</button>
                   </form>
               );
           }"#,
    );

    let adapter = ReactSourceAdapter::default_windows();
    let model = adapter
        .extract_semantics(td.as_str())
        .expect("Semantic extraction failed");

    assert!(model
        .interactions
        .iter()
        .any(|i| i.event_name == "onSubmit"));
    assert!(model
        .interactions
        .iter()
        .any(|i| i.event_name == "onChange"));

    let click_interaction = model
        .interactions
        .iter()
        .find(|i| i.event_name == "onClick")
        .unwrap();
    assert!(click_interaction
        .state_transitions
        .contains(&"setCount".to_string()));
}

// ---------------------------------------------------------------------------
// Test 10: Route Definitions Extraction
// ---------------------------------------------------------------------------
#[test]
fn test_extract_route_definitions() {
    let td = TestDir::new("routes");
    td.write_file(
        "src/App.tsx",
        r#"import React from 'react';
           import { Routes, Route } from 'react-router-dom';
           export function App() {
               return (
                   <Routes>
                       <Route path="/dashboard" element={<Dashboard />} />
                       <Route path="/tasks/:taskId" element={<TaskDetail />} />
                       <Route path="/settings" component={Settings} />
                   </Routes>
               );
           }"#,
    );

    let adapter = ReactSourceAdapter::default_windows();
    let model = adapter
        .extract_semantics(td.as_str())
        .expect("Semantic extraction failed");

    assert_eq!(model.routes.len(), 3);

    let dashboard_route = model
        .routes
        .iter()
        .find(|r| r.path == "/dashboard")
        .unwrap();
    assert_eq!(dashboard_route.view_component, "Dashboard");
    assert!(dashboard_route.params.is_empty());

    let task_route = model
        .routes
        .iter()
        .find(|r| r.path == "/tasks/:taskId")
        .unwrap();
    assert_eq!(task_route.view_component, "TaskDetail");
    assert_eq!(task_route.params, vec!["taskId"]);

    let settings_route = model.routes.iter().find(|r| r.path == "/settings").unwrap();
    assert_eq!(settings_route.view_component, "Settings");
}

// ---------------------------------------------------------------------------
// Test 11: CSS Tokens and Layout Constraints Extraction
// ---------------------------------------------------------------------------
#[test]
fn test_extract_css_tokens_and_layout_constraints() {
    let td = TestDir::new("css_tokens");
    td.write_file(
        "src/theme.css",
        r#":root {
               --primary-color: #0078d4;
               --background-color: #f3f3f3;
               --border-radius: 4px;
           }
           .main-container {
               display: flex;
               flex-direction: column;
               gap: 16px;
               padding: 24px;
           }
           @media (max-width: 768px) {
               .main-container {
                   flex-direction: column;
               }
           }"#,
    );

    let adapter = ReactSourceAdapter::default_windows();
    let model = adapter
        .extract_semantics(td.as_str())
        .expect("Semantic extraction failed");

    assert_eq!(model.style_tokens.len(), 3);
    let primary_token = model
        .style_tokens
        .iter()
        .find(|t| t.name == "--primary-color")
        .unwrap();
    assert_eq!(primary_token.value, "#0078d4");

    let layout = model.layout_constraints.first().unwrap();
    assert_eq!(layout.direction, "column");
    assert_eq!(layout.spacing_px, 16.0);
    assert_eq!(layout.padding_px, 24.0);
    assert_eq!(layout.border_radius_px, 4.0);
    assert_eq!(layout.responsive_rules.len(), 1);
    assert!(layout.responsive_rules[0].contains("@media (max-width: 768px)"));
}

#[test]
fn test_extract_css_roundness_font_size_and_scale_factor() {
    let td = TestDir::new("css_roundness_scale");
    td.write_file(
        "src/theme.css",
        r#":root {
               --color-primary: #2563eb;
               --radius-md: 8px;
               --font-size-base: 14px;
               --scale-factor: 1.25;
           }
           .app-shell {
               display: flex;
               flex-direction: column;
               gap: 12px;
               padding: 16px;
               border-radius: 8px;
               font-size: 14px;
               zoom: 1.25;
           }"#,
    );

    let adapter = ReactSourceAdapter::default_windows();
    let model = adapter
        .extract_semantics(td.as_str())
        .expect("Semantic extraction failed");

    let layout = model.layout_constraints.first().unwrap();
    assert_eq!(layout.spacing_px, 12.0);
    assert_eq!(layout.padding_px, 16.0);
    assert_eq!(layout.border_radius_px, 8.0);
    assert_eq!(layout.font_size_px, 14.0);
    assert!((layout.scale_factor - 1.25).abs() < 1e-6);
}

// ---------------------------------------------------------------------------
// Test 12: Browser-Only API Capability Gap Classification
// ---------------------------------------------------------------------------
#[test]
fn test_classify_browser_only_api_gaps() {
    let td = TestDir::new("gap_browser");
    td.write_file(
        "src/storage.ts",
        r#"export function saveToken(t: string) {
               window.localStorage.setItem("auth_token", t);
               sessionStorage.setItem("session_key", "val");
               document.cookie = "user=admin";
               navigator.geolocation.getCurrentPosition(() => {});
               window.open("https://example.com");
           }"#,
    );

    let adapter = ReactSourceAdapter::default_windows();
    let model = SemanticUiModel::new(TargetPlatform::WindowsDesktop);
    let report = adapter
        .classify_source_gaps(td.as_str(), &model)
        .expect("Gap classification failed");

    let browser_gaps: Vec<_> = report
        .gaps
        .iter()
        .filter(|g| g.reason_code == GapReasonCode::UiBrowserOnlyApi)
        .collect();

    assert_eq!(browser_gaps.len(), 5);

    let storage_gap = browser_gaps
        .iter()
        .find(|g| g.description.contains("localStorage"))
        .unwrap();
    assert_eq!(storage_gap.severity, ContractSeverity::Critical);
    assert_eq!(
        storage_gap.strategy,
        MigrationStrategy::ReplacementComponent
    );
    assert!(storage_gap.human_decision_required);

    assert!(report.has_critical_unresolved_gaps());
}

// ---------------------------------------------------------------------------
// Test 13: Canvas and Complex Animation Capability Gap Classification
// ---------------------------------------------------------------------------
#[test]
fn test_classify_canvas_and_complex_animation_gaps() {
    let td = TestDir::new("gap_canvas_anim");
    td.write_file(
        "src/Chart.tsx",
        r#"export function Chart() {
               const ctx = canvas.getContext("2d");
               return <canvas id="mainChart" />;
           }"#,
    );
    td.write_file(
        "src/animation.css",
        r#"@keyframes pulse {
               0% { transform: scale(1); }
               50% { transform: scale(1.1); }
               100% { transform: scale(1); }
           }"#,
    );

    let adapter = ReactSourceAdapter::default_windows();
    let model = SemanticUiModel::new(TargetPlatform::WindowsDesktop);
    let report = adapter
        .classify_source_gaps(td.as_str(), &model)
        .expect("Gap classification failed");

    let canvas_gap = report
        .gaps
        .iter()
        .find(|g| g.reason_code == GapReasonCode::UiCanvas)
        .unwrap();
    assert_eq!(canvas_gap.severity, ContractSeverity::High);
    assert_eq!(canvas_gap.strategy, MigrationStrategy::CustomImplementation);

    let anim_gap = report
        .gaps
        .iter()
        .find(|g| g.reason_code == GapReasonCode::UiComplexAnimation)
        .unwrap();
    assert_eq!(anim_gap.severity, ContractSeverity::Low);
    assert_eq!(anim_gap.strategy, MigrationStrategy::SemanticReconstruction);
}

// ---------------------------------------------------------------------------
// Test 14: Unsupported CSS and Dynamic Style Gap Classification
// ---------------------------------------------------------------------------
#[test]
fn test_classify_unsupported_css_and_dynamic_styles() {
    let td = TestDir::new("gap_css");
    td.write_file(
        "src/glass.css",
        r#".glass-panel {
               backdrop-filter: blur(10px);
               mix-blend-mode: multiply;
               position: sticky;
           }"#,
    );
    td.write_file(
        "src/Widget.tsx",
        r#"export function Widget({ dynamicWidth }: { dynamicWidth: number }) {
               return <div style={{ width: dynamicWidth, color: "red" }}>Widget</div>;
           }"#,
    );

    let adapter = ReactSourceAdapter::default_windows();
    let model = SemanticUiModel::new(TargetPlatform::WindowsDesktop);
    let report = adapter
        .classify_source_gaps(td.as_str(), &model)
        .expect("Gap classification failed");

    let css_gaps: Vec<_> = report
        .gaps
        .iter()
        .filter(|g| g.reason_code == GapReasonCode::UiUnsupportedCss)
        .collect();
    assert_eq!(css_gaps.len(), 3);

    let dynamic_gap = report
        .gaps
        .iter()
        .find(|g| g.reason_code == GapReasonCode::UiDynamicStyle)
        .unwrap();
    assert_eq!(dynamic_gap.severity, ContractSeverity::Low);
    assert_eq!(
        dynamic_gap.strategy,
        MigrationStrategy::SemanticReconstruction
    );
}

// ---------------------------------------------------------------------------
// Test 15: Third-Party DOM Widgets and External State Libraries Classification
// ---------------------------------------------------------------------------
#[test]
fn test_classify_third_party_widgets_and_state_libraries() {
    let td = TestDir::new("gap_widgets");
    td.write_file(
        "src/ComplexViews.tsx",
        r#"import Select from 'react-select';
           import Editor from 'monaco-editor';
           import { AgGridReact } from 'ag-grid';
           import { createSlice } from '@reduxjs/toolkit';
           export function ComplexViews() {}"#,
    );

    let adapter = ReactSourceAdapter::default_windows();
    let model = SemanticUiModel::new(TargetPlatform::WindowsDesktop);
    let report = adapter
        .classify_source_gaps(td.as_str(), &model)
        .expect("Gap classification failed");

    let widget_gaps: Vec<_> = report
        .gaps
        .iter()
        .filter(|g| g.reason_code == GapReasonCode::UiThirdPartyWidget)
        .collect();
    assert_eq!(widget_gaps.len(), 3);

    let state_gap = report
        .gaps
        .iter()
        .find(|g| g.reason_code == GapReasonCode::UiUnresolvedStateLibrary)
        .unwrap();
    assert_eq!(state_gap.severity, ContractSeverity::High);
    assert_eq!(
        state_gap.strategy,
        MigrationStrategy::SemanticReconstruction
    );
}

// ---------------------------------------------------------------------------
// Test 16: Accessibility Gaps Classification
// ---------------------------------------------------------------------------
#[test]
fn test_classify_accessibility_gaps() {
    let td = TestDir::new("gap_a11y");
    td.write_file(
        "src/Toolbar.tsx",
        r#"export function Toolbar({ onAction }: { onAction: () => void }) {
               return (
                   <div>
                       <button onClick={onAction} />
                       <input onChange={() => {}} />
                   </div>
               );
           }"#,
    );

    let adapter = ReactSourceAdapter::default_windows();
    let model = SemanticUiModel::new(TargetPlatform::WindowsDesktop);
    let report = adapter
        .classify_source_gaps(td.as_str(), &model)
        .expect("Gap classification failed");

    let a11y_gaps: Vec<_> = report
        .gaps
        .iter()
        .filter(|g| g.reason_code == GapReasonCode::UiAccessibilityGap)
        .collect();
    assert_eq!(a11y_gaps.len(), 2);
}

// ---------------------------------------------------------------------------
// Test 17: Playwright Runtime Trace Baseline Parsing & Merging
// ---------------------------------------------------------------------------
#[test]
fn test_runtime_trace_baseline_parsing_and_merging() {
    let manifest_json = r#"{
        "manifest_id": "playwright-run-42",
        "source_revision": 3,
        "viewport_width": 1920,
        "viewport_height": 1080,
        "screenshots": [
            {
                "name": "dashboard_full",
                "file_path": "screenshots/dashboard.png",
                "viewport_width": 1920,
                "viewport_height": 1080,
                "route_path": "/dashboard"
            }
        ],
        "route_transitions": [
            {
                "from_route": "/dashboard",
                "to_route": "/settings",
                "trigger_event": "click:settings-icon",
                "duration_ms": 120
            }
        ],
        "dom_observations": [
            {
                "component_id": "Dashboard",
                "role": "main",
                "accessible_name": "Main Dashboard",
                "bounding_box": [0.0, 0.0, 1920.0, 1080.0]
            }
        ]
    }"#;

    let baseline = RuntimeTraceBaseline::parse_manifest(manifest_json).expect("Parse failed");
    assert_eq!(baseline.manifest_id, "playwright-run-42");
    assert_eq!(baseline.source_revision, ArtifactRevision::new(3));
    assert_eq!(baseline.screenshots.len(), 1);
    assert_eq!(baseline.route_transitions.len(), 1);
    assert_eq!(baseline.dom_observations.len(), 1);

    let mut model = SemanticUiModel::new(TargetPlatform::WindowsDesktop);
    let adapter = ReactSourceAdapter::default_windows();
    let artifacts = adapter
        .merge_runtime_observations(&mut model, &baseline)
        .expect("Merging failed");

    assert_eq!(artifacts.len(), 2);

    let trace_artifact = artifacts
        .iter()
        .find(|a| a.kind == ArtifactKind::RuntimeTraceSet)
        .unwrap();
    assert_eq!(
        trace_artifact.provenance.origin,
        ProvenanceOrigin::RuntimeObservation
    );
    assert_eq!(trace_artifact.revision, ArtifactRevision::new(3));
    assert!(trace_artifact.verify_integrity());

    let screenshot_artifact = artifacts
        .iter()
        .find(|a| a.kind == ArtifactKind::ScreenshotBaseline)
        .unwrap();
    assert_eq!(
        screenshot_artifact.provenance.origin,
        ProvenanceOrigin::RuntimeObservation
    );
    assert!(screenshot_artifact.verify_integrity());

    // Invariant: runtime route was added without replacing static truths
    assert!(model.routes.iter().any(|r| r.path == "/settings"));
    assert!(model
        .layout_constraints
        .iter()
        .any(|c| c.direction == "runtime-viewport"));
}

// ---------------------------------------------------------------------------
// Test 18: End-to-End SourceAdapterPort Lifecycle
// ---------------------------------------------------------------------------
#[test]
fn test_source_adapter_port_end_to_end() {
    let td = TestDir::new("port_e2e");
    td.write_file(
        "package.json",
        r#"{"name": "complete-app", "version": "2.0.0"}"#,
    );
    td.write_file(
        "src/main.tsx",
        r#"import React, { useState } from 'react';
           export function Main() {
               const [count, setCount] = useState<number>(0);
               return <button onClick={() => setCount(count + 1)}>Count: {count}</button>;
           }"#,
    );
    td.write_file("src/theme.css", ":root { --brand: #107c41; }");

    let adapter: Box<dyn SourceAdapterPort> = Box::new(ReactSourceAdapter::default_windows());

    // 1. Discover
    let inventory_artifact = adapter.discover(td.as_str()).expect("Discover failed");
    assert_eq!(inventory_artifact.kind, ArtifactKind::SourceInventory);

    // 2. Extract Semantics
    let semantic_model = adapter
        .extract_semantics(td.as_str())
        .expect("Extract semantics failed");
    assert_eq!(semantic_model.platform, TargetPlatform::WindowsDesktop);
    assert_eq!(semantic_model.components.len(), 1);
    assert_eq!(semantic_model.state_fields.len(), 1);
    assert_eq!(semantic_model.style_tokens.len(), 1);

    // 3. Assess Capabilities
    let capability_report = adapter
        .assess_capabilities(&semantic_model)
        .expect("Assess capabilities failed");
    assert!(!capability_report.assessments.is_empty());
}

#[test]
fn test_extract_api_endpoint_bindings() {
    let td = TestDir::new("api_endpoints");
    td.write_file(
        "src/App.tsx",
        r#"import React, { useEffect } from 'react';
           export function App() {
               useEffect(() => {
                   fetch('http://127.0.0.1:4010/api/state');
                   fetch('http://127.0.0.1:4010/api/tasks', { method: 'POST' });
               }, []);
               return <div>App</div>;
           }"#,
    );

    let adapter = ReactSourceAdapter::default_windows();
    let model = adapter
        .extract_semantics(td.as_str())
        .expect("Extract semantics failed");

    assert_eq!(model.api_endpoints.len(), 2);
    assert_eq!(model.api_endpoints[0].method, "GET");
    assert_eq!(
        model.api_endpoints[0].url_or_path,
        "http://127.0.0.1:4010/api/state"
    );
    assert_eq!(model.api_endpoints[1].method, "POST");
    assert_eq!(
        model.api_endpoints[1].url_or_path,
        "http://127.0.0.1:4010/api/tasks"
    );
}

#[test]
fn test_extract_jsx_ui_elements_with_options_and_variants() {
    let td = TestDir::new("ui_elements_options");
    td.write_file(
        "src/SettingsView.tsx",
        r#"import React from 'react';
           export function SettingsView() {
               return (
                   <div>
                       <button role="tab" aria-label="Range 7 Days Tab" onClick={() => setTimeRange('7d')}>
                           Last 7 Days
                       </button>
                       <select aria-label="Theme Mode Select" onChange={(e) => setThemeMode(e.target.value)}>
                           <option value="dark-slate">Dark Slate</option>
                           <option value="light-fluent">Light Fluent</option>
                           <option value="high-contrast">High Contrast</option>
                       </select>
                       <button aria-label="Complete All Visible Button" onClick={() => setAllCompleted(true)}>
                           Complete All Visible
                       </button>
                       <input type="checkbox" aria-label="Compact Mode Checkbox" checked={true} />
                   </div>
               );
           }"#,
    );

    let adapter = ReactSourceAdapter::default_windows();
    let model = adapter
        .extract_semantics(td.as_str())
        .expect("Extract semantics failed");

    let comp = model
        .components
        .iter()
        .find(|c| c.name == "SettingsView")
        .expect("SettingsView component must exist");

    assert_eq!(comp.elements.len(), 4);

    // Tab button with inner text and variant
    let tab = &comp.elements[0];
    assert_eq!(tab.label, "Last 7 Days");
    assert_eq!(tab.variant.as_deref(), Some("tab-selector"));

    // Select with options
    let select = &comp.elements[1];
    assert_eq!(select.options.len(), 3);
    assert!(select.options.contains(&"dark-slate".to_string()));
    assert!(select.options.contains(&"light-fluent".to_string()));
    assert!(select.options.contains(&"high-contrast".to_string()));
    assert_eq!(select.variant.as_deref(), Some("theme-selector"));

    // Complete all button
    let complete_all = &comp.elements[2];
    assert_eq!(complete_all.label, "Complete All Visible");
    assert_eq!(complete_all.variant.as_deref(), Some("complete-all"));

    // Checkbox
    let checkbox = &comp.elements[3];
    assert_eq!(checkbox.kind, mae_domain::UiElementKind::Checkbox);
}

#[test]
fn test_extract_css_responsive_wrap_and_max_width() {
    let td = TestDir::new("css_responsive");
    td.write_file(
        "src/theme.css",
        r#".app-shell {
               display: flex;
               flex-direction: column;
               flex-wrap: wrap;
               max-width: 1280px;
               gap: 12px;
               padding: 16px;
           }
           @media (max-width: 768px) {
               .app-shell { padding: 8px; }
           }"#,
    );

    let adapter = ReactSourceAdapter::default_windows();
    let model = adapter
        .extract_semantics(td.as_str())
        .expect("Extract semantics failed");

    let layout = model
        .layout_constraints
        .first()
        .expect("Layout constraint must exist");
    assert!(layout.flex_wrap);
    assert_eq!(layout.max_width_px, Some(1280.0));
    assert!(!layout.responsive_rules.is_empty());
}
