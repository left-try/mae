use mae_domain::{
    ArtifactId, ArtifactKind, ArtifactProvenance, ArtifactRevision, DomainError, LayoutConstraint,
    ProvenanceOrigin, RouteDefinition, SemanticUiModel, VersionedArtifact,
};

use crate::json::JsonValue;

/// Screenshot capture observation recorded by Playwright.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenshotObservation {
    pub name: String,
    pub file_path: String,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub route_path: String,
}

/// Dynamic route transition observed during Playwright test execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteTransitionObservation {
    pub from_route: String,
    pub to_route: String,
    pub trigger_event: String,
    pub duration_ms: u64,
}

/// DOM accessibility tree observation captured at runtime.
#[derive(Debug, Clone, PartialEq)]
pub struct DomAccessibilityObservation {
    pub component_id: String,
    pub role: String,
    pub accessible_name: String,
    pub bounding_box: (f64, f64, f64, f64),
}

/// Baseline of runtime traces, screenshots, and DOM observations from Playwright.
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeTraceBaseline {
    pub manifest_id: String,
    pub source_revision: ArtifactRevision,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub screenshots: Vec<ScreenshotObservation>,
    pub route_transitions: Vec<RouteTransitionObservation>,
    pub dom_observations: Vec<DomAccessibilityObservation>,
}

impl RuntimeTraceBaseline {
    pub fn new(manifest_id: impl Into<String>, source_revision: ArtifactRevision) -> Self {
        Self {
            manifest_id: manifest_id.into(),
            source_revision,
            viewport_width: 1280,
            viewport_height: 720,
            screenshots: Vec::new(),
            route_transitions: Vec::new(),
            dom_observations: Vec::new(),
        }
    }

    pub fn with_viewport(mut self, width: u32, height: u32) -> Self {
        self.viewport_width = width;
        self.viewport_height = height;
        self
    }

    pub fn add_screenshot(&mut self, screenshot: ScreenshotObservation) {
        self.screenshots.push(screenshot);
    }

    pub fn add_route_transition(&mut self, transition: RouteTransitionObservation) {
        self.route_transitions.push(transition);
    }

    pub fn add_dom_observation(&mut self, obs: DomAccessibilityObservation) {
        self.dom_observations.push(obs);
    }

    /// Parses a Playwright baseline manifest JSON string.
    pub fn parse_manifest(json_str: &str) -> Result<Self, DomainError> {
        let val = JsonValue::parse(json_str)?;

        let manifest_id = val
            .get_string("manifest_id")
            .unwrap_or_else(|| "playwright-baseline-manifest".to_string());
        let source_rev_num = val.get_u64("source_revision").unwrap_or(1);
        let viewport_w = val.get_u64("viewport_width").unwrap_or(1280) as u32;
        let viewport_h = val.get_u64("viewport_height").unwrap_or(720) as u32;

        let mut baseline = Self::new(manifest_id, ArtifactRevision::new(source_rev_num))
            .with_viewport(viewport_w, viewport_h);

        // Parse screenshots
        if let Some(shots_arr) = val.get_array("screenshots") {
            for item in shots_arr {
                if let (Some(name), Some(file_path)) =
                    (item.get_string("name"), item.get_string("file_path"))
                {
                    let vw = item.get_u64("viewport_width").unwrap_or(viewport_w as u64) as u32;
                    let vh = item.get_u64("viewport_height").unwrap_or(viewport_h as u64) as u32;
                    let route = item
                        .get_string("route_path")
                        .unwrap_or_else(|| "/".to_string());

                    baseline.add_screenshot(ScreenshotObservation {
                        name,
                        file_path,
                        viewport_width: vw,
                        viewport_height: vh,
                        route_path: route,
                    });
                }
            }
        }

        // Parse route transitions
        if let Some(trans_arr) = val.get_array("route_transitions") {
            for item in trans_arr {
                if let (Some(from), Some(to)) =
                    (item.get_string("from_route"), item.get_string("to_route"))
                {
                    let trigger = item
                        .get_string("trigger_event")
                        .unwrap_or_else(|| "navigate".to_string());
                    let duration = item.get_u64("duration_ms").unwrap_or(0);

                    baseline.add_route_transition(RouteTransitionObservation {
                        from_route: from,
                        to_route: to,
                        trigger_event: trigger,
                        duration_ms: duration,
                    });
                }
            }
        }

        // Parse DOM observations
        if let Some(dom_arr) = val.get_array("dom_observations") {
            for item in dom_arr {
                if let (Some(cid), Some(role), Some(acc_name)) = (
                    item.get_string("component_id"),
                    item.get_string("role"),
                    item.get_string("accessible_name"),
                ) {
                    let mut bbox = (0.0, 0.0, 0.0, 0.0);
                    if let Some(b_arr) = item.get_array("bounding_box") {
                        if b_arr.len() == 4 {
                            let x = b_arr[0].as_f64().unwrap_or(0.0);
                            let y = b_arr[1].as_f64().unwrap_or(0.0);
                            let w = b_arr[2].as_f64().unwrap_or(0.0);
                            let h = b_arr[3].as_f64().unwrap_or(0.0);
                            bbox = (x, y, w, h);
                        }
                    }

                    baseline.add_dom_observation(DomAccessibilityObservation {
                        component_id: cid,
                        role,
                        accessible_name: acc_name,
                        bounding_box: bbox,
                    });
                }
            }
        }

        Ok(baseline)
    }

    /// Serializes runtime traces to deterministic JSON.
    pub fn to_trace_set_json(&self) -> String {
        let mut transitions_json = Vec::new();
        for t in &self.route_transitions {
            transitions_json.push(JsonValue::Object(vec![
                (
                    "from_route".to_string(),
                    JsonValue::String(t.from_route.clone()),
                ),
                (
                    "to_route".to_string(),
                    JsonValue::String(t.to_route.clone()),
                ),
                (
                    "trigger_event".to_string(),
                    JsonValue::String(t.trigger_event.clone()),
                ),
                (
                    "duration_ms".to_string(),
                    JsonValue::Number(t.duration_ms as f64),
                ),
            ]));
        }

        let mut dom_json = Vec::new();
        for d in &self.dom_observations {
            dom_json.push(JsonValue::Object(vec![
                (
                    "component_id".to_string(),
                    JsonValue::String(d.component_id.clone()),
                ),
                ("role".to_string(), JsonValue::String(d.role.clone())),
                (
                    "accessible_name".to_string(),
                    JsonValue::String(d.accessible_name.clone()),
                ),
                (
                    "bounding_box".to_string(),
                    JsonValue::Array(vec![
                        JsonValue::Number(d.bounding_box.0),
                        JsonValue::Number(d.bounding_box.1),
                        JsonValue::Number(d.bounding_box.2),
                        JsonValue::Number(d.bounding_box.3),
                    ]),
                ),
            ]));
        }

        let root = JsonValue::Object(vec![
            (
                "manifest_id".to_string(),
                JsonValue::String(self.manifest_id.clone()),
            ),
            (
                "viewport_width".to_string(),
                JsonValue::Number(self.viewport_width as f64),
            ),
            (
                "viewport_height".to_string(),
                JsonValue::Number(self.viewport_height as f64),
            ),
            (
                "route_transitions".to_string(),
                JsonValue::Array(transitions_json),
            ),
            ("dom_observations".to_string(), JsonValue::Array(dom_json)),
        ]);

        root.to_pretty_json(0)
    }

    /// Serializes screenshot baseline observations to deterministic JSON.
    pub fn to_screenshot_baseline_json(&self) -> String {
        let mut shots_json = Vec::new();
        for s in &self.screenshots {
            shots_json.push(JsonValue::Object(vec![
                ("name".to_string(), JsonValue::String(s.name.clone())),
                (
                    "file_path".to_string(),
                    JsonValue::String(s.file_path.clone()),
                ),
                (
                    "viewport_width".to_string(),
                    JsonValue::Number(s.viewport_width as f64),
                ),
                (
                    "viewport_height".to_string(),
                    JsonValue::Number(s.viewport_height as f64),
                ),
                (
                    "route_path".to_string(),
                    JsonValue::String(s.route_path.clone()),
                ),
            ]));
        }

        let root = JsonValue::Object(vec![
            (
                "manifest_id".to_string(),
                JsonValue::String(self.manifest_id.clone()),
            ),
            ("screenshots".to_string(), JsonValue::Array(shots_json)),
        ]);

        root.to_pretty_json(0)
    }

    /// Emits `ArtifactKind::RuntimeTraceSet` tagged with `ProvenanceOrigin::RuntimeObservation`.
    pub fn to_trace_set_artifact(&self) -> VersionedArtifact {
        let content = self.to_trace_set_json();
        let artifact_id = ArtifactId::new(format!("trace-set:{}", self.manifest_id));
        let provenance = ArtifactProvenance::new(
            ProvenanceOrigin::RuntimeObservation,
            "playwright-trace-collector",
            "1970-01-01T00:00:00Z",
            self.source_revision,
        );
        VersionedArtifact::from_string(
            artifact_id,
            ArtifactKind::RuntimeTraceSet,
            self.source_revision,
            provenance,
            content,
        )
    }

    /// Emits `ArtifactKind::ScreenshotBaseline` tagged with `ProvenanceOrigin::RuntimeObservation`.
    pub fn to_screenshot_baseline_artifact(&self) -> VersionedArtifact {
        let content = self.to_screenshot_baseline_json();
        let artifact_id = ArtifactId::new(format!("screenshot-baseline:{}", self.manifest_id));
        let provenance = ArtifactProvenance::new(
            ProvenanceOrigin::RuntimeObservation,
            "playwright-screenshot-collector",
            "1970-01-01T00:00:00Z",
            self.source_revision,
        );
        VersionedArtifact::from_string(
            artifact_id,
            ArtifactKind::ScreenshotBaseline,
            self.source_revision,
            provenance,
            content,
        )
    }
}

/// Merges runtime observations into `SemanticUiModel` while preserving static truth provenance.
pub fn merge_runtime_observations(
    model: &mut SemanticUiModel,
    baseline: &RuntimeTraceBaseline,
) -> Result<Vec<VersionedArtifact>, DomainError> {
    // 1. Merge observed route transitions into model routes if missing
    for transition in &baseline.route_transitions {
        if !model.routes.iter().any(|r| r.path == transition.to_route) {
            let view_name = sanitize_view_name(&transition.to_route);
            model.add_route(RouteDefinition::new(&transition.to_route, view_name));
        }
    }

    // 2. Add layout constraint for observed runtime viewport without overwriting static constraints
    let viewport_constraint = LayoutConstraint::new(
        "runtime-viewport",
        baseline.viewport_width as f64,
        baseline.viewport_height as f64,
    );
    if !model
        .layout_constraints
        .iter()
        .any(|c| c.direction == "runtime-viewport")
    {
        model.add_layout_constraint(viewport_constraint);
    }

    // 3. Emit versioned artifacts with ProvenanceOrigin::RuntimeObservation
    let trace_artifact = baseline.to_trace_set_artifact();
    let screenshot_artifact = baseline.to_screenshot_baseline_artifact();

    Ok(vec![trace_artifact, screenshot_artifact])
}

fn sanitize_view_name(route: &str) -> String {
    let clean = route.trim_start_matches('/');
    if clean.is_empty() {
        "HomeView".to_string()
    } else {
        let mut s = String::new();
        for part in clean.split('/') {
            let mut chars = part.chars();
            if let Some(c) = chars.next() {
                s.push(c.to_ascii_uppercase());
                s.extend(chars);
            }
        }
        format!("{}View", s)
    }
}
