use std::fs;
use std::path::Path;

use mae_domain::{DomainError, MigrationEvalMetrics, SemanticUiModel};

/// Evaluates naming and architectural consistency between a React `SemanticUiModel`
/// and a generated native Rust + Slint target directory, populating `MigrationEvalMetrics`.
pub struct ConsistencyAndArchitectureEvaluator;

impl ConsistencyAndArchitectureEvaluator {
    pub fn evaluate_target_consistency(
        model: &SemanticUiModel,
        target_dir: &Path,
        metrics: &mut MigrationEvalMetrics,
    ) -> Result<(), DomainError> {
        let app_slint_path = target_dir.join("ui/app.slint");
        let tokens_slint_path = target_dir.join("ui/theme/tokens.slint");
        let state_rs_path = target_dir.join("src/app/state.rs");
        let reducer_rs_path = target_dir.join("src/app/reducer.rs");
        let events_rs_path = target_dir.join("src/app/events.rs");
        let effects_rs_path = target_dir.join("src/app/effects.rs");
        let nav_rs_path = target_dir.join("src/app/navigation.rs");

        let app_slint = fs::read_to_string(&app_slint_path).unwrap_or_default();
        let state_rs = fs::read_to_string(&state_rs_path).unwrap_or_default();
        let nav_rs = fs::read_to_string(&nav_rs_path).unwrap_or_default();
        let reducer_rs = fs::read_to_string(&reducer_rs_path).unwrap_or_default();

        // Collect all component .slint file contents as well
        let mut all_slint_content = app_slint.clone();
        let mut comp_slint_content = String::new();
        let comp_dir = target_dir.join("ui/components");
        if comp_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&comp_dir) {
                for entry in entries.flatten() {
                    if let Ok(txt) = fs::read_to_string(entry.path()) {
                        all_slint_content.push('\n');
                        all_slint_content.push_str(&txt);
                        comp_slint_content.push('\n');
                        comp_slint_content.push_str(&txt);
                    }
                }
            }
        }

        // 1. Component Name Consistency
        metrics.matched_components.clear();
        metrics.missing_components.clear();
        for comp in &model.components {
            if comp.name.eq_ignore_ascii_case("App") {
                if app_slint.contains("export component MainWindow")
                    || all_slint_content.contains("export component App")
                {
                    metrics.matched_components.push(comp.name.clone());
                } else {
                    metrics.missing_components.push(comp.name.clone());
                }
            } else {
                let lower_ident = sanitize_identifier(&comp.name).to_lowercase();
                let snake = to_snake_case(&comp.name);
                let comp_file_lower = comp_dir.join(format!("{}.slint", lower_ident));
                let comp_file_snake = comp_dir.join(format!("{}.slint", snake));
                if comp_file_lower.exists()
                    || comp_file_snake.exists()
                    || all_slint_content.contains(&format!("export component {}", comp.name))
                {
                    metrics.matched_components.push(comp.name.clone());
                } else {
                    metrics.missing_components.push(comp.name.clone());
                }
            }
        }
        metrics.component_name_consistency_ratio = if model.components.is_empty() {
            1.0
        } else {
            metrics.matched_components.len() as f64 / model.components.len() as f64
        };

        // 2. Route Consistency
        metrics.matched_routes.clear();
        metrics.missing_routes.clear();
        for route in &model.routes {
            if nav_rs.contains(&format!("\"{}\"", route.path))
                || app_slint.contains(&format!("\"{}\"", route.path))
            {
                metrics.matched_routes.push(route.path.clone());
            } else {
                metrics.missing_routes.push(route.path.clone());
            }
        }
        metrics.route_consistency_ratio = if model.routes.is_empty() {
            1.0
        } else {
            metrics.matched_routes.len() as f64 / model.routes.len() as f64
        };

        // 3. State Field Consistency
        metrics.matched_state_fields.clear();
        metrics.missing_state_fields.clear();
        for sf in &model.state_fields {
            let lower_ident = sanitize_identifier(&sf.name).to_lowercase();
            let snake = to_snake_case(&sf.name);
            let kebab = sanitize_identifier(&sf.name).replace('_', "-");
            if state_rs.contains(&format!("pub {}:", lower_ident))
                || state_rs.contains(&format!("pub {}:", snake))
                || app_slint.contains(&format!("state-{}", kebab))
            {
                metrics.matched_state_fields.push(sf.name.clone());
            } else {
                metrics.missing_state_fields.push(sf.name.clone());
            }
        }
        metrics.state_field_consistency_ratio = if model.state_fields.is_empty() {
            1.0
        } else {
            metrics.matched_state_fields.len() as f64 / model.state_fields.len() as f64
        };

        // 4. UI Element Parity
        metrics.source_ui_elements_count = model.total_ui_elements();
        let slint_for_elements = if comp_slint_content.is_empty() {
            &all_slint_content
        } else {
            &comp_slint_content
        };
        metrics.target_ui_elements_count = count_slint_ui_elements(slint_for_elements);
        metrics.ui_element_parity_ratio = if metrics.source_ui_elements_count == 0 {
            1.0
        } else {
            (metrics
                .target_ui_elements_count
                .min(metrics.source_ui_elements_count) as f64)
                / (metrics.source_ui_elements_count as f64)
        };

        // 5. Architectural Consistency (Clean Architecture & Logic-UI Separation checks)
        let checks = [
            app_slint_path.exists() && app_slint.contains("export component MainWindow"),
            tokens_slint_path.exists(),
            state_rs_path.exists() && state_rs.contains("pub struct AppState"),
            reducer_rs_path.exists() && reducer_rs.contains("pub fn reduce("),
            events_rs_path.exists(),
            effects_rs_path.exists(),
            nav_rs_path.exists()
                && (nav_rs.contains("pub const ROUTES") || nav_rs.contains("can_navigate")),
            !app_slint.contains("std::") && !app_slint.contains("use crate::"),
        ];
        let passed_checks = checks.iter().filter(|&&ok| ok).count();
        metrics.architecture_consistency_score = passed_checks as f64 / checks.len() as f64;

        Ok(())
    }
}

fn count_slint_ui_elements(all_slint: &str) -> usize {
    all_slint
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("accessible-label:") || trimmed.starts_with("// ui-element:")
        })
        .count()
}

fn sanitize_identifier(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if (ch == '_' || ch == '-') && !out.ends_with('_') {
            out.push('_');
        }
    }
    let trimmed = out.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "field".to_string()
    } else {
        trimmed
    }
}

fn to_snake_case(input: &str) -> String {
    let mut out = String::new();
    for (idx, ch) in input.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if idx > 0 {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}
