use std::collections::HashMap;
use anyhow::Result;
use crate::sequences::vn_cold_email_templates;

pub struct TemplateEngine {
    templates: HashMap<String, Template>,
}

#[derive(Debug, Clone)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub content: String,
    pub variables: Vec<String>,
}

impl TemplateEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            templates: HashMap::new(),
        };
        engine.load_vietnamese_templates();
        engine
    }

    fn load_vietnamese_templates(&mut self) {
        for (id, name, content) in vn_cold_email_templates() {
            let variables = Self::extract_variables(content);
            self.templates.insert(
                id.to_string(),
                Template {
                    id: id.to_string(),
                    name: name.to_string(),
                    content: content.to_string(),
                    variables,
                },
            );
        }
    }

    fn extract_variables(content: &str) -> Vec<String> {
        content
            .match_indices("{{")
            .filter_map(|(start, _)| {
                content[start..].find("}}").map(|end| {
                    content[start + 2..start + end].to_string()
                })
            })
            .collect()
    }

    pub fn personalize(&self, template_id: &str, _lead_id: &str) -> Result<String> {
        let template = self
            .templates
            .get(template_id)
            .ok_or_else(|| anyhow::anyhow!("Template not found: {}", template_id))?;

        let mut content = template.content.clone();
        
        let replacements: HashMap<&str, &str> = [
            ("name", "Anh/Chị"),
            ("company", "Công ty của bạn"),
            ("sender_name", "BizClaw"),
            ("benefit", "50% chi phí vận hành"),
            ("pain_point", "quản lý công việc"),
            ("value_proposition", "giải pháp của chúng tôi đã giúp 100+ doanh nghiệp"),
            ("next_week", "tuần sau"),
            ("service", "tự động hóa marketing"),
        ]
        .iter()
        .cloned()
        .collect();

        for (var, value) in replacements {
            let pattern = format!("{{{{{}}}}}", var);
            content = content.replace(&pattern, value);
        }

        Ok(content)
    }

    pub fn list_templates(&self) -> Vec<&Template> {
        self.templates.values().collect()
    }

    pub fn get_template(&self, id: &str) -> Option<&Template> {
        self.templates.get(id)
    }

    pub fn add_template(&mut self, id: &str, name: &str, content: &str) {
        let variables = Self::extract_variables(content);
        self.templates.insert(
            id.to_string(),
            Template {
                id: id.to_string(),
                name: name.to_string(),
                content: content.to_string(),
                variables,
            },
        );
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}
