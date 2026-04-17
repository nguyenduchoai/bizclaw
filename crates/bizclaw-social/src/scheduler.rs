use crate::types::*;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub struct SocialScheduler {
    posts: Arc<RwLock<HashMap<String, ScheduledPost>>>,
    pending_posts: Arc<RwLock<Vec<String>>>,
    #[allow(dead_code)]
    tx: mpsc::Sender<ScheduledPost>,
}

impl SocialScheduler {
    pub fn new() -> Self {
        let (tx, _rx) = mpsc::channel(100);

        Self {
            posts: Arc::new(RwLock::new(HashMap::new())),
            pending_posts: Arc::new(RwLock::new(Vec::new())),
            tx,
        }
    }

    pub fn with_channel(tx: mpsc::Sender<ScheduledPost>) -> Self {
        Self {
            posts: Arc::new(RwLock::new(HashMap::new())),
            pending_posts: Arc::new(RwLock::new(Vec::new())),
            tx,
        }
    }

    pub fn schedule(&self, post: ScheduledPost) -> Result<String> {
        let post_id = post.id.clone();

        {
            let mut posts = self.posts.write();
            posts.insert(post_id.clone(), post.clone());
        }

        {
            let mut pending = self.pending_posts.write();
            pending.push(post_id.clone());
        }

        info!("Scheduled post: {}", post_id);
        Ok(post_id)
    }

    pub fn cancel(&self, post_id: &str) -> Result<()> {
        {
            let mut posts = self.posts.write();
            if posts.remove(post_id).is_none() {
                anyhow::bail!("Post not found: {}", post_id);
            }
        }

        {
            let mut pending = self.pending_posts.write();
            pending.retain(|id| id != post_id);
        }

        info!("Cancelled post: {}", post_id);
        Ok(())
    }

    pub fn get_pending(&self) -> Vec<ScheduledPost> {
        let posts = self.posts.read();
        let pending = self.pending_posts.read();

        pending
            .iter()
            .filter_map(|id| posts.get(id).cloned())
            .collect()
    }

    pub fn get_post(&self, post_id: &str) -> Option<ScheduledPost> {
        let posts = self.posts.read();
        posts.get(post_id).cloned()
    }

    pub fn get_posts_by_platform(&self, platform: Platform) -> Vec<ScheduledPost> {
        let posts = self.posts.read();
        posts
            .values()
            .filter(|p| p.platform == platform)
            .cloned()
            .collect()
    }

    pub fn get_due_posts(&self, before: DateTime<Utc>) -> Vec<ScheduledPost> {
        let posts = self.posts.read();
        let pending = self.pending_posts.read();

        pending
            .iter()
            .filter_map(|id| {
                let post = posts.get(id)?;
                if post.scheduled_at <= before {
                    Some(post.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn mark_publishing(&self, post_id: &str) -> Result<()> {
        let mut posts = self.posts.write();

        if let Some(post) = posts.get_mut(post_id) {
            post.status = PostStatus::Publishing;
            post.updated_at = Utc::now();
            Ok(())
        } else {
            anyhow::bail!("Post not found: {}", post_id)
        }
    }

    pub fn mark_published(&self, post_id: &str) -> Result<()> {
        let mut posts = self.posts.write();

        if let Some(post) = posts.get_mut(post_id) {
            post.publish();
            Ok(())
        } else {
            anyhow::bail!("Post not found: {}", post_id)
        }
    }

    pub fn mark_failed(&self, post_id: &str, error: String) -> Result<()> {
        let mut posts = self.posts.write();

        if let Some(post) = posts.get_mut(post_id) {
            post.fail(error);
            Ok(())
        } else {
            anyhow::bail!("Post not found: {}", post_id)
        }
    }

    pub fn remove_from_pending(&self, post_id: &str) {
        let mut pending = self.pending_posts.write();
        pending.retain(|id| id != post_id);
    }

    pub fn list_all(&self) -> Vec<ScheduledPost> {
        let posts = self.posts.read();
        posts.values().cloned().collect()
    }

    pub fn clear_completed(&self, older_than: DateTime<Utc>) {
        let mut posts = self.posts.write();
        posts.retain(|_, post| {
            if let Some(published_at) = post.published_at {
                published_at > older_than
            } else {
                true
            }
        });
    }
}

impl Default for SocialScheduler {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CronScheduler {
    scheduler: SocialScheduler,
    interval_seconds: u64,
}

impl CronScheduler {
    pub fn new(interval_seconds: u64) -> Self {
        Self {
            scheduler: SocialScheduler::new(),
            interval_seconds,
        }
    }

    pub fn start<F>(&self, mut executor: F)
    where
        F: FnMut(ScheduledPost) -> tokio::task::JoinHandle<Result<()>> + Send + 'static,
    {
        let scheduler = self.scheduler.clone();
        let interval_secs = self.interval_seconds;

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));

            loop {
                interval.tick().await;

                let now = Utc::now();
                let due_posts = scheduler.get_due_posts(now);

                for post in due_posts {
                    let post_id = post.id.clone();

                    if let Err(e) = scheduler.mark_publishing(&post_id) {
                        error!("Failed to mark post as publishing: {}", e);
                        continue;
                    }

                    let handle = executor(post.clone());

                    let scheduler_clone = scheduler.clone();
                    tokio::spawn(async move {
                        match handle.await {
                            Ok(Ok(())) => {
                                if let Err(e) = scheduler_clone.mark_published(&post_id) {
                                    error!("Failed to mark post as published: {}", e);
                                }
                            }
                            Ok(Err(e)) => {
                                if let Err(e) = scheduler_clone.mark_failed(&post_id, e.to_string())
                                {
                                    error!("Failed to mark post as failed: {}", e);
                                }
                            }
                            Err(e) => {
                                if let Err(e) = scheduler_clone.mark_failed(&post_id, e.to_string())
                                {
                                    error!("Failed to mark post as failed: {}", e);
                                }
                            }
                        }

                        scheduler_clone.remove_from_pending(&post_id);
                    });
                }
            }
        });
    }
}

#[derive(Debug, Clone)]
pub struct SchedulerHandle {
    scheduler: Arc<SocialScheduler>,
}

impl SchedulerHandle {
    pub fn new(scheduler: SocialScheduler) -> Self {
        Self {
            scheduler: Arc::new(scheduler),
        }
    }

    pub fn schedule(
        &self,
        platform: Platform,
        content: SocialContent,
        scheduled_at: DateTime<Utc>,
    ) -> Result<String> {
        let post = ScheduledPost::new(platform, content, scheduled_at);
        self.scheduler.schedule(post)
    }

    pub fn cancel(&self, post_id: &str) -> Result<()> {
        self.scheduler.cancel(post_id)
    }

    pub fn get_pending(&self) -> Vec<ScheduledPost> {
        self.scheduler.get_pending()
    }

    pub fn get_due_posts(&self) -> Vec<ScheduledPost> {
        self.scheduler.get_due_posts(Utc::now())
    }

    pub fn list_all(&self) -> Vec<ScheduledPost> {
        self.scheduler.list_all()
    }
}

pub struct RecurringSchedule {
    pub interval: Duration,
    pub platforms: Vec<Platform>,
    pub content_template: String,
    pub hashtags: Vec<String>,
}

impl RecurringSchedule {
    pub fn new(interval_hours: i64) -> Self {
        Self {
            interval: Duration::hours(interval_hours),
            platforms: Vec::new(),
            content_template: String::new(),
            hashtags: Vec::new(),
        }
    }

    pub fn platforms(mut self, platforms: Vec<Platform>) -> Self {
        self.platforms = platforms;
        self
    }

    pub fn content_template(mut self, template: &str) -> Self {
        self.content_template = template.to_string();
        self
    }

    pub fn hashtags(mut self, tags: Vec<&str>) -> Self {
        self.hashtags = tags.into_iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn next_run(&self, from: DateTime<Utc>) -> DateTime<Utc> {
        from + self.interval
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_post() {
        let scheduler = SocialScheduler::new();

        let content = SocialContent::builder()
            .text("Test post")
            .platform(Platform::ZaloOA)
            .build();

        let post = ScheduledPost::new(Platform::ZaloOA, content, Utc::now() + Duration::hours(1));

        let result = scheduler.schedule(post);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cancel_post() {
        let scheduler = SocialScheduler::new();

        let content = SocialContent::builder()
            .text("Test post")
            .platform(Platform::TikTok)
            .build();

        let post = ScheduledPost::new(Platform::TikTok, content, Utc::now() + Duration::hours(1));

        let post_id = scheduler.schedule(post).unwrap();

        let result = scheduler.cancel(&post_id);
        assert!(result.is_ok());

        assert!(scheduler.get_post(&post_id).is_none());
    }

    #[test]
    fn test_get_due_posts() {
        let scheduler = SocialScheduler::new();

        let content = SocialContent::builder()
            .text("Past post")
            .platform(Platform::Facebook)
            .build();

        let past_post =
            ScheduledPost::new(Platform::Facebook, content, Utc::now() - Duration::hours(1));

        scheduler.schedule(past_post).unwrap();

        let content2 = SocialContent::builder()
            .text("Future post")
            .platform(Platform::Instagram)
            .build();

        let future_post = ScheduledPost::new(
            Platform::Instagram,
            content2,
            Utc::now() + Duration::hours(1),
        );

        scheduler.schedule(future_post).unwrap();

        let due_posts = scheduler.get_due_posts(Utc::now());
        assert_eq!(due_posts.len(), 1);
        assert_eq!(due_posts[0].platform, Platform::Facebook);
    }

    #[test]
    fn test_recurring_schedule() {
        let schedule = RecurringSchedule::new(24)
            .platforms(vec![Platform::ZaloOA, Platform::TikTok])
            .content_template("Daily update!")
            .hashtags(vec!["bizclaw", "ai"]);

        assert_eq!(schedule.platforms.len(), 2);
        assert_eq!(schedule.hashtags.len(), 2);

        let now = Utc::now();
        let next = schedule.next_run(now);
        assert!(next > now);
    }
}
