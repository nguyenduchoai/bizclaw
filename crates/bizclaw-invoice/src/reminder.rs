use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reminder {
    pub id: String,
    pub invoice_id: String,
    pub reminder_type: ReminderType,
    pub scheduled_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub status: ReminderStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderType {
    DueIn7Days,
    DueIn3Days,
    DueToday,
    Overdue1Day,
    Overdue3Days,
    Overdue7Days,
    Overdue14Days,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ReminderStatus {
    Scheduled,
    Sent,
    Cancelled,
    Failed,
}

pub struct ReminderScheduler {
    reminders: RwLock<Vec<Reminder>>,
}

impl ReminderScheduler {
    pub fn new() -> Self {
        Self {
            reminders: RwLock::new(Vec::new()),
        }
    }

    pub async fn schedule(&self, invoice_id: &str, days_before: i64) -> anyhow::Result<()> {
        let reminder = Reminder {
            id: Uuid::new_v4().to_string(),
            invoice_id: invoice_id.to_string(),
            reminder_type: Self::determine_type(days_before),
            scheduled_at: Utc::now() + Duration::days(days_before),
            sent_at: None,
            status: ReminderStatus::Scheduled,
        };

        let mut reminders = self.reminders.write().await;
        reminders.push(reminder);
        Ok(())
    }

    fn determine_type(days_before: i64) -> ReminderType {
        match days_before {
            7 => ReminderType::DueIn7Days,
            3 => ReminderType::DueIn3Days,
            0 => ReminderType::DueToday,
            -1 => ReminderType::Overdue1Day,
            -3 => ReminderType::Overdue3Days,
            -7 => ReminderType::Overdue7Days,
            -14 => ReminderType::Overdue14Days,
            _ => ReminderType::Custom,
        }
    }

    pub async fn get_pending_reminders(&self) -> Vec<Reminder> {
        let reminders = self.reminders.read().await;
        reminders
            .iter()
            .filter(|r| r.status == ReminderStatus::Scheduled && r.scheduled_at <= Utc::now())
            .cloned()
            .collect()
    }

    pub async fn mark_sent(&self, reminder_id: &str) -> anyhow::Result<()> {
        let mut reminders = self.reminders.write().await;
        if let Some(reminder) = reminders.iter_mut().find(|r| r.id == reminder_id) {
            reminder.status = ReminderStatus::Sent;
            reminder.sent_at = Some(Utc::now());
        }
        Ok(())
    }

    pub fn vn_reminder_templates(reminder_type: ReminderType) -> &'static str {
        match reminder_type {
            ReminderType::DueIn7Days => "Nhắc nhở: Hóa đơn sẽ đến hạn trong 7 ngày. Vui lòng chuẩn bị thanh toán.",
            ReminderType::DueIn3Days => "Nhắc nhở: Hóa đơn sẽ đến hạn trong 3 ngày. Hãy thanh toán sớm để tránh phí.",
            ReminderType::DueToday => "Hôm nay là ngày đến hạn thanh toán. Vui lòng thanh toán hóa đơn.",
            ReminderType::Overdue1Day => "Hóa đơn đã quá hạn 1 ngày. Vui lòng thanh toán sớm nhất có thể.",
            ReminderType::Overdue3Days => "Hóa đơn đã quá hạn 3 ngày. Vui lòng thanh toán ngay để tránh bị ngừng cung cấp dịch vụ.",
            ReminderType::Overdue7Days => "Hóa đơn đã quá hạn 7 ngày. Chúng tôi sẽ liên hệ nếu không nhận được thanh toán.",
            ReminderType::Overdue14Days => "Hóa đơn đã quá hạn 14 ngày. Tài khoản có thể bị tạm khóa.",
            ReminderType::Custom => "Nhắc nhở thanh toán hóa đơn.",
        }
    }
}

impl Default for ReminderScheduler {
    fn default() -> Self {
        Self::new()
    }
}
