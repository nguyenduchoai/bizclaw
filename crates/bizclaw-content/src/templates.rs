use crate::types::{ContentPlatform, ContentType, TemplateVariable};
use anyhow::{Context, Result};
use std::collections::HashMap;

pub struct TemplateManager {
    templates: HashMap<String, Template>,
}

#[derive(Debug, Clone)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub platform: ContentPlatform,
    pub content_type: ContentType,
    pub prompt_template: String,
    pub variables: Vec<TemplateVariable>,
}

impl TemplateManager {
    pub fn new() -> Self {
        let mut manager = Self {
            templates: HashMap::new(),
        };
        manager.load_default_templates();
        manager
    }

    pub fn get_template(&self, id: &str) -> Option<&Template> {
        self.templates.get(id)
    }

    pub fn list_templates(&self) -> Vec<&Template> {
        self.templates.values().collect()
    }

    pub fn list_templates_by_platform(&self, platform: &ContentPlatform) -> Vec<&Template> {
        self.templates
            .values()
            .filter(|t| &t.platform == platform)
            .collect()
    }

    pub fn list_templates_by_category(&self, category: &str) -> Vec<&Template> {
        self.templates
            .values()
            .filter(|t| t.name.to_lowercase().contains(&category.to_lowercase()))
            .collect()
    }

    pub fn render_template(
        &self,
        template_id: &str,
        variables: &HashMap<String, String>,
    ) -> Result<String> {
        let template = self
            .templates
            .get(template_id)
            .context("Template not found")?;

        let mut result = template.prompt_template.clone();
        for var in &template.variables {
            let placeholder = format!("{{{{{}}}}}", var.name);
            let value = variables
                .get(&var.name)
                .or(var.default_value.as_ref())
                .context(format!("Missing required variable: {}", var.name))?;
            result = result.replace(&placeholder, value);
        }

        Ok(result)
    }

    pub fn import_template(&mut self, template: Template) {
        self.templates.insert(template.id.clone(), template);
    }

    fn load_default_templates(&mut self) {
        let default_templates = vec![
            Template {
                id: "fb-product-launch".to_string(),
                name: "Product Launch".to_string(),
                platform: ContentPlatform::Facebook,
                content_type: ContentType::Post,
                prompt_template: r#"Viết bài đăng Facebook giới thiệu sản phẩm mới.

Sản phẩm: {{{product_name}}}
Giá: {{{price}}}
Mô tả: {{{description}}}

Yêu cầu:
- Hook gây chú ý trong 2 dòng đầu
- 3-5 bullet points về tính năng nổi bật
- Call-to-action rõ ràng
- 2-3 hashtags phù hợp"#
                    .to_string(),
                variables: vec![
                    TemplateVariable {
                        name: "product_name".to_string(),
                        description: "Tên sản phẩm".to_string(),
                        required: true,
                        default_value: None,
                    },
                    TemplateVariable {
                        name: "price".to_string(),
                        description: "Giá sản phẩm".to_string(),
                        required: true,
                        default_value: None,
                    },
                    TemplateVariable {
                        name: "description".to_string(),
                        description: "Mô tả sản phẩm".to_string(),
                        required: true,
                        default_value: None,
                    },
                ],
            },
            Template {
                id: "zalo-ou".to_string(),
                name: "Zalo OA Announcement".to_string(),
                platform: ContentPlatform::Zalo,
                content_type: ContentType::Announcement,
                prompt_template: r#"Viết thông báo cho Zalo Official Account.

Tiêu đề: {{{title}}}
Nội dung: {{{content}}}

Yêu cầu:
- Ngôn ngữ thân thiện, gần gũi
- Phù hợp với người dùng Việt Nam
- Có thể copy-paste trực tiếp"#
                    .to_string(),
                variables: vec![
                    TemplateVariable {
                        name: "title".to_string(),
                        description: "Tiêu đề thông báo".to_string(),
                        required: true,
                        default_value: None,
                    },
                    TemplateVariable {
                        name: "content".to_string(),
                        description: "Nội dung chi tiết".to_string(),
                        required: true,
                        default_value: None,
                    },
                ],
            },
            Template {
                id: "shopee-product-desc".to_string(),
                name: "Shopee Product Description".to_string(),
                platform: ContentPlatform::Shopee,
                content_type: ContentType::ProductDescription,
                prompt_template: r#"Viết mô tả sản phẩm cho Shopee.

Tên sản phẩm: {{{product_name}}}
Danh mục: {{{category}}}
Giá gốc: {{{original_price}}}
Giá bán: {{{sale_price}}}

Yêu cầu:
- Mô tả chi tiết, đầy đủ thông tin
- Highlight USP (điểm bán hàng độc nhất)
- Ưu điểm nổi bật so với đối thủ
- Thông tin vận chuyển, bảo hành
- Keywords cho SEO tìm kiếm"#
                    .to_string(),
                variables: vec![
                    TemplateVariable {
                        name: "product_name".to_string(),
                        description: "Tên sản phẩm".to_string(),
                        required: true,
                        default_value: None,
                    },
                    TemplateVariable {
                        name: "category".to_string(),
                        description: "Danh mục sản phẩm".to_string(),
                        required: true,
                        default_value: None,
                    },
                    TemplateVariable {
                        name: "original_price".to_string(),
                        description: "Giá gốc".to_string(),
                        required: false,
                        default_value: Some("Liên hệ".to_string()),
                    },
                    TemplateVariable {
                        name: "sale_price".to_string(),
                        description: "Giá bán".to_string(),
                        required: true,
                        default_value: None,
                    },
                ],
            },
            Template {
                id: "email-newsletter".to_string(),
                name: "Email Newsletter".to_string(),
                platform: ContentPlatform::Email,
                content_type: ContentType::Newsletter,
                prompt_template: r#"Viết email newsletter.

Chủ đề: {{{subject}}}
Nội dung chính: {{{main_content}}}
CTA: {{{call_to_action}}}

Yêu cầu:
- Subject line hấp dẫn, tăng open rate
- Preview text gây tò mò
- Cấu trúc: Greeting -> Value -> Main content -> CTA
- Personal signing"#
                    .to_string(),
                variables: vec![
                    TemplateVariable {
                        name: "subject".to_string(),
                        description: "Subject line".to_string(),
                        required: true,
                        default_value: None,
                    },
                    TemplateVariable {
                        name: "main_content".to_string(),
                        description: "Nội dung chính".to_string(),
                        required: true,
                        default_value: None,
                    },
                    TemplateVariable {
                        name: "call_to_action".to_string(),
                        description: "Call to action button text".to_string(),
                        required: false,
                        default_value: Some("Xem ngay".to_string()),
                    },
                ],
            },
            Template {
                id: "fb-customer-reply".to_string(),
                name: "Customer Reply Template".to_string(),
                platform: ContentPlatform::Facebook,
                content_type: ContentType::Reply,
                prompt_template: r#"Viết trả lời khách hàng Facebook.

Loại câu hỏi: {{{question_type}}}
Tên khách: {{{customer_name}}}
Nội dung câu hỏi: {{{question}}}

Yêu cầu:
- Thân thiện, chuyên nghiệp
- Trả lời đúng trọng tâm
- Có upsell/cross-sell nếu phù hợp"#
                    .to_string(),
                variables: vec![
                    TemplateVariable {
                        name: "question_type".to_string(),
                        description: "Loại câu hỏi (hỏi giá, hỏi size, khiếu nại...)".to_string(),
                        required: true,
                        default_value: None,
                    },
                    TemplateVariable {
                        name: "customer_name".to_string(),
                        description: "Tên khách hàng".to_string(),
                        required: false,
                        default_value: Some("Quý khách".to_string()),
                    },
                    TemplateVariable {
                        name: "question".to_string(),
                        description: "Nội dung câu hỏi".to_string(),
                        required: true,
                        default_value: None,
                    },
                ],
            },
            Template {
                id: "tiktok-hook".to_string(),
                name: "TikTok Video Hook".to_string(),
                platform: ContentPlatform::TikTok,
                content_type: ContentType::Reel,
                prompt_template: r#"Viết kịch bản TikTok video.

Chủ đề: {{{topic}}}
Thời lượng: {{{duration}}}

Yêu cầu:
- Hook mạnh trong 3 giây đầu
- Tốc độ nhanh, hấp dẫn
- Pattern switch để giữ viewer
- CTA cuối video"#
                    .to_string(),
                variables: vec![
                    TemplateVariable {
                        name: "topic".to_string(),
                        description: "Chủ đề video".to_string(),
                        required: true,
                        default_value: None,
                    },
                    TemplateVariable {
                        name: "duration".to_string(),
                        description: "Thời lượng video".to_string(),
                        required: false,
                        default_value: Some("15-30s".to_string()),
                    },
                ],
            },
        ];

        for template in default_templates {
            self.templates.insert(template.id.clone(), template);
        }
    }
}

impl Default for TemplateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_rendering() {
        let manager = TemplateManager::new();
        let mut vars = HashMap::new();
        vars.insert("product_name".to_string(), "Áo Thun Nam".to_string());
        vars.insert("price".to_string(), "299.000đ".to_string());
        vars.insert(
            "description".to_string(),
            "Chất liệu cotton 100%, thoáng mát".to_string(),
        );

        let result = manager.render_template("fb-product-launch", &vars);
        assert!(result.is_ok());

        let rendered = result.unwrap();
        assert!(rendered.contains("Áo Thun Nam"));
        assert!(rendered.contains("299.000đ"));
    }

    #[test]
    fn test_list_templates_by_platform() {
        let manager = TemplateManager::new();
        let fb_templates = manager.list_templates_by_platform(&ContentPlatform::Facebook);

        assert!(!fb_templates.is_empty());
        assert!(
            fb_templates
                .iter()
                .all(|t| t.platform == ContentPlatform::Facebook)
        );
    }
}
