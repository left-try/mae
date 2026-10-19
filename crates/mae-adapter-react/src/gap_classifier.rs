use std::fs;
use std::path::{Path, PathBuf};

use mae_domain::{
    CapabilityAssessment, CapabilityLevel, CapabilityReport, ContractSeverity, DomainError,
    GapReasonCode, MigrationGap, MigrationStrategy, SemanticUiModel,
};

/// Classifies capability gaps and constructs a complete `CapabilityReport` for the source codebase.
pub fn classify_source_gaps(
    source_path: &str,
    model: &SemanticUiModel,
) -> Result<CapabilityReport, DomainError> {
    let path = PathBuf::from(source_path);
    let mut gaps = Vec::new();

    if path.exists() {
        if path.is_file() {
            let content = fs::read_to_string(&path).map_err(|e| {
                DomainError::PortError(format!("Failed to read {}: {}", path.display(), e))
            })?;
            let file_gaps = scan_file_gaps(&path.to_string_lossy(), &content);
            gaps.extend(file_gaps);
        } else {
            scan_dir_gaps(&path, &path, &mut gaps)?;
        }
    }

    // Also inspect SemanticUiModel for semantic-level gaps
    let model_gaps = scan_model_gaps(model);
    for g in model_gaps {
        if !gaps.iter().any(|existing| {
            existing.source_location == g.source_location && existing.reason_code == g.reason_code
        }) {
            gaps.push(g);
        }
    }

    gaps.sort_by(|a, b| a.source_location.cmp(&b.source_location));

    Ok(build_capability_report(gaps))
}

/// Fallback classifier that assesses capabilities using only the `SemanticUiModel`.
pub fn classify_model_gaps(model: &SemanticUiModel) -> Result<CapabilityReport, DomainError> {
    let gaps = scan_model_gaps(model);
    Ok(build_capability_report(gaps))
}

fn scan_dir_gaps(
    current: &Path,
    root: &Path,
    gaps: &mut Vec<MigrationGap>,
) -> Result<(), DomainError> {
    if !current.is_dir() {
        return Ok(());
    }

    let entries = fs::read_dir(current).map_err(|e| {
        DomainError::PortError(format!("Failed to read dir {}: {}", current.display(), e))
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            DomainError::PortError(format!(
                "Directory entry error in {}: {}",
                current.display(),
                e
            ))
        })?;
        let p = entry.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();

        if p.is_dir() {
            if name == "node_modules"
                || name == ".git"
                || name == "target"
                || name == "dist"
                || name == "build"
                || name == ".next"
                || name == ".turbo"
                || name == ".okf-index"
            {
                continue;
            }
            scan_dir_gaps(&p, root, gaps)?;
        } else {
            let lower = name.to_lowercase();
            if lower.ends_with(".tsx")
                || lower.ends_with(".jsx")
                || lower.ends_with(".ts")
                || lower.ends_with(".js")
                || lower.ends_with(".css")
            {
                if let Ok(content) = fs::read_to_string(&p) {
                    let rel_path = p
                        .strip_prefix(root)
                        .map(|r| r.to_string_lossy().replace('\\', "/"))
                        .unwrap_or_else(|_| p.to_string_lossy().replace('\\', "/"));
                    let file_gaps = scan_file_gaps(&rel_path, &content);
                    gaps.extend(file_gaps);
                }
            }
        }
    }
    Ok(())
}

/// Scans source text line-by-line and extracts specific `MigrationGap`s.
pub fn scan_file_gaps(file_path: &str, content: &str) -> Vec<MigrationGap> {
    let mut gaps = Vec::new();

    for (line_idx, line) in content.lines().enumerate() {
        let line_num = line_idx + 1;
        let loc = format!("{}:{}", file_path, line_num);
        let trimmed = line.trim();

        // 1. Browser-only APIs: window.localStorage, sessionStorage, document.cookie, navigator.geolocation, window.open
        if trimmed.contains("localStorage") {
            let mut gap = MigrationGap::new(
                &loc,
                GapReasonCode::UiBrowserOnlyApi,
                ContractSeverity::Critical,
                MigrationStrategy::ReplacementComponent,
                "Browser-only API `window.localStorage` requires native persistent storage replacement",
            );
            gap = gap.with_human_decision_required(true);
            gaps.push(gap);
        } else if trimmed.contains("sessionStorage") {
            let mut gap = MigrationGap::new(
                &loc,
                GapReasonCode::UiBrowserOnlyApi,
                ContractSeverity::High,
                MigrationStrategy::ReplacementComponent,
                "Browser-only API `sessionStorage` requires native in-memory session replacement",
            );
            gap = gap.with_human_decision_required(false);
            gaps.push(gap);
        } else if trimmed.contains("document.cookie") {
            let mut gap = MigrationGap::new(
                &loc,
                GapReasonCode::UiBrowserOnlyApi,
                ContractSeverity::High,
                MigrationStrategy::ReplacementComponent,
                "Browser-only API `document.cookie` requires native credential storage",
            );
            gap = gap.with_human_decision_required(false);
            gaps.push(gap);
        } else if trimmed.contains("navigator.geolocation") {
            let mut gap = MigrationGap::new(
                &loc,
                GapReasonCode::UiBrowserOnlyApi,
                ContractSeverity::High,
                MigrationStrategy::CustomImplementation,
                "Browser-only API `navigator.geolocation` requires native platform location service",
            );
            gap = gap.with_human_decision_required(false);
            gaps.push(gap);
        } else if trimmed.contains("window.open") {
            let mut gap = MigrationGap::new(
                &loc,
                GapReasonCode::UiBrowserOnlyApi,
                ContractSeverity::Medium,
                MigrationStrategy::CustomImplementation,
                "Browser-only API `window.open` requires native window spawn or OS shell open",
            );
            gap = gap.with_human_decision_required(false);
            gaps.push(gap);
        }

        // 2. Canvas Graphics: <canvas or getContext("2d") / webgl
        if trimmed.contains("<canvas")
            || trimmed.contains("getContext(\"2d\")")
            || trimmed.contains("getContext('2d')")
            || trimmed.contains("getContext(\"webgl\")")
            || trimmed.contains("getContext('webgl')")
        {
            let gap = MigrationGap::new(
                &loc,
                GapReasonCode::UiCanvas,
                ContractSeverity::High,
                MigrationStrategy::CustomImplementation,
                "HTML5 Canvas API is unsupported in native Slint; requires custom native rasterization or Slint Image",
            );
            gaps.push(gap);
        }

        // 3. Complex Animations: @keyframes, animation:
        if trimmed.starts_with("@keyframes") || trimmed.contains("animation:") {
            let gap = MigrationGap::new(
                &loc,
                GapReasonCode::UiComplexAnimation,
                ContractSeverity::Low,
                MigrationStrategy::SemanticReconstruction,
                "CSS @keyframes or complex animation property requires translation to Slint state animations",
            );
            gaps.push(gap);
        }

        // 4. Unsupported CSS: backdrop-filter, mix-blend-mode, position: sticky
        if trimmed.contains("backdrop-filter") {
            let gap = MigrationGap::new(
                &loc,
                GapReasonCode::UiUnsupportedCss,
                ContractSeverity::High,
                MigrationStrategy::SemanticReconstruction,
                "Unsupported CSS property `backdrop-filter` in native desktop renderer",
            );
            gaps.push(gap);
        } else if trimmed.contains("mix-blend-mode") {
            let gap = MigrationGap::new(
                &loc,
                GapReasonCode::UiUnsupportedCss,
                ContractSeverity::Low,
                MigrationStrategy::SemanticReconstruction,
                "Unsupported CSS property `mix-blend-mode` in native desktop renderer",
            );
            gaps.push(gap);
        } else if trimmed.contains("position: sticky") || trimmed.contains("position:sticky") {
            let gap = MigrationGap::new(
                &loc,
                GapReasonCode::UiUnsupportedCss,
                ContractSeverity::High,
                MigrationStrategy::SemanticReconstruction,
                "Unsupported CSS property `position: sticky` requires custom native scroll pinning",
            );
            gaps.push(gap);
        }

        // 5. Dynamic inline styles: style={{ ... }}
        if trimmed.contains("style={{") {
            let gap = MigrationGap::new(
                &loc,
                GapReasonCode::UiDynamicStyle,
                ContractSeverity::Low,
                MigrationStrategy::SemanticReconstruction,
                "Dynamic inline `style={{ ... }}` computed expression must be translated to declarative Slint properties",
            );
            gaps.push(gap);
        }

        // 6. Third-party DOM widgets: react-select, monaco-editor, ag-grid
        if trimmed.contains("react-select") {
            let gap = MigrationGap::new(
                &loc,
                GapReasonCode::UiThirdPartyWidget,
                ContractSeverity::High,
                MigrationStrategy::ReplacementComponent,
                "Third-party widget `react-select` requires native Slint ComboBox/Dropdown replacement",
            );
            gaps.push(gap);
        } else if trimmed.contains("monaco-editor") {
            let gap = MigrationGap::new(
                &loc,
                GapReasonCode::UiThirdPartyWidget,
                ContractSeverity::High,
                MigrationStrategy::ReplacementComponent,
                "Third-party widget `monaco-editor` requires native text editor component replacement",
            );
            gaps.push(gap);
        } else if trimmed.contains("ag-grid") {
            let gap = MigrationGap::new(
                &loc,
                GapReasonCode::UiThirdPartyWidget,
                ContractSeverity::High,
                MigrationStrategy::ReplacementComponent,
                "Third-party widget `ag-grid` requires native Slint TableView replacement",
            );
            gaps.push(gap);
        }

        // 7. Unresolved external state libraries: redux, mobx
        if (trimmed.starts_with("import ") || trimmed.starts_with("from "))
            && (trimmed.contains("\"redux\"")
                || trimmed.contains("'redux'")
                || trimmed.contains("\"@reduxjs/toolkit\"")
                || trimmed.contains("'@reduxjs/toolkit'")
                || trimmed.contains("\"mobx\"")
                || trimmed.contains("'mobx'"))
        {
            let gap = MigrationGap::new(
                &loc,
                GapReasonCode::UiUnresolvedStateLibrary,
                ContractSeverity::High,
                MigrationStrategy::SemanticReconstruction,
                "External state library dependency requires migration to pure Rust application reducer architecture",
            );
            gaps.push(gap);
        }

        // 8. Accessibility gaps: interactive elements missing accessible name
        if (trimmed.contains("<button") || trimmed.contains("<input"))
            && !trimmed.contains("aria-label")
            && !trimmed.contains("aria-labelledby")
            && (trimmed.ends_with("/>")
                || trimmed.contains("onClick")
                || trimmed.contains("onChange"))
            && !trimmed.contains("label")
        {
            let gap = MigrationGap::new(
                &loc,
                GapReasonCode::UiAccessibilityGap,
                ContractSeverity::Low,
                MigrationStrategy::DirectNativeMapping,
                "Interactive UI element lacks `aria-label` or accessible name attribute",
            );
            gaps.push(gap);
        }
    }

    gaps
}

fn scan_model_gaps(model: &SemanticUiModel) -> Vec<MigrationGap> {
    let mut gaps = Vec::new();

    // Check components for third-party widgets or missing names
    for comp in &model.components {
        let lower = comp.name.to_lowercase();
        if lower.contains("select") && !lower.contains("native") {
            gaps.push(MigrationGap::new(
                &comp.source_path,
                GapReasonCode::UiThirdPartyWidget,
                ContractSeverity::Medium,
                MigrationStrategy::ReplacementComponent,
                format!(
                    "Component `{}` may require native desktop dropdown adaptation",
                    comp.name
                ),
            ));
        }
    }

    // Check style tokens for complex animations
    for token in &model.style_tokens {
        if token.value.contains("cubic-bezier") || token.name.contains("animation") {
            gaps.push(MigrationGap::new(
                format!("token:{}", token.name),
                GapReasonCode::UiComplexAnimation,
                ContractSeverity::Low,
                MigrationStrategy::SemanticReconstruction,
                format!(
                    "Style token `{}` defines complex timing or animation",
                    token.name
                ),
            ));
        }
    }

    gaps
}

fn build_capability_report(gaps: Vec<MigrationGap>) -> CapabilityReport {
    let mut report = CapabilityReport::new();

    let has_browser_api = gaps
        .iter()
        .any(|g| g.reason_code == GapReasonCode::UiBrowserOnlyApi);
    let has_canvas = gaps
        .iter()
        .any(|g| g.reason_code == GapReasonCode::UiCanvas);
    let has_unsupported_css = gaps
        .iter()
        .any(|g| g.reason_code == GapReasonCode::UiUnsupportedCss);
    let has_complex_anim = gaps
        .iter()
        .any(|g| g.reason_code == GapReasonCode::UiComplexAnimation);
    let has_dynamic_style = gaps
        .iter()
        .any(|g| g.reason_code == GapReasonCode::UiDynamicStyle);
    let has_third_party = gaps
        .iter()
        .any(|g| g.reason_code == GapReasonCode::UiThirdPartyWidget);
    let has_unresolved_state = gaps
        .iter()
        .any(|g| g.reason_code == GapReasonCode::UiUnresolvedStateLibrary);
    let has_a11y_gap = gaps
        .iter()
        .any(|g| g.reason_code == GapReasonCode::UiAccessibilityGap);

    // 1. Browser APIs Assessment
    let browser_level = if has_browser_api {
        CapabilityLevel::Partial
    } else {
        CapabilityLevel::Supported
    };
    let browser_strat = if has_browser_api {
        MigrationStrategy::ReplacementComponent
    } else {
        MigrationStrategy::DirectNativeMapping
    };
    report.add_assessment(
        CapabilityAssessment::new("Browser APIs & Windowing", browser_level, browser_strat)
            .with_notes("Evaluates window, localStorage, and navigator dependencies"),
    );

    // 2. Canvas & Rendering Assessment
    let canvas_level = if has_canvas {
        CapabilityLevel::Unsupported
    } else {
        CapabilityLevel::Supported
    };
    let canvas_strat = if has_canvas {
        MigrationStrategy::CustomImplementation
    } else {
        MigrationStrategy::DirectNativeMapping
    };
    report.add_assessment(
        CapabilityAssessment::new("Graphics & Canvas Rendering", canvas_level, canvas_strat)
            .with_notes("Evaluates 2D canvas, WebGL, and custom rasterization"),
    );

    // 3. Styling & Layout Assessment
    let style_level = if has_unsupported_css || has_complex_anim || has_dynamic_style {
        CapabilityLevel::Partial
    } else {
        CapabilityLevel::Supported
    };
    report.add_assessment(
        CapabilityAssessment::new(
            "Styling & Responsive Layout",
            style_level,
            MigrationStrategy::SemanticReconstruction,
        )
        .with_notes(
            "Evaluates CSS variables, flex/grid constraints, backdrop filters, and animations",
        ),
    );

    // 4. Third-Party Widgets Assessment
    let widget_level = if has_third_party {
        CapabilityLevel::Partial
    } else {
        CapabilityLevel::Supported
    };
    report.add_assessment(
        CapabilityAssessment::new(
            "Third-Party UI Widgets",
            widget_level,
            MigrationStrategy::ReplacementComponent,
        )
        .with_notes("Evaluates external DOM components like Monaco, AgGrid, and React-Select"),
    );

    // 5. State Management Assessment
    let state_level = if has_unresolved_state {
        CapabilityLevel::Partial
    } else {
        CapabilityLevel::Supported
    };
    report.add_assessment(
        CapabilityAssessment::new(
            "Application State Architecture",
            state_level,
            MigrationStrategy::SemanticReconstruction,
        )
        .with_notes(
            "Evaluates local hooks (useState/useReducer) and external libraries (Redux/MobX)",
        ),
    );

    // 6. Accessibility Assessment
    let a11y_level = if has_a11y_gap {
        CapabilityLevel::Partial
    } else {
        CapabilityLevel::Supported
    };
    report.add_assessment(
        CapabilityAssessment::new(
            "Accessibility & Assistive Technology",
            a11y_level,
            MigrationStrategy::DirectNativeMapping,
        )
        .with_notes("Evaluates accessible names, roles, and keyboard navigation"),
    );

    // Attach all identified gaps
    for gap in gaps {
        report.add_gap(gap);
    }

    report
}
