use std::fmt;

use crate::ports::DomainError;

/// Target deployment platform for migrated desktop application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TargetPlatform {
    WindowsDesktop,
    UbuntuDesktop,
    LinuxDesktop,
    CrossPlatformDesktop,
}

impl TargetPlatform {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WindowsDesktop => "WINDOWS_DESKTOP",
            Self::UbuntuDesktop => "UBUNTU_DESKTOP",
            Self::LinuxDesktop => "LINUX_DESKTOP",
            Self::CrossPlatformDesktop => "CROSS_PLATFORM_DESKTOP",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "windows" | "win" | "windows-desktop" | "windows_desktop" => Ok(Self::WindowsDesktop),
            "ubuntu" | "ubuntu-desktop" | "ubuntu_desktop" | "debian" => Ok(Self::UbuntuDesktop),
            "linux" | "linux-desktop" | "linux_desktop" => Ok(Self::LinuxDesktop),
            "cross-platform" | "cross_platform" | "all" | "multiplatform" => {
                Ok(Self::CrossPlatformDesktop)
            }
            other => Err(DomainError::PortError(format!(
                "Unsupported target platform '{other}'. Expected windows, ubuntu, linux, or cross-platform"
            ))),
        }
    }

    pub fn requires_windows_subsystem(&self) -> bool {
        matches!(self, Self::WindowsDesktop | Self::CrossPlatformDesktop)
    }

    pub fn requires_linux_desktop_entry(&self) -> bool {
        matches!(
            self,
            Self::UbuntuDesktop | Self::LinuxDesktop | Self::CrossPlatformDesktop
        )
    }

    pub fn native_window_backend(&self) -> &'static str {
        match self {
            Self::WindowsDesktop => "win32-winit",
            Self::UbuntuDesktop | Self::LinuxDesktop => "wayland-x11-winit",
            Self::CrossPlatformDesktop => "winit-multiplatform",
        }
    }
}

impl fmt::Display for TargetPlatform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Prop or parameter definition for a UI component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropDefinition {
    pub name: String,
    pub prop_type: String,
    pub required: bool,
    pub default_value: Option<String>,
}

impl PropDefinition {
    pub fn new(name: impl Into<String>, prop_type: impl Into<String>, required: bool) -> Self {
        Self {
            name: name.into(),
            prop_type: prop_type.into(),
            required,
            default_value: None,
        }
    }

    pub fn with_default(mut self, default_value: impl Into<String>) -> Self {
        self.default_value = Some(default_value.into());
        self
    }
}

/// Component tree node extracted from source UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentNode {
    pub id: String,
    pub name: String,
    pub props: Vec<PropDefinition>,
    pub children: Vec<String>,
    pub source_path: String,
}

impl ComponentNode {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        source_path: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            props: Vec::new(),
            children: Vec::new(),
            source_path: source_path.into(),
        }
    }

    pub fn with_prop(mut self, prop: PropDefinition) -> Self {
        self.props.push(prop);
        self
    }

    pub fn with_child(mut self, child_id: impl Into<String>) -> Self {
        self.children.push(child_id.into());
        self
    }
}

/// Reactive state variable or reducer state field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateField {
    pub name: String,
    pub data_type: String,
    pub initial_value: String,
    pub hook_kind: String,
}

impl StateField {
    pub fn new(
        name: impl Into<String>,
        data_type: impl Into<String>,
        initial_value: impl Into<String>,
        hook_kind: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            data_type: data_type.into(),
            initial_value: initial_value.into(),
            hook_kind: hook_kind.into(),
        }
    }
}

/// Side-effect binding (e.g. data fetching, timers, lifecycle subscriptions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectBinding {
    pub id: String,
    pub dependencies: Vec<String>,
    pub kind: String,
    pub is_async: bool,
}

impl EffectBinding {
    pub fn new(id: impl Into<String>, kind: impl Into<String>, is_async: bool) -> Self {
        Self {
            id: id.into(),
            dependencies: Vec::new(),
            kind: kind.into(),
            is_async,
        }
    }

    pub fn with_dependency(mut self, dep: impl Into<String>) -> Self {
        self.dependencies.push(dep.into());
        self
    }
}

/// User event interaction binding connected to state transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionBinding {
    pub component_id: String,
    pub event_name: String,
    pub handler_name: String,
    pub state_transitions: Vec<String>,
}

impl InteractionBinding {
    pub fn new(
        component_id: impl Into<String>,
        event_name: impl Into<String>,
        handler_name: impl Into<String>,
    ) -> Self {
        Self {
            component_id: component_id.into(),
            event_name: event_name.into(),
            handler_name: handler_name.into(),
            state_transitions: Vec::new(),
        }
    }

    pub fn with_transition(mut self, transition: impl Into<String>) -> Self {
        self.state_transitions.push(transition.into());
        self
    }
}

/// Route and view navigation definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteDefinition {
    pub path: String,
    pub view_component: String,
    pub params: Vec<String>,
}

impl RouteDefinition {
    pub fn new(path: impl Into<String>, view_component: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            view_component: view_component.into(),
            params: Vec::new(),
        }
    }

    pub fn with_param(mut self, param: impl Into<String>) -> Self {
        self.params.push(param.into());
        self
    }
}

/// Layout constraints and spacing rules.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutConstraint {
    pub direction: String,
    pub spacing_px: f64,
    pub padding_px: f64,
    pub responsive_rules: Vec<String>,
}

impl LayoutConstraint {
    pub fn new(direction: impl Into<String>, spacing_px: f64, padding_px: f64) -> Self {
        Self {
            direction: direction.into(),
            spacing_px,
            padding_px,
            responsive_rules: Vec::new(),
        }
    }

    pub fn with_responsive_rule(mut self, rule: impl Into<String>) -> Self {
        self.responsive_rules.push(rule.into());
        self
    }
}

/// Design system style token (color, typography, border, shadow).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleToken {
    pub name: String,
    pub value: String,
}

impl StyleToken {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

/// Image, font, or binary media asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetEntry {
    pub id: String,
    pub path: String,
    pub kind: String,
}

impl AssetEntry {
    pub fn new(id: impl Into<String>, path: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            path: path.into(),
            kind: kind.into(),
        }
    }
}

/// Composable Frontend Domain Intermediate Representation.
///
/// Models component hierarchy, state, effects, interactions, routing,
/// layout, tokens, and assets without inventing an unverified universal AST.
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticUiModel {
    pub platform: TargetPlatform,
    pub components: Vec<ComponentNode>,
    pub state_fields: Vec<StateField>,
    pub effects: Vec<EffectBinding>,
    pub interactions: Vec<InteractionBinding>,
    pub routes: Vec<RouteDefinition>,
    pub layout_constraints: Vec<LayoutConstraint>,
    pub style_tokens: Vec<StyleToken>,
    pub assets: Vec<AssetEntry>,
}

impl SemanticUiModel {
    pub fn new(platform: TargetPlatform) -> Self {
        Self {
            platform,
            components: Vec::new(),
            state_fields: Vec::new(),
            effects: Vec::new(),
            interactions: Vec::new(),
            routes: Vec::new(),
            layout_constraints: Vec::new(),
            style_tokens: Vec::new(),
            assets: Vec::new(),
        }
    }

    pub fn add_component(&mut self, component: ComponentNode) {
        self.components.push(component);
    }

    pub fn add_state_field(&mut self, field: StateField) {
        self.state_fields.push(field);
    }

    pub fn add_effect(&mut self, effect: EffectBinding) {
        self.effects.push(effect);
    }

    pub fn add_interaction(&mut self, interaction: InteractionBinding) {
        self.interactions.push(interaction);
    }

    pub fn add_route(&mut self, route: RouteDefinition) {
        self.routes.push(route);
    }

    pub fn add_layout_constraint(&mut self, constraint: LayoutConstraint) {
        self.layout_constraints.push(constraint);
    }

    pub fn add_style_token(&mut self, token: StyleToken) {
        self.style_tokens.push(token);
    }

    pub fn add_asset(&mut self, asset: AssetEntry) {
        self.assets.push(asset);
    }
}
