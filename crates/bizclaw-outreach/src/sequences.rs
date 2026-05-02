use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sequence {
    pub id: String,
    pub name: String,
    pub steps: Vec<SequenceStep>,
    pub delay_days: i64,
    pub max_followups: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceStep {
    pub step_number: u32,
    pub step_type: StepType,
    pub delay_days: i64,
    pub template_id: Option<String>,
    pub subject: Option<String>,
    pub content: Option<String>,
    pub condition: Option<StepCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StepType {
    Email,
    Zalo,
    LinkedIn,
    Wait,
    Task,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepCondition {
    pub check: ConditionCheck,
    pub value: String,
    pub then_action: StepType,
    pub else_action: Option<StepType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionCheck {
    Opened,
    Clicked,
    Replied,
    VisitedProfile,
}

impl Sequence {
    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            steps: vec![],
            delay_days: 1,
            max_followups: 3,
        }
    }

    pub fn add_email_step(
        mut self,
        delay_days: i64,
        subject: &str,
        content: &str,
    ) -> Self {
        self.steps.push(SequenceStep {
            step_number: self.steps.len() as u32 + 1,
            step_type: StepType::Email,
            delay_days,
            template_id: None,
            subject: Some(subject.to_string()),
            content: Some(content.to_string()),
            condition: None,
        });
        self
    }

    pub fn add_zalo_step(
        mut self,
        delay_days: i64,
        message: &str,
    ) -> Self {
        self.steps.push(SequenceStep {
            step_number: self.steps.len() as u32 + 1,
            step_type: StepType::Zalo,
            delay_days,
            template_id: None,
            subject: None,
            content: Some(message.to_string()),
            condition: None,
        });
        self
    }

    pub fn add_followup_sequence(mut self, max_followups: u32) -> Self {
        self.max_followups = max_followups;
        self
    }

    pub fn calculate_send_time(&self, start_time: DateTime<Utc>, step: u32) -> DateTime<Utc> {
        let total_delay: i64 = self.steps[..step as usize]
            .iter()
            .map(|s| s.delay_days)
            .sum();
        start_time + Duration::days(total_delay)
    }
}

pub fn vn_cold_email_templates() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        (
            "cold_email_basic",
            "Giới thiệu dịch vụ",
            r#"Xin chào {{name}},

Tôi là {{sender_name}} từ {{company}}.

Tôi biết bạn đang {{pain_point}} và tôi nghĩ chúng tôi có thể giúp bạn tiết kiệm {{benefit}}.

Bạn có muốn dành 15 phút để trao đổi không?

Trân trọng,
{{sender_name}}"#,
        ),
        (
            "cold_email_followup",
            "Follow-up sau 3 ngày",
            r#"Xin chào {{name}},

Tôi đã gửi email cách đây 3 ngày nhưng chưa nhận được phản hồi.

Tôi hiểu bạn có thể bận, nhưng tôi muốn chia sẻ rằng {{value_proposition}}.

Có thể chúng ta sắp xếp một cuộc gọi ngắn vào {{next_week}} không?

{{sender_name}}"#,
        ),
        (
            "zalo_intro",
            "Tin nhắn Zalo giới thiệu",
            "Chào anh/chị {{name}}! 👋\n\nTôi là {{sender_name}}, có thể giúp anh/chị về {{service}}.\n\nAnh/chị có quan tâm không ạ?",
        ),
    ]
}
