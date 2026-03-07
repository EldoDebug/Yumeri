use std::collections::HashMap;
use std::path::Path;

use ron::extensions::Extensions;
use yumeri_ui::element::Element;
use yumeri_ui::template_provider::{TemplateBuildContext, TemplateProvider};

use crate::default_tokens::default_tokens;
use crate::resolve::resolve_template;
use crate::schema::Template;
use crate::token::{validate_tokens, TokenValue};

fn ron_options() -> ron::Options {
    ron::Options::default().with_default_extension(Extensions::IMPLICIT_SOME)
}

pub struct TemplateRegistry {
    templates: HashMap<String, Template>,
    global_tokens: HashMap<String, TokenValue>,
}

impl TemplateRegistry {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
            global_tokens: default_tokens(),
        }
    }

    pub fn load_file(&mut self, path: &Path) -> Result<(), String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
        let template: Template = ron_options().from_str(&content)
            .map_err(|e| format!("failed to parse {}: {}", path.display(), e))?;

        self.validate_template_tokens(&template)
            .map_err(|e| format!("token validation failed in {}: {}", path.display(), e))?;

        self.templates.insert(template.name.clone(), template);
        Ok(())
    }

    pub fn load_dir(&mut self, dir: &Path) -> Result<(), String> {
        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("failed to read directory {}: {}", dir.display(), e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("failed to read entry: {}", e))?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("ron") {
                self.load_file(&path)?;
            }
        }
        Ok(())
    }

    pub fn load_str(&mut self, content: &str) -> Result<(), String> {
        let template: Template = ron_options().from_str(content)
            .map_err(|e| format!("failed to parse template: {}", e))?;

        self.validate_template_tokens(&template)?;

        self.templates.insert(template.name.clone(), template);
        Ok(())
    }

    fn validate_template_tokens(&self, template: &Template) -> Result<(), String> {
        if template.tokens.is_empty() {
            return validate_tokens(&self.global_tokens);
        }
        let mut combined = self.global_tokens.clone();
        for (k, v) in &template.tokens {
            combined.insert(k.clone(), v.clone());
        }
        validate_tokens(&combined)
    }

    pub fn get(&self, name: &str) -> Option<&Template> {
        self.templates.get(name)
    }

    pub fn set_token(&mut self, key: &str, value: TokenValue) {
        self.global_tokens.insert(key.to_string(), value);
    }

    pub fn set_tokens(&mut self, tokens: HashMap<String, TokenValue>) {
        self.global_tokens = tokens;
    }

    pub fn global_tokens(&self) -> &HashMap<String, TokenValue> {
        &self.global_tokens
    }

    /// Load a tokens file (RON HashMap format) and merge into global tokens.
    /// Existing tokens with the same key are overwritten; others are preserved.
    pub fn load_tokens_file(&mut self, path: &Path) -> Result<(), String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
        let tokens: HashMap<String, TokenValue> = ron_options().from_str(&content)
            .map_err(|e| format!("failed to parse tokens {}: {}", path.display(), e))?;
        for (k, v) in tokens {
            self.global_tokens.insert(k, v);
        }
        Ok(())
    }
}

impl Default for TemplateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateProvider for TemplateRegistry {
    fn build_template(&self, name: &str, ctx: TemplateBuildContext) -> Option<Element> {
        let template = self.get(name)?;
        Some(resolve_template(template, &self.global_tokens, ctx))
    }
}
