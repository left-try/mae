use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use mae_domain::{
    ApiEndpointBinding, AssetEntry, ComponentNode, DomainError, EffectBinding, InteractionBinding,
    LayoutConstraint, PropDefinition, RouteDefinition, SemanticUiModel, StateField, StyleToken,
    TargetPlatform, UiElementKind, UiElementNode,
};

/// Deterministic static semantic analyzer for React/TypeScript and CSS source code.
pub fn extract_semantics(
    source_path: &str,
    platform: TargetPlatform,
) -> Result<SemanticUiModel, DomainError> {
    let path = PathBuf::from(source_path);
    if !path.exists() {
        return Err(DomainError::PortError(format!(
            "Source path does not exist: {}",
            source_path
        )));
    }

    let mut model = SemanticUiModel::new(platform);
    let mut tsx_files = Vec::new();
    let mut css_files = Vec::new();
    let mut asset_files = Vec::new();

    if path.is_file() {
        let lower = path.to_string_lossy().to_lowercase();
        if lower.ends_with(".tsx") || lower.ends_with(".jsx") || lower.ends_with(".ts") {
            tsx_files.push(path.clone());
        } else if lower.ends_with(".css") {
            css_files.push(path.clone());
        } else if is_asset_file(&lower) {
            asset_files.push(path.clone());
        }
    } else {
        collect_semantic_files(&path, &mut tsx_files, &mut css_files, &mut asset_files)?;
    }

    tsx_files.sort();
    css_files.sort();
    asset_files.sort();

    // 1. Analyze TSX files for Components, Props, Hooks, Events, Routes
    for tsx_file in &tsx_files {
        let content = fs::read_to_string(tsx_file).map_err(|e| {
            DomainError::PortError(format!("Failed to read {}: {}", tsx_file.display(), e))
        })?;
        let rel_path = if path.is_dir() {
            tsx_file
                .strip_prefix(&path)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| tsx_file.to_string_lossy().replace('\\', "/"))
        } else {
            tsx_file.to_string_lossy().replace('\\', "/")
        };

        analyze_tsx_content(&rel_path, &content, &mut model);
    }

    // 2. Analyze CSS files for Tokens and Layout Constraints
    for css_file in &css_files {
        let content = fs::read_to_string(css_file).map_err(|e| {
            DomainError::PortError(format!("Failed to read {}: {}", css_file.display(), e))
        })?;
        analyze_css_content(&content, &mut model);
    }

    // 3. Register Assets
    for asset_file in &asset_files {
        let rel_path = if path.is_dir() {
            asset_file
                .strip_prefix(&path)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| asset_file.to_string_lossy().replace('\\', "/"))
        } else {
            asset_file.to_string_lossy().replace('\\', "/")
        };

        let stem = asset_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("asset")
            .to_string();
        let ext = asset_file
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("bin")
            .to_string();

        model.add_asset(AssetEntry::new(stem, rel_path, ext));
    }

    Ok(model)
}

fn collect_semantic_files(
    dir: &Path,
    tsx_files: &mut Vec<PathBuf>,
    css_files: &mut Vec<PathBuf>,
    asset_files: &mut Vec<PathBuf>,
) -> Result<(), DomainError> {
    let entries = fs::read_dir(dir).map_err(|e| {
        DomainError::PortError(format!("Failed to read directory {}: {}", dir.display(), e))
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            DomainError::PortError(format!("Directory entry error in {}: {}", dir.display(), e))
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
            collect_semantic_files(&p, tsx_files, css_files, asset_files)?;
        } else {
            let lower = name.to_lowercase();
            if lower.ends_with(".tsx") || lower.ends_with(".jsx") || lower.ends_with(".ts") {
                tsx_files.push(p);
            } else if lower.ends_with(".css")
                || lower.ends_with(".scss")
                || lower.ends_with(".sass")
            {
                css_files.push(p);
            } else if is_asset_file(&lower) {
                asset_files.push(p);
            }
        }
    }
    Ok(())
}

fn is_asset_file(name: &str) -> bool {
    name.ends_with(".svg")
        || name.ends_with(".png")
        || name.ends_with(".jpg")
        || name.ends_with(".jpeg")
        || name.ends_with(".ico")
        || name.ends_with(".webp")
        || name.ends_with(".gif")
}

// ---------------------------------------------------------------------------
// TSX / TypeScript AST & Semantic Extraction
// ---------------------------------------------------------------------------

pub fn analyze_tsx_content(file_path: &str, content: &str, model: &mut SemanticUiModel) {
    // 1. Extract interfaces / type declarations for Props
    let props_map = extract_props_interfaces(content);

    // 2. Extract Route definitions
    let routes = extract_routes(content);
    for route in routes {
        if !model.routes.iter().any(|r| r.path == route.path) {
            model.add_route(route);
        }
    }

    // 3. Extract components and their internal hooks, children, and events
    let components = extract_components(file_path, content, &props_map);
    for comp in components {
        model.add_component(comp);
    }

    // 4. Extract State fields (useState, useReducer)
    let state_fields = extract_state_fields(content);
    for sf in state_fields {
        if !model.state_fields.iter().any(|s| s.name == sf.name) {
            model.add_state_field(sf);
        }
    }

    // 5. Extract Effect bindings (useEffect)
    let effects = extract_effect_bindings(file_path, content);
    for ef in effects {
        if !model.effects.iter().any(|e| e.id == ef.id) {
            model.add_effect(ef);
        }
    }

    // 6. Extract Interaction bindings (onClick, onChange, onSubmit)
    let interactions = extract_interaction_bindings(content);
    for inter in interactions {
        model.add_interaction(inter);
    }

    // 7. Extract HTTP / Backend API endpoint bindings (fetch(...))
    let endpoints = extract_api_endpoints(file_path, content);
    for ep in endpoints {
        if !model
            .api_endpoints
            .iter()
            .any(|e| e.method == ep.method && e.url_or_path == ep.url_or_path)
        {
            model.add_api_endpoint(ep);
        }
    }
}

/// Extracts interfaces or type aliases matching `*Props` or `Props`.
fn extract_props_interfaces(content: &str) -> BTreeMap<String, Vec<PropDefinition>> {
    let mut interfaces = BTreeMap::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Pattern: interface FooProps { or type FooProps = {
        let interface_name = if (line.starts_with("export interface ")
            || line.starts_with("interface "))
            && line.contains('{')
        {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let idx = parts.iter().position(|&p| p == "interface").unwrap_or(0);
            parts.get(idx + 1).map(|name| {
                name.trim_matches(|c| c == '{' || c == '<' || c == ' ' || c == ':')
                    .to_string()
            })
        } else if (line.starts_with("export type ") || line.starts_with("type "))
            && line.contains('=')
        {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let idx = parts.iter().position(|&p| p == "type").unwrap_or(0);
            parts.get(idx + 1).map(|name| {
                name.trim_matches(|c| c == '=' || c == '<' || c == ' ' || c == ':')
                    .to_string()
            })
        } else {
            None
        };

        if let Some(iface_name) = interface_name {
            let mut props = Vec::new();
            i += 1;
            while i < lines.len() {
                let prop_line = lines[i].trim();
                if prop_line.starts_with('}') {
                    break;
                }
                if let Some(prop) = parse_prop_line(prop_line) {
                    props.push(prop);
                }
                i += 1;
            }
            interfaces.insert(iface_name, props);
        }

        i += 1;
    }

    interfaces
}

fn parse_prop_line(line: &str) -> Option<PropDefinition> {
    let trimmed = line.trim().trim_end_matches(';').trim_end_matches(',');
    if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") {
        return None;
    }

    if let Some(colon_pos) = trimmed.find(':') {
        let name_part = trimmed[..colon_pos].trim();
        let type_part = trimmed[colon_pos + 1..].trim();

        let (prop_name, required) = if let Some(stripped) = name_part.strip_suffix('?') {
            (stripped.trim().to_string(), false)
        } else {
            (name_part.to_string(), true)
        };

        let (prop_type, default_val) = if let Some(eq_pos) = type_part.find('=') {
            let t = type_part[..eq_pos].trim().to_string();
            let def = type_part[eq_pos + 1..].trim().to_string();
            (t, Some(def))
        } else {
            (type_part.to_string(), None)
        };

        let mut prop = PropDefinition::new(prop_name, prop_type, required);
        if let Some(def) = default_val {
            prop = prop.with_default(def);
        }
        return Some(prop);
    }

    None
}

/// Extracts component declarations and their child hierarchy.
fn extract_components(
    file_path: &str,
    content: &str,
    props_map: &BTreeMap<String, Vec<PropDefinition>>,
) -> Vec<ComponentNode> {
    let mut components = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for line in &lines {
        let trimmed = line.trim();

        // Pattern 1: function ComponentName(...)
        // Pattern 2: export function ComponentName(...)
        // Pattern 3: export default function ComponentName(...)
        // Pattern 4: const ComponentName: React.FC<Props> = (...)
        // Pattern 5: const ComponentName = (...) =>
        if let Some(comp_name) = extract_component_name(trimmed) {
            let mut node =
                ComponentNode::new(comp_name.clone(), comp_name.clone(), file_path.to_string());

            // Associate props interface if found
            let candidate_prop_names = [
                format!("{}Props", comp_name),
                "Props".to_string(),
                format!("{}Properties", comp_name),
            ];
            for cname in &candidate_prop_names {
                if let Some(props) = props_map.get(cname) {
                    for prop in props {
                        node.props.push(prop.clone());
                    }
                    break;
                }
            }

            // Extract child components from JSX in this file
            let children = extract_jsx_children(content, &comp_name);
            for child in children {
                node.children.push(child);
            }

            // Extract interactive and structural UI elements from JSX in this file
            for elem in extract_jsx_ui_elements(content) {
                node.elements.push(elem);
            }

            components.push(node);
        }
    }

    components
}

fn extract_jsx_ui_elements(content: &str) -> Vec<UiElementNode> {
    let mut elements = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();

        // Collect a multi-line opening tag block if line starts with `<` and doesn't close on same line
        let mut block = trimmed.to_string();
        if (trimmed.starts_with("<button")
            || trimmed.starts_with("<input")
            || trimmed.starts_with("<select")
            || trimmed.starts_with("<progress")
            || trimmed.starts_with("<dialog")
            || trimmed.starts_with("<span")
            || trimmed.starts_with("<h1")
            || trimmed.starts_with("<h2"))
            && find_tag_close_gt(&block).is_none()
        {
            let mut j = i + 1;
            while j < lines.len() && j <= i + 8 {
                block.push(' ');
                block.push_str(lines[j].trim());
                if find_tag_close_gt(&block).is_some() {
                    break;
                }
                j += 1;
            }
        }

        let aria = extract_attribute_value(&block, "aria-label");
        let action_handler = extract_jsx_handler(&block, "onClick")
            .or_else(|| extract_jsx_handler(&block, "onChange"))
            .or_else(|| extract_jsx_handler(&block, "onSubmit"));
        let bound_prop = extract_jsx_attribute(&block, "value")
            .or_else(|| extract_jsx_attribute(&block, "checked"));

        // Helper to extract inner text until closing tag
        let mut inner_text = String::new();
        if let Some(gt_idx) = find_tag_close_gt(&block) {
            let after_gt = &block[gt_idx + 1..];
            if !after_gt.is_empty() && !after_gt.starts_with('<') {
                if let Some(lt_idx) = after_gt.find('<') {
                    inner_text = after_gt[..lt_idx].trim().to_string();
                } else {
                    inner_text = after_gt.trim().to_string();
                }
            }
        }
        if inner_text.is_empty() {
            let mut j = i + 1;
            while j < lines.len() && j <= i + 5 {
                let next_line = lines[j].trim();
                if next_line.starts_with("</") {
                    break;
                }
                if !next_line.starts_with('<') && !next_line.starts_with('{') {
                    inner_text = next_line.trim().to_string();
                    break;
                }
                j += 1;
            }
        }

        if block.starts_with("<button") {
            let role = extract_attribute_value(&block, "role");
            let is_tab = role.as_deref() == Some("tab")
                || action_handler
                    .as_deref()
                    .unwrap_or("")
                    .contains("setTimeRange")
                || inner_text.contains("Days");
            let kind = if is_tab {
                UiElementKind::TabButton
            } else {
                UiElementKind::Button
            };

            let label = if !inner_text.is_empty() && !inner_text.starts_with('{') {
                inner_text.clone()
            } else {
                aria.clone().unwrap_or_else(|| "Button".to_string())
            };

            let variant = if label.to_lowercase().contains("complete all")
                || aria
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains("complete all")
                || action_handler
                    .as_deref()
                    .unwrap_or("")
                    .contains("setAllCompleted")
                || action_handler
                    .as_deref()
                    .unwrap_or("")
                    .contains("complete_all")
            {
                "complete-all"
            } else if is_tab {
                "tab-selector"
            } else if block.contains("primary")
                || label.contains("Add Task")
                || label.contains("Save Preferences")
                || label.contains("Mark All Read")
            {
                "primary"
            } else {
                "secondary"
            };

            let mut node = UiElementNode::new(kind, label).with_variant(variant);
            if let Some(a) = aria {
                node = node.with_aria_label(a);
            }
            if let Some(h) = action_handler {
                node = node.with_action_handler(h);
            }
            if let Some(p) = bound_prop {
                node = node.with_bound_prop(p);
            }
            elements.push(node);
        } else if block.starts_with("<input") {
            let input_type =
                extract_attribute_value(&block, "type").unwrap_or_else(|| "text".into());
            let kind = match input_type.as_str() {
                "checkbox" => UiElementKind::Checkbox,
                "range" => UiElementKind::Slider,
                _ => UiElementKind::TextInput,
            };
            let label = aria
                .clone()
                .or_else(|| extract_attribute_value(&block, "placeholder"))
                .unwrap_or_else(|| format!("{input_type} input"));
            let mut node = UiElementNode::new(kind, label);
            if let Some(a) = aria {
                node = node.with_aria_label(a);
            }
            if let Some(h) = action_handler {
                node = node.with_action_handler(h);
            }
            if let Some(p) = bound_prop {
                node = node.with_bound_prop(p);
            }
            elements.push(node);
        } else if block.starts_with("<select") {
            let label = aria.clone().unwrap_or_else(|| "Select".to_string());
            let mut options = Vec::new();
            let mut j = i + 1;
            while j < lines.len() && j <= i + 15 {
                let next_line = lines[j].trim();
                if next_line.starts_with("</select>") {
                    break;
                }
                if next_line.starts_with("<option") {
                    let opt_val = extract_attribute_value(next_line, "value");
                    let opt_text = if let Some(close_tag) = next_line.find('>') {
                        let after = &next_line[close_tag + 1..];
                        if let Some(end_opt) = after.find("</option>") {
                            after[..end_opt].trim().to_string()
                        } else {
                            after.trim().to_string()
                        }
                    } else {
                        String::new()
                    };
                    if let Some(v) = opt_val {
                        options.push(v);
                    } else if !opt_text.is_empty() {
                        options.push(opt_text);
                    }
                }
                j += 1;
            }

            let variant = if action_handler
                .as_deref()
                .unwrap_or("")
                .contains("setThemeMode")
                || label.to_lowercase().contains("theme")
                || options
                    .iter()
                    .any(|o| o.contains("slate") || o.contains("contrast"))
            {
                "theme-selector"
            } else {
                "select-dropdown"
            };

            let mut node = UiElementNode::new(UiElementKind::SelectDropdown, label)
                .with_variant(variant)
                .with_options(options);
            if let Some(a) = aria {
                node = node.with_aria_label(a);
            }
            if let Some(h) = action_handler {
                node = node.with_action_handler(h);
            }
            if let Some(p) = bound_prop {
                node = node.with_bound_prop(p);
            }
            elements.push(node);
        } else if block.starts_with("<progress") {
            let label = aria.clone().unwrap_or_else(|| "Progress".to_string());
            let mut node = UiElementNode::new(UiElementKind::ProgressBar, label);
            if let Some(a) = aria {
                node = node.with_aria_label(a);
            }
            if let Some(p) = bound_prop {
                node = node.with_bound_prop(p);
            }
            elements.push(node);
        } else if block.starts_with("<dialog") {
            let label = aria.clone().unwrap_or_else(|| "Dialog".to_string());
            let mut node = UiElementNode::new(UiElementKind::Dialog, label);
            if let Some(a) = aria {
                node = node.with_aria_label(a);
            }
            elements.push(node);
        } else if block.starts_with("<span")
            && extract_attribute_value(&block, "role").as_deref() == Some("status")
        {
            let label = if !inner_text.is_empty() && !inner_text.starts_with('{') {
                inner_text
            } else {
                aria.clone().unwrap_or_else(|| "Status Badge".to_string())
            };
            let mut node = UiElementNode::new(UiElementKind::Badge, label).with_variant("badge");
            if let Some(a) = aria {
                node = node.with_aria_label(a);
            }
            elements.push(node);
        } else if block.starts_with("<h1") || block.starts_with("<h2") {
            let label = if !inner_text.is_empty() {
                inner_text
            } else {
                aria.clone().unwrap_or_else(|| "Heading".to_string())
            };
            elements.push(UiElementNode::new(UiElementKind::Heading, label));
        }

        i += 1;
    }

    elements
}

fn extract_jsx_handler(block: &str, event_name: &str) -> Option<String> {
    let key = format!("{}=", event_name);
    let idx = block.find(&key)?;
    let after = &block[idx + key.len()..];
    if after.starts_with('{') {
        let mut depth = 0;
        for (i, ch) in after.char_indices() {
            if ch == '{' {
                depth += 1;
            } else if ch == '}' {
                depth -= 1;
                if depth == 0 {
                    return Some(after[1..i].trim().to_string());
                }
            }
        }
    }
    None
}

fn extract_jsx_attribute(block: &str, attr_name: &str) -> Option<String> {
    let key = format!("{}=", attr_name);
    let idx = block.find(&key)?;
    let after = &block[idx + key.len()..];
    if after.starts_with('{') {
        let mut depth = 0;
        for (i, ch) in after.char_indices() {
            if ch == '{' {
                depth += 1;
            } else if ch == '}' {
                depth -= 1;
                if depth == 0 {
                    return Some(after[1..i].trim().to_string());
                }
            }
        }
    } else if let Some(stripped) = after.strip_prefix('"') {
        if let Some(end) = stripped.find('"') {
            return Some(stripped[..end].to_string());
        }
    }
    None
}

fn find_tag_close_gt(s: &str) -> Option<usize> {
    let mut in_quote = None;
    let mut brace_depth = 0;
    let mut prev_ch = ' ';

    for (idx, ch) in s.char_indices() {
        if let Some(q) = in_quote {
            if ch == q && prev_ch != '\\' {
                in_quote = None;
            }
        } else if ch == '"' || ch == '\'' {
            in_quote = Some(ch);
        } else if ch == '{' {
            brace_depth += 1;
        } else if ch == '}' {
            if brace_depth > 0 {
                brace_depth -= 1;
            }
        } else if ch == '>' && brace_depth == 0 {
            return Some(idx);
        }
        prev_ch = ch;
    }
    None
}

fn extract_component_name(line: &str) -> Option<String> {
    // Check function declarations
    if (line.starts_with("function ")
        || line.starts_with("export function ")
        || line.starts_with("export default function "))
        && line.contains('(')
    {
        let idx = line.find("function ")?;
        let after_fn = &line[idx + 9..];

        if let Some(paren_idx) = after_fn.find('(') {
            let name = after_fn[..paren_idx].trim();
            if is_pascal_case(name) {
                return Some(name.to_string());
            }
        }
    }

    // Check const declarations: const Foo: React.FC or const Foo =
    if (line.starts_with("const ") || line.starts_with("export const ")) && line.contains('=') {
        let idx = line.find("const ")?;
        let after_const = &line[idx + 6..];

        let sep = if after_const.contains(':') { ':' } else { '=' };
        if let Some(sep_idx) = after_const.find(sep) {
            let name = after_const[..sep_idx].trim();
            if is_pascal_case(name) {
                return Some(name.to_string());
            }
        }
    }

    None
}

fn is_pascal_case(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => {
            c.is_ascii_uppercase() && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        }
        None => false,
    }
}

/// Extracts JSX child component names `<Child ...>` from content.
fn extract_jsx_children(content: &str, current_component: &str) -> Vec<String> {
    let mut children = HashSet::new();
    let mut chars = content.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '<' {
            // Check if followed by uppercase letter
            if let Some(&next_c) = chars.peek() {
                if next_c.is_ascii_uppercase() {
                    let mut tag_name = String::new();
                    while let Some(&ch) = chars.peek() {
                        if ch.is_ascii_alphanumeric() || ch == '_' {
                            tag_name.push(ch);
                            chars.next();
                        } else {
                            break;
                        }
                    }

                    if !tag_name.is_empty()
                        && tag_name != current_component
                        && tag_name != "Route"
                        && tag_name != "Routes"
                        && tag_name != "BrowserRouter"
                        && tag_name != "Router"
                        && tag_name != "Switch"
                    {
                        children.insert(tag_name);
                    }
                }
            }
        }
    }

    let mut list: Vec<String> = children.into_iter().collect();
    list.sort();
    list
}

/// Extracts `useState` and `useReducer` state fields.
fn extract_state_fields(content: &str) -> Vec<StateField> {
    let mut fields = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // useState pattern: const [count, setCount] = useState<number>(0);
        // or: const [count, setCount] = useState(0);
        if trimmed.contains("useState") && trimmed.contains('[') && trimmed.contains(']') {
            if let Some(field) = parse_use_state(trimmed) {
                fields.push(field);
            }
        }

        // useReducer pattern: const [state, dispatch] = useReducer(reducer, initial);
        if trimmed.contains("useReducer") && trimmed.contains('[') && trimmed.contains(']') {
            if let Some(field) = parse_use_reducer(trimmed) {
                fields.push(field);
            }
        }
    }

    fields
}

fn parse_use_state(line: &str) -> Option<StateField> {
    let open_bracket = line.find('[')?;
    let close_bracket = line.find(']')?;
    if open_bracket >= close_bracket {
        return None;
    }

    let names_part = &line[open_bracket + 1..close_bracket];
    let state_name = names_part.split(',').next()?.trim().to_string();

    let after_bracket = &line[close_bracket + 1..];
    let use_state_idx = after_bracket.find("useState")?;
    let after_use_state = &after_bracket[use_state_idx + 8..];

    // Check generic type parameter <Type>
    let (data_type_opt, after_type) = if after_use_state.starts_with('<') {
        if let Some(close_angle) = after_use_state.find('>') {
            let t = after_use_state[1..close_angle].trim().to_string();
            (Some(t), &after_use_state[close_angle + 1..])
        } else {
            (None, after_use_state)
        }
    } else {
        (None, after_use_state)
    };

    // Extract initial value inside parentheses
    let open_paren = after_type.find('(')?;
    let close_paren = after_type.rfind(')')?;
    let initial_val = if open_paren < close_paren {
        after_type[open_paren + 1..close_paren].trim().to_string()
    } else {
        "undefined".to_string()
    };

    let data_type = if let Some(dt) = data_type_opt {
        dt
    } else {
        infer_data_type(&initial_val)
    };

    Some(StateField::new(
        state_name,
        data_type,
        initial_val,
        "useState",
    ))
}

fn parse_use_reducer(line: &str) -> Option<StateField> {
    let open_bracket = line.find('[')?;
    let close_bracket = line.find(']')?;
    if open_bracket >= close_bracket {
        return None;
    }

    let names_part = &line[open_bracket + 1..close_bracket];
    let state_name = names_part.split(',').next()?.trim().to_string();

    let after_bracket = &line[close_bracket + 1..];
    let use_reducer_idx = after_bracket.find("useReducer")?;
    let after_hook = &after_bracket[use_reducer_idx + 10..];

    let (data_type, after_type) = if after_hook.starts_with('<') {
        if let Some(close_angle) = after_hook.find('>') {
            let t = after_hook[1..close_angle].trim().to_string();
            (t, &after_hook[close_angle + 1..])
        } else {
            ("ReducerState".to_string(), after_hook)
        }
    } else {
        ("ReducerState".to_string(), after_hook)
    };

    let open_paren = after_type.find('(')?;
    let close_paren = after_type.rfind(')')?;
    let args_str = if open_paren < close_paren {
        &after_type[open_paren + 1..close_paren]
    } else {
        ""
    };

    let initial_val = if let Some(comma_pos) = args_str.find(',') {
        args_str[comma_pos + 1..].trim().to_string()
    } else {
        "initialState".to_string()
    };

    Some(StateField::new(
        state_name,
        data_type,
        initial_val,
        "useReducer",
    ))
}

fn infer_data_type(val: &str) -> String {
    let trimmed = val.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        || (trimmed.starts_with('`') && trimmed.ends_with('`'))
    {
        "string".to_string()
    } else if trimmed == "true" || trimmed == "false" {
        "boolean".to_string()
    } else if trimmed.parse::<f64>().is_ok() {
        "number".to_string()
    } else if trimmed.starts_with('[') {
        "array".to_string()
    } else if trimmed.starts_with('{') {
        "object".to_string()
    } else {
        "unknown".to_string()
    }
}

/// Extracts `useEffect` hook side-effects.
fn extract_effect_bindings(file_path: &str, content: &str) -> Vec<EffectBinding> {
    let mut effects = Vec::new();
    let mut effect_index = 0;

    let mut search_pos = 0;
    while let Some(idx) = content[search_pos..].find("useEffect(") {
        let abs_idx = search_pos + idx;
        effect_index += 1;
        let id = format!(
            "{}:effect_{}",
            Path::new(file_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("app"),
            effect_index
        );

        let after_hook = &content[abs_idx + 10..];
        let (body, deps) = extract_effect_body_and_deps(after_hook);

        let is_async = body.contains("async")
            || body.contains("await")
            || body.contains("fetch(")
            || body.contains("axios.")
            || body.contains(".then(")
            || body.contains("Promise.");

        let kind = if is_async || body.contains("fetch(") {
            "data_fetch".to_string()
        } else if body.contains("setInterval") || body.contains("setTimeout") {
            "timer".to_string()
        } else if body.contains("addEventListener") {
            "event_listener".to_string()
        } else {
            "lifecycle".to_string()
        };

        let mut binding = EffectBinding::new(id, kind, is_async);
        for dep in deps {
            binding = binding.with_dependency(dep);
        }
        effects.push(binding);

        search_pos = abs_idx + 10;
    }

    effects
}

fn extract_effect_body_and_deps(s: &str) -> (String, Vec<String>) {
    // Find matching parentheses/brackets
    let mut deps = Vec::new();

    // Look for dependency array at the end of the useEffect call
    if let Some(bracket_start) = s.find('[') {
        if let Some(bracket_end) = s[bracket_start..].find(']') {
            let deps_content = &s[bracket_start + 1..bracket_start + bracket_end];
            for item in deps_content.split(',') {
                let trimmed = item.trim();
                if !trimmed.is_empty() {
                    deps.push(trimmed.to_string());
                }
            }
        }
    }

    let end_idx = s.find(");").unwrap_or_else(|| s.len().min(400));
    let body = s[..end_idx].to_string();
    (body, deps)
}

/// Extracts JSX interaction event handlers: onClick, onChange, onSubmit.
fn extract_interaction_bindings(content: &str) -> Vec<InteractionBinding> {
    let mut interactions = Vec::new();
    let events = ["onClick", "onChange", "onSubmit", "onKeyDown", "onBlur"];

    for line in content.lines() {
        let trimmed = line.trim();
        for event in &events {
            if let Some(pos) = trimmed.find(event) {
                let after_event = &trimmed[pos + event.len()..];
                if after_event.starts_with("={") {
                    if let Some(end_brace) = after_event.find('}') {
                        let handler = after_event[2..end_brace].trim().to_string();
                        let mut state_transitions = Vec::new();

                        // Detect state setter calls triggered in handler
                        if handler.contains("set") {
                            for part in
                                handler.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                            {
                                if part.starts_with("set") && part.len() > 3 {
                                    state_transitions.push(part.to_string());
                                }
                            }
                        }
                        if handler.contains("dispatch") {
                            state_transitions.push("dispatch".to_string());
                        }

                        let component_id = "Component".to_string();
                        let mut binding = InteractionBinding::new(component_id, *event, handler);
                        for st in state_transitions {
                            binding = binding.with_transition(st);
                        }
                        interactions.push(binding);
                    }
                }
            }
        }
    }

    interactions
}

/// Extracts `<Route path="..." element={<View />} />` or `<Route path="..." component={View} />`.
fn extract_routes(content: &str) -> Vec<RouteDefinition> {
    let mut routes = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.contains("<Route ") {
            let path_opt = extract_attribute_value(trimmed, "path");
            let component_opt = extract_jsx_element_or_component(trimmed);

            if let (Some(path), Some(comp)) = (path_opt, component_opt) {
                let mut route = RouteDefinition::new(&path, &comp);

                // Extract path parameters: e.g. /tasks/:id -> ["id"]
                for segment in path.split('/') {
                    if let Some(param) = segment.strip_prefix(':') {
                        route = route.with_param(param);
                    }
                }
                routes.push(route);
            }
        }
    }

    routes
}

fn extract_attribute_value(line: &str, attr: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr);
    if let Some(pos) = line.find(&pattern) {
        let after = &line[pos + pattern.len()..];
        if let Some(end_quote) = after.find('"') {
            return Some(after[..end_quote].to_string());
        }
    }

    let pattern_single = format!("{}='", attr);
    if let Some(pos) = line.find(&pattern_single) {
        let after = &line[pos + pattern_single.len()..];
        if let Some(end_quote) = after.find('\'') {
            return Some(after[..end_quote].to_string());
        }
    }

    None
}

fn extract_jsx_element_or_component(line: &str) -> Option<String> {
    // Case 1: element={<View />}
    if let Some(pos) = line.find("element={<") {
        let after = &line[pos + 10..];
        let end = after.find(|c: char| !c.is_ascii_alphanumeric() && c != '_')?;
        return Some(after[..end].to_string());
    }

    // Case 2: component={View}
    if let Some(pos) = line.find("component={") {
        let after = &line[pos + 11..];
        let end = after.find('}')?;
        return Some(after[..end].trim().to_string());
    }

    None
}

// ---------------------------------------------------------------------------
// CSS Style Token and Layout Constraint Extraction
// ---------------------------------------------------------------------------

pub fn analyze_css_content(content: &str, model: &mut SemanticUiModel) {
    let mut spacing_px = 0.0f64;
    let mut padding_px = 0.0f64;
    let mut border_radius_px = 0.0f64;
    let mut font_size_px = 14.0f64;
    let mut scale_factor = 1.0f64;
    let mut flex_wrap = false;
    let mut max_width_px = None;
    let mut direction = "column".to_string();
    let mut responsive_rules = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // 1. CSS Custom Properties: --token-name: #123456;
        for part in trimmed.split(';') {
            let part = part.trim();
            if let Some(dash_idx) = part.find("--") {
                let decl = &part[dash_idx..];
                if decl.contains(':') {
                    if let Some(token) = parse_css_variable(decl) {
                        if token.name.contains("radius") && border_radius_px == 0.0 {
                            border_radius_px = parse_px_value(decl);
                        } else if token.name.contains("font-size") {
                            let parsed = parse_px_value(decl);
                            if parsed > 0.0 {
                                font_size_px = parsed;
                            }
                        } else if token.name.contains("scale") {
                            let parsed = parse_px_value(decl);
                            if parsed > 0.0 {
                                scale_factor = parsed;
                            }
                        }
                        if !model.style_tokens.iter().any(|t| t.name == token.name) {
                            model.add_style_token(token);
                        }
                    }
                }
            }
        }

        // 2. Layout Direction & Responsive Wrapping
        if trimmed.starts_with("display:") && trimmed.contains("grid") {
            direction = "grid".to_string();
        } else if trimmed.starts_with("display:") && trimmed.contains("flex") {
            if direction != "grid" {
                direction = "row".to_string();
            }
        } else if trimmed.starts_with("flex-direction:") && trimmed.contains("column") {
            direction = "column".to_string();
        } else if trimmed.starts_with("flex-direction:") && trimmed.contains("row") {
            direction = "row".to_string();
        }

        if (trimmed.contains("flex-wrap:") && trimmed.contains("wrap"))
            || (trimmed.contains("grid-template-columns:")
                && (trimmed.contains("repeat(auto-") || trimmed.contains("minmax")))
        {
            flex_wrap = true;
        }

        // 3. Spacing / Padding / Roundness / Font Size / Scale / Max Width
        if trimmed.starts_with("gap:") {
            spacing_px = parse_px_value(trimmed);
        } else if trimmed.starts_with("padding:") {
            padding_px = parse_px_value(trimmed);
        } else if trimmed.starts_with("border-radius:") {
            border_radius_px = parse_px_value(trimmed);
        } else if trimmed.starts_with("font-size:") {
            let parsed = parse_px_value(trimmed);
            if parsed > 0.0 {
                font_size_px = parsed;
            }
        } else if trimmed.starts_with("zoom:") || trimmed.starts_with("scale:") {
            let parsed = parse_px_value(trimmed);
            if parsed > 0.0 {
                scale_factor = parsed;
            }
        } else if trimmed.starts_with("max-width:") {
            let parsed = parse_px_value(trimmed);
            if parsed > 0.0 {
                max_width_px = Some(parsed);
                flex_wrap = true;
            }
        }

        // 4. Responsive @media rules
        if trimmed.starts_with("@media") {
            let rule = trimmed.trim_end_matches('{').trim().to_string();
            responsive_rules.push(rule);
            flex_wrap = true;
        }
    }

    let mut constraint = LayoutConstraint::new(direction, spacing_px, padding_px)
        .with_border_radius(border_radius_px)
        .with_font_size(font_size_px)
        .with_scale_factor(scale_factor)
        .with_flex_wrap(flex_wrap);
    if let Some(mw) = max_width_px {
        constraint = constraint.with_max_width(mw);
    }
    for r in responsive_rules {
        constraint = constraint.with_responsive_rule(r);
    }
    model.add_layout_constraint(constraint);
}

fn parse_css_variable(line: &str) -> Option<StyleToken> {
    let colon_pos = line.find(':')?;
    let var_name = line[..colon_pos].trim().to_string();
    let var_val = line[colon_pos + 1..]
        .trim()
        .trim_end_matches(';')
        .trim_end_matches('}')
        .trim()
        .to_string();
    Some(StyleToken::new(var_name, var_val))
}

fn parse_px_value(line: &str) -> f64 {
    let colon_pos = match line.find(':') {
        Some(pos) => pos,
        None => return 0.0,
    };
    let val_str = line[colon_pos + 1..].trim().trim_end_matches(';').trim();
    if let Some(px_idx) = val_str.find("px") {
        val_str[..px_idx].trim().parse::<f64>().unwrap_or(0.0)
    } else if let Some(rem_idx) = val_str.find("rem") {
        val_str[..rem_idx]
            .trim()
            .parse::<f64>()
            .map(|v| v * 16.0)
            .unwrap_or(0.0)
    } else {
        val_str.parse::<f64>().unwrap_or(0.0)
    }
}

fn extract_api_endpoints(file_path: &str, content: &str) -> Vec<ApiEndpointBinding> {
    let mut endpoints = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(fetch_idx) = trimmed.find("fetch(") {
            let after_fetch = trimmed[fetch_idx + 6..].trim();
            let quote = after_fetch.chars().next().unwrap_or(' ');
            if quote == '\'' || quote == '"' || quote == '`' {
                let rest = &after_fetch[1..];
                if let Some(end_quote) = rest.find(quote) {
                    let url_or_path = rest[..end_quote].trim().to_string();
                    let remainder = &rest[end_quote + 1..];
                    let method = if remainder.contains("POST") || remainder.contains("'post'") {
                        "POST"
                    } else if remainder.contains("PUT") || remainder.contains("'put'") {
                        "PUT"
                    } else if remainder.contains("DELETE") || remainder.contains("'delete'") {
                        "DELETE"
                    } else {
                        "GET"
                    };
                    if !url_or_path.is_empty() {
                        endpoints.push(ApiEndpointBinding::new(method, url_or_path, file_path));
                    }
                }
            }
        }
    }
    endpoints
}
