use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use mae_domain::{ContentHash, DomainError, PromptProvenance, PromptProviderPort, PromptSource};

/// Structured representation of an Open Prompt Format (OPF) 0.2 prompt template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpfPrompt {
    pub id: String,
    pub format: String,
    pub description: String,
    pub tags: Vec<String>,
    pub required_inputs: Vec<String>,
    pub optional_inputs: Vec<String>,
    pub template: String,
}

impl OpfPrompt {
    /// Parses an OPF 0.2 markdown file content with YAML frontmatter.
    pub fn parse(content: &str) -> Result<Self, DomainError> {
        let trimmed = content.trim_start();
        if !trimmed.starts_with("---") {
            return Err(DomainError::PortError(
                "Missing opening '---' delimiter in OPF frontmatter".to_string(),
            ));
        }

        // Find the closing delimiter
        let rest = &trimmed[3..];
        let end_idx = rest.find("\n---").ok_or_else(|| {
            DomainError::PortError("Missing closing '---' delimiter in OPF frontmatter".to_string())
        })?;

        let frontmatter = &rest[..end_idx];
        let body = rest[end_idx + 4..].trim_start_matches(['\r', '\n']);

        let mut id = None;
        let mut format = None;
        let mut description = String::new();
        let mut tags = Vec::new();
        let mut required_inputs = Vec::new();
        let mut optional_inputs = Vec::new();

        let mut in_inputs_block = false;
        let mut current_input_name: Option<String> = None;
        let mut current_input_required = true;

        let flush_current_input = |name: Option<String>,
                                   required: bool,
                                   reqs: &mut Vec<String>,
                                   opts: &mut Vec<String>| {
            if let Some(var_name) = name {
                if required {
                    if !reqs.contains(&var_name) {
                        reqs.push(var_name);
                    }
                } else if !opts.contains(&var_name) {
                    opts.push(var_name);
                }
            }
        };

        for line in frontmatter.lines() {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() || line_trimmed.starts_with('#') {
                continue;
            }

            // Check if top-level key
            if !line.starts_with(' ') && !line.starts_with('\t') {
                if in_inputs_block {
                    flush_current_input(
                        current_input_name.take(),
                        current_input_required,
                        &mut required_inputs,
                        &mut optional_inputs,
                    );
                    in_inputs_block = false;
                }

                if let Some(val) = line_trimmed.strip_prefix("format:") {
                    format = Some(val.trim().to_string());
                } else if let Some(val) = line_trimmed.strip_prefix("id:") {
                    id = Some(val.trim().to_string());
                } else if let Some(val) = line_trimmed.strip_prefix("description:") {
                    description = val.trim().to_string();
                } else if let Some(val) = line_trimmed.strip_prefix("tags:") {
                    let tags_str = val.trim().trim_start_matches('[').trim_end_matches(']');
                    tags = tags_str
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                } else if line_trimmed == "inputs:" {
                    in_inputs_block = true;
                }
                continue;
            }

            if in_inputs_block {
                let indent = line.len() - line.trim_start().len();
                let trimmed_entry = line.trim();

                // If indented entry defining a new variable name (e.g. "  context_path:")
                if indent <= 4 && trimmed_entry.ends_with(':') {
                    flush_current_input(
                        current_input_name.take(),
                        current_input_required,
                        &mut required_inputs,
                        &mut optional_inputs,
                    );
                    let var_name = trimmed_entry.trim_end_matches(':').trim().to_string();
                    current_input_name = Some(var_name);
                    current_input_required = true; // default in OPF
                } else if let Some(req_val) = trimmed_entry.strip_prefix("required:") {
                    if req_val.trim() == "false" {
                        current_input_required = false;
                    } else if req_val.trim() == "true" {
                        current_input_required = true;
                    }
                }
            }
        }

        if in_inputs_block {
            flush_current_input(
                current_input_name.take(),
                current_input_required,
                &mut required_inputs,
                &mut optional_inputs,
            );
        }

        let format_str = format.ok_or_else(|| {
            DomainError::PortError("Missing 'format:' field in OPF frontmatter".to_string())
        })?;
        if format_str != "opf/0.2" {
            return Err(DomainError::PortError(format!(
                "Invalid OPF format '{}': expected 'opf/0.2'",
                format_str
            )));
        }

        let id_str = id.ok_or_else(|| {
            DomainError::PortError("Missing 'id:' field in OPF frontmatter".to_string())
        })?;

        Ok(Self {
            id: id_str,
            format: format_str,
            description,
            tags,
            required_inputs,
            optional_inputs,
            template: body.to_string(),
        })
    }

    /// Renders placeholders `{{ var }}` and computes cryptographic provenance.
    pub fn render(&self, vars: &[(&str, &str)]) -> Result<(String, PromptProvenance), DomainError> {
        // Validate that all required inputs are provided
        for req_var in &self.required_inputs {
            if !vars.iter().any(|(k, _)| *k == req_var.as_str()) {
                return Err(DomainError::PortError(format!(
                    "Missing required input variable '{}' for prompt '{}'",
                    req_var, self.id
                )));
            }
        }

        let mut rendered = self.template.clone();
        for (key, val) in vars {
            // Replace various forms of placeholder whitespace: {{ key }}, {{key}}, {{  key  }}
            let placeholder_spaced = format!("{{{{ {} }}}}", key);
            let placeholder_tight = format!("{{{{{}}}}}", key);
            rendered = rendered.replace(&placeholder_spaced, val);
            rendered = rendered.replace(&placeholder_tight, val);

            // Also check generic whitespace between {{ and }}
            let search_prefix = "{{";
            while let Some(start) = rendered.find(search_prefix) {
                if let Some(rel_end) = rendered[start..].find("}}") {
                    let end = start + rel_end + 2;
                    let inner = rendered[start + 2..end - 2].trim();
                    if inner == *key {
                        rendered.replace_range(start..end, val);
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }

        // Deterministic content digest: canonical template + sorted variables
        let mut digest_payload = format!("template_id:{}\ntemplate:{}\n", self.id, self.template);
        let mut sorted_vars = vars.to_vec();
        sorted_vars.sort_by_key(|(k, _)| *k);
        for (k, v) in &sorted_vars {
            digest_payload.push_str(&format!("var:{}={}\n", k, v));
        }
        let content_digest = ContentHash::compute(digest_payload.as_bytes()).0;

        let provenance = PromptProvenance {
            prompt_id: self.id.clone(),
            content_digest,
            rendered_vars: sorted_vars
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        };

        Ok((rendered, provenance))
    }
}

/// Prompt catalog provider implementing Open Prompt Format (OPF) 0.2.
#[derive(Debug, Clone)]
pub struct OpfPromptProvider {
    prompts: BTreeMap<String, OpfPrompt>,
}

impl OpfPromptProvider {
    pub fn new() -> Self {
        Self {
            prompts: BTreeMap::new(),
        }
    }

    /// Loads prompts from root directory containing `opf.yaml` and `prompts/`.
    pub fn from_root(root_dir: impl AsRef<Path>) -> Result<Self, DomainError> {
        let root = root_dir.as_ref();
        let registry_path = if root.join("opf.yaml").exists() {
            root.join("opf.yaml")
        } else if root.join("../../opf.yaml").exists() {
            root.join("../../opf.yaml")
        } else {
            return Err(DomainError::PortError(format!(
                "OPF registry not found at {}",
                root.join("opf.yaml").display()
            )));
        };

        let resolved_root = registry_path.parent().unwrap_or(root);
        let registry_content = fs::read_to_string(&registry_path).map_err(|e| {
            DomainError::PortError(format!(
                "Failed to read OPF registry {}: {}",
                registry_path.display(),
                e
            ))
        })?;

        let mut provider = Self::new();
        provider.load_registry(resolved_root, &registry_content)?;
        Ok(provider)
    }

    /// Loads prompts from an in-memory collection of prompt file contents.
    pub fn from_in_memory(files: &[(&str, &str)]) -> Result<Self, DomainError> {
        let mut provider = Self::new();
        for (name, content) in files {
            provider.register_prompt(name, content)?;
        }
        Ok(provider)
    }

    /// Registers a single raw OPF prompt content string.
    pub fn register_prompt(&mut self, prompt_name: &str, content: &str) -> Result<(), DomainError> {
        let parsed = OpfPrompt::parse(content)?;
        self.prompts.insert(prompt_name.to_string(), parsed);
        Ok(())
    }

    fn load_registry(&mut self, root: &Path, registry_yaml: &str) -> Result<(), DomainError> {
        let mut in_prompts = false;
        let mut current_prompt_id: Option<String> = None;

        for line in registry_yaml.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if line.starts_with("prompts:") {
                in_prompts = true;
                continue;
            }

            if in_prompts {
                let indent = line.len() - line.trim_start().len();
                if indent == 2 && trimmed.ends_with(':') {
                    let prompt_id = trimmed.trim_end_matches(':').trim().to_string();
                    current_prompt_id = Some(prompt_id);
                } else if indent >= 4 && trimmed.starts_with("source:") {
                    if let Some(ref pid) = current_prompt_id {
                        let rel_path = trimmed
                            .strip_prefix("source:")
                            .unwrap_or("")
                            .trim()
                            .trim_matches('"')
                            .trim_matches('\'');
                        let prompt_file = root.join(rel_path);
                        let file_content = fs::read_to_string(&prompt_file).map_err(|e| {
                            DomainError::PortError(format!(
                                "Failed to read prompt file {}: {}",
                                prompt_file.display(),
                                e
                            ))
                        })?;
                        self.register_prompt(pid, &file_content)?;
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for OpfPromptProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptProviderPort for OpfPromptProvider {
    fn get_prompt(&self, prompt_name: &str) -> Result<PromptSource, DomainError> {
        let prompt = self.prompts.get(prompt_name).ok_or_else(|| {
            DomainError::PortError(format!("Prompt '{}' not found in OPF catalog", prompt_name))
        })?;

        Ok(PromptSource {
            id: prompt.id.clone(),
            name: prompt.id.clone(),
            template: prompt.template.clone(),
            variables: prompt.required_inputs.clone(),
        })
    }

    fn render_prompt(
        &self,
        prompt_name: &str,
        vars: &[(&str, &str)],
    ) -> Result<(String, PromptProvenance), DomainError> {
        let prompt = self.prompts.get(prompt_name).ok_or_else(|| {
            DomainError::PortError(format!("Prompt '{}' not found in OPF catalog", prompt_name))
        })?;

        prompt.render(vars)
    }
}

/// Backward compatibility provider for legacy bundled markdown templates without OPF frontmatter.
#[derive(Debug, Clone, Default)]
pub struct LegacyPromptProvider {
    templates: BTreeMap<String, String>,
}

impl LegacyPromptProvider {
    pub fn new() -> Self {
        Self {
            templates: BTreeMap::new(),
        }
    }

    pub fn register_template(&mut self, name: impl Into<String>, template: impl Into<String>) {
        self.templates.insert(name.into(), template.into());
    }
}

impl PromptProviderPort for LegacyPromptProvider {
    fn get_prompt(&self, prompt_name: &str) -> Result<PromptSource, DomainError> {
        let template = self.templates.get(prompt_name).ok_or_else(|| {
            DomainError::PortError(format!(
                "Legacy template '{}' not found in catalog",
                prompt_name
            ))
        })?;

        Ok(PromptSource {
            id: prompt_name.to_string(),
            name: prompt_name.to_string(),
            template: template.clone(),
            variables: Vec::new(),
        })
    }

    fn render_prompt(
        &self,
        prompt_name: &str,
        vars: &[(&str, &str)],
    ) -> Result<(String, PromptProvenance), DomainError> {
        let template = self.templates.get(prompt_name).ok_or_else(|| {
            DomainError::PortError(format!(
                "Legacy template '{}' not found in catalog",
                prompt_name
            ))
        })?;

        let mut rendered = template.clone();
        for (k, v) in vars {
            rendered = rendered.replace(&format!("{{{{ {} }}}}", k), v);
            rendered = rendered.replace(&format!("{{{{{}}}}}", k), v);
        }

        let mut sorted_vars = vars.to_vec();
        sorted_vars.sort_by_key(|(k, _)| *k);
        let mut digest_payload = format!("legacy_prompt:{}\ntemplate:{}\n", prompt_name, template);
        for (k, v) in &sorted_vars {
            digest_payload.push_str(&format!("var:{}={}\n", k, v));
        }
        let content_digest = ContentHash::compute(digest_payload.as_bytes()).0;

        let provenance = PromptProvenance {
            prompt_id: prompt_name.to_string(),
            content_digest,
            rendered_vars: sorted_vars
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        };

        Ok((rendered, provenance))
    }
}
