use crate::types::{Content, ContentCampaign, ContentPlatform, ContentStatus};
use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Duration, Utc, Weekday};
use std::collections::HashMap;
use tokio::sync::mpsc;

pub struct ContentScheduler {
    content_store: HashMap<String, Content>,
    campaigns: HashMap<String, ContentCampaign>,
    pending_queue: Vec<ScheduledItem>,
    sender: Option<mpsc::Sender<Content>>,
}

#[derive(Debug, Clone)]
struct ScheduledItem {
    content_id: String,
    scheduled_at: DateTime<Utc>,
    platform: ContentPlatform,
}

impl ContentScheduler {
    pub fn new() -> Self {
        Self {
            content_store: HashMap::new(),
            campaigns: HashMap::new(),
            pending_queue: Vec::new(),
            sender: None,
        }
    }

    pub fn with_publisher(channel: mpsc::Sender<Content>) -> Self {
        Self {
            content_store: HashMap::new(),
            campaigns: HashMap::new(),
            pending_queue: Vec::new(),
            sender: Some(channel),
        }
    }

    pub fn add_content(&mut self, content: Content) -> Result<String> {
        let id = content.id.clone();
        self.content_store.insert(id.clone(), content);
        Ok(id)
    }

    pub fn schedule_content(
        &mut self,
        content_id: &str,
        scheduled_at: DateTime<Utc>,
    ) -> Result<()> {
        let content = self
            .content_store
            .get_mut(content_id)
            .context("Content not found")?;

        content.scheduled_at = Some(scheduled_at);
        content.status = ContentStatus::Scheduled;

        self.pending_queue.push(ScheduledItem {
            content_id: content_id.to_string(),
            scheduled_at,
            platform: content.platform.clone(),
        });

        self.pending_queue.sort_by_key(|item| item.scheduled_at);

        Ok(())
    }

    pub fn publish_now(&mut self, content_id: &str) -> Result<Content> {
        let content = self
            .content_store
            .get_mut(content_id)
            .context("Content not found")?;

        content.status = ContentStatus::Published;

        if let Some(sender) = &self.sender {
            let _ = sender.try_send(content.clone());
        }

        Ok(content.clone())
    }

    pub fn get_content(&self, content_id: &str) -> Option<&Content> {
        self.content_store.get(content_id)
    }

    pub fn list_content(
        &self,
        platform: Option<&ContentPlatform>,
        status: Option<&ContentStatus>,
    ) -> Vec<&Content> {
        self.content_store
            .values()
            .filter(|c| {
                let platform_match = platform.map_or(true, |p| &c.platform == p);
                let status_match = status.map_or(true, |s| &c.status == s);
                platform_match && status_match
            })
            .collect()
    }

    pub fn create_campaign(&mut self, campaign: ContentCampaign) -> Result<String> {
        let id = campaign.id.clone();
        self.campaigns.insert(id.clone(), campaign);
        Ok(id)
    }

    pub fn get_due_content(&self, now: DateTime<Utc>) -> Vec<&Content> {
        self.pending_queue
            .iter()
            .filter(|item| item.scheduled_at <= now)
            .filter_map(|item| self.content_store.get(&item.content_id))
            .collect()
    }

    pub fn remove_from_queue(&mut self, content_id: &str) {
        self.pending_queue
            .retain(|item| item.content_id != content_id);
    }

    pub fn get_optimal_times(&self, platform: &ContentPlatform) -> Vec<DateTime<Utc>> {
        let now = Utc::now();
        let optimal_hours = self.get_platform_optimal_hours(platform);
        let mut times = Vec::new();

        for day_offset in 0..7 {
            let base_date = now.naive_utc().date() + Duration::days(day_offset);
            let weekday = base_date.weekday();

            if !self.is_rest_day(weekday) {
                for &hour in &optimal_hours {
                    if let Some(naive) = base_date.and_hms_opt(hour, 0, 0) {
                        let utc_dt = DateTime::from_naive_utc_and_offset(naive, Utc);
                        if utc_dt > now {
                            times.push(utc_dt);
                        }
                    }
                }
            }
        }

        times.truncate(14);
        times
    }

    fn get_platform_optimal_hours(&self, platform: &ContentPlatform) -> Vec<u32> {
        match platform {
            ContentPlatform::Facebook => vec![9, 12, 19, 20, 21],
            ContentPlatform::Zalo => vec![8, 11, 17, 18],
            ContentPlatform::TikTok => vec![7, 12, 18, 20, 22],
            ContentPlatform::Shopee => vec![9, 10, 14, 20],
            ContentPlatform::Website => vec![10, 14, 16],
            ContentPlatform::Email => vec![9, 10, 11],
        }
    }

    fn is_rest_day(&self, weekday: Weekday) -> bool {
        matches!(weekday, Weekday::Sat | Weekday::Sun)
    }

    pub fn cancel_scheduled(&mut self, content_id: &str) -> Result<()> {
        let content = self
            .content_store
            .get_mut(content_id)
            .context("Content not found")?;

        if content.status != ContentStatus::Scheduled {
            anyhow::bail!("Content is not in scheduled status");
        }

        content.scheduled_at = None;
        content.status = ContentStatus::Draft;

        self.remove_from_queue(content_id);

        Ok(())
    }

    pub fn reschedule(&mut self, content_id: &str, new_time: DateTime<Utc>) -> Result<()> {
        self.remove_from_queue(content_id);
        self.schedule_content(content_id, new_time)
    }
}

impl Default for ContentScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ContentType;

    fn create_test_content() -> Content {
        Content {
            id: "test-1".to_string(),
            title: "Test Post".to_string(),
            body: "Test content body".to_string(),
            platform: ContentPlatform::Facebook,
            content_type: ContentType::Post,
            media: vec![],
            hashtags: vec![],
            created_at: Utc::now(),
            scheduled_at: None,
            status: ContentStatus::Draft,
        }
    }

    #[test]
    fn test_add_and_get_content() {
        let mut scheduler = ContentScheduler::new();
        let content = create_test_content();
        let id = scheduler.add_content(content).unwrap();
        assert_eq!(id, "test-1");
        assert!(scheduler.get_content("test-1").is_some());
    }

    #[test]
    fn test_schedule_content() {
        let mut scheduler = ContentScheduler::new();
        let content = create_test_content();
        scheduler.add_content(content).unwrap();

        let future_time = Utc::now() + Duration::hours(1);
        scheduler.schedule_content("test-1", future_time).unwrap();

        let content = scheduler.get_content("test-1").unwrap();
        assert_eq!(content.status, ContentStatus::Scheduled);
        assert!(content.scheduled_at.is_some());
    }

    #[test]
    fn test_get_optimal_times_facebook() {
        let scheduler = ContentScheduler::new();
        let times = scheduler.get_optimal_times(&ContentPlatform::Facebook);
        assert!(!times.is_empty());
        assert!(times.len() <= 14);
    }
}
