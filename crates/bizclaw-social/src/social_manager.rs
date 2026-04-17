//! # Social Media Manager - Unified Multi-Platform Integration
//!
//! Integrates all social platforms from BrightBean Studio:
//! - Facebook, Instagram, LinkedIn, TikTok, YouTube, Pinterest, Threads, Bluesky, Google Business Profile
//!
//! Features per platform:
//! | Platform | Publish | Comments | DMs | Insights |
//! |----------|---------|----------|-----|----------|
//! | Facebook | ✅ | ✅ | ✅ | ✅ |
//! | Instagram | ✅ | ✅ | ✅ | ✅ |
//! | Instagram Personal | ✅ | ✅ | ✅ | ✅ |
//! | LinkedIn Personal | ✅ | ✅ | - | ✅ |
//! | LinkedIn Company | ✅ | ✅ | - | ✅ |
//! | TikTok | ✅ | ✅ | - | ✅ |
//! | YouTube | ✅ | ✅ | - | ✅ |
//! | Pinterest | ✅ | - | - | ✅ |
//! | Threads | ✅ | ✅ | - | ✅ |
//! | Bluesky | ✅ | ✅ | - | ✅ |
//! | Google Business | ✅ | - | - | ✅ |

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformCredentials {
    pub platform: String,
    pub client_id: String,
    pub client_secret: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub account_id: String,
    pub account_name: String,
    pub account_type: AccountType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    Personal,
    Business,
    Creator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostContent {
    pub text: String,
    pub media_urls: Vec<String>,
    pub link_url: Option<String>,
    pub link_preview_text: Option<String>,
    pub scheduled_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostResponse {
    pub post_id: String,
    pub platform: String,
    pub permalink: String,
    pub posted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub post_id: String,
    pub author_id: String,
    pub author_name: String,
    pub text: String,
    pub created_at: DateTime<Utc>,
    pub likes: u64,
    pub replies: Vec<Comment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectMessage {
    pub id: String,
    pub thread_id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub recipient_id: String,
    pub text: String,
    pub created_at: DateTime<Utc>,
    pub attachments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insights {
    pub impressions: u64,
    pub reach: u64,
    pub engagements: u64,
    pub likes: u64,
    pub comments: u64,
    pub shares: u64,
    pub saves: u64,
    pub clicks: u64,
    pub followers: Option<u64>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

pub struct SocialMediaManager {
    credentials: Arc<RwLock<HashMap<String, PlatformCredentials>>>,
    client: Client,
}

impl SocialMediaManager {
    pub fn new() -> Self {
        Self {
            credentials: Arc::new(RwLock::new(HashMap::new())),
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }

    pub async fn register_account(&self, creds: PlatformCredentials) -> Result<()> {
        let key = format!("{}:{}", creds.platform, creds.account_id);
        let platform = creds.platform.clone();
        let account_name = creds.account_name.clone();
        let mut credentials = self.credentials.write().await;
        credentials.insert(key, creds);
        info!("Registered {} account: {}", platform, account_name);
        Ok(())
    }

    pub async fn get_credentials(
        &self,
        platform: &str,
        account_id: &str,
    ) -> Option<PlatformCredentials> {
        let credentials = self.credentials.read().await;
        let key = format!("{}:{}", platform, account_id);
        credentials.get(&key).cloned()
    }

    pub async fn post(
        &self,
        platform: &str,
        account_id: &str,
        content: PostContent,
    ) -> Result<PostResponse> {
        match platform {
            "facebook" => self.post_facebook(account_id, content).await,
            "instagram" | "instagram_personal" => self.post_instagram(account_id, content).await,
            "linkedin_personal" | "linkedin_company" => {
                self.post_linkedin(platform, account_id, content).await
            }
            "tiktok" => self.post_tiktok(account_id, content).await,
            "youtube" => self.post_youtube(account_id, content).await,
            "pinterest" => self.post_pinterest(account_id, content).await,
            "threads" => self.post_threads(account_id, content).await,
            "bluesky" => self.post_bluesky(account_id, content).await,
            "google_business" => self.post_google_business(account_id, content).await,
            _ => anyhow::bail!("Unsupported platform: {}", platform),
        }
    }

    async fn post_facebook(&self, account_id: &str, content: PostContent) -> Result<PostResponse> {
        let creds = self
            .get_credentials("facebook", account_id)
            .await
            .context("Facebook account not found")?;

        let access_token = creds.access_token.clone();
        let mut params = vec![
            ("message", content.text.as_str()),
            ("access_token", access_token.as_str()),
        ];

        if let Some(link) = &content.link_url {
            params.push(("link", link.as_str()));
        }

        let url = format!("https://graph.facebook.com/v18.0/{}/feed", account_id);
        let response = self.client.post(&url).form(&params).send().await?;

        #[derive(Deserialize)]
        struct FbPostResponse {
            id: String,
        }

        let result: FbPostResponse = response.json().await?;
        let post_id = result.id.clone();

        Ok(PostResponse {
            post_id: post_id.clone(),
            platform: "facebook".to_string(),
            permalink: format!("https://facebook.com/{}", post_id),
            posted_at: Utc::now(),
        })
    }

    async fn post_instagram(&self, account_id: &str, content: PostContent) -> Result<PostResponse> {
        let creds = self
            .get_credentials("instagram", account_id)
            .await
            .context("Instagram account not found")?;

        let access_token = creds.access_token.clone();
        let is_video = content
            .media_urls
            .first()
            .map(|u| u.ends_with(".mp4"))
            .unwrap_or(false);

        let media_url = content.media_urls.first().cloned().unwrap_or_default();
        let create_url = format!("https://graph.facebook.com/v18.0/{}/media", account_id);

        let mut params = vec![
            ("caption", content.text.as_str()),
            ("access_token", access_token.as_str()),
        ];

        if is_video {
            params.push(("media_type", "REELS"));
            params.push(("video_url", media_url.as_str()));
        } else if !media_url.is_empty() {
            params.push(("image_url", media_url.as_str()));
        }

        let container_resp = self.client.post(&create_url).form(&params).send().await?;
        #[derive(Deserialize)]
        struct IgResponse {
            id: String,
        }
        let container: IgResponse = container_resp.json().await?;
        let creation_id = container.id.clone();

        let publish_url = format!(
            "https://graph.facebook.com/v18.0/{}/media_publish",
            account_id
        );
        let publish_params = vec![
            ("creation_id", creation_id.as_str()),
            ("access_token", access_token.as_str()),
        ];
        let publish_resp = self
            .client
            .post(&publish_url)
            .form(&publish_params)
            .send()
            .await?;
        let publish_result: IgResponse = publish_resp.json().await?;
        let post_id = publish_result.id.clone();

        Ok(PostResponse {
            post_id: post_id.clone(),
            platform: "instagram".to_string(),
            permalink: format!("https://instagram.com/p/{}", post_id),
            posted_at: Utc::now(),
        })
    }

    async fn post_linkedin(
        &self,
        platform: &str,
        account_id: &str,
        content: PostContent,
    ) -> Result<PostResponse> {
        let creds = self
            .get_credentials(platform, account_id)
            .await
            .context("LinkedIn account not found")?;

        let urn = if platform == "linkedin_company" {
            format!("urn:li:company:{}", account_id)
        } else {
            format!("urn:li:person:{}", account_id)
        };

        let post_body = serde_json::json!({
            "author": urn,
            "lifecycleState": "PUBLISHED",
            "specificContent": {
                "com.linkedin.ugc.ShareContent": {
                    "shareCommentary": { "text": content.text },
                    "shareMediaCategory": if content.media_urls.is_empty() { "NONE" } else { "IMAGE" },
                    "media": content.media_urls.iter().map(|u| {
                        serde_json::json!({ "media": u })
                    }).collect::<Vec<_>>()
                }
            },
            "visibility": {
                "com.linkedin.ugc.MemberNetworkVisibility": if platform == "linkedin_company" { "PUBLIC" } else { "CONNECTIONS" }
            }
        });

        let url = "https://api.linkedin.com/v2/ugcPosts";
        let access_token = creds.access_token.clone();
        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .header("Content-Type", "application/json")
            .json(&post_body)
            .send()
            .await?;

        #[derive(Deserialize)]
        struct LinkedInResponse {
            id: String,
        }
        let result: LinkedInResponse = response.json().await?;
        let post_id = result.id.clone();

        Ok(PostResponse {
            post_id: post_id.clone(),
            platform: platform.to_string(),
            permalink: format!("https://linkedin.com/feed/update/{}", post_id),
            posted_at: Utc::now(),
        })
    }

    async fn post_tiktok(&self, account_id: &str, content: PostContent) -> Result<PostResponse> {
        let creds = self
            .get_credentials("tiktok", account_id)
            .await
            .context("TikTok account not found")?;

        let video_url = content.media_urls.first().cloned().unwrap_or_default();
        if video_url.is_empty() {
            anyhow::bail!("TikTok requires video URL");
        }

        let url = "https://open.tiktokapis.com/v2/post/publish/";
        let access_token = creds.access_token.clone();
        let body = serde_json::json!({
            "post_info": {
                "title": content.text.chars().take(100).collect::<String>(),
                "description": content.text,
                "privacy_level": "SITE_PUBLIC",
                "disable_comment": false,
                "disable_share": false,
            },
            "source_info": {
                "source": "PULL_FROM_URL",
                "video_url": video_url,
            }
        });

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        #[derive(Deserialize)]
        struct TikTokResponse {
            post_id: String,
        }
        let result: TikTokResponse = response.json().await?;
        let post_id = result.post_id.clone();

        Ok(PostResponse {
            post_id: post_id.clone(),
            platform: "tiktok".to_string(),
            permalink: format!("https://tiktok.com/@{}/video/{}", account_id, post_id),
            posted_at: Utc::now(),
        })
    }

    async fn post_youtube(&self, account_id: &str, content: PostContent) -> Result<PostResponse> {
        let creds = self
            .get_credentials("youtube", account_id)
            .await
            .context("YouTube account not found")?;

        let video_url = content.media_urls.first().cloned().unwrap_or_default();
        if video_url.is_empty() {
            anyhow::bail!("YouTube requires video URL");
        }

        let url = "https://www.googleapis.com/upload/youtube/v3/videos";
        let access_token = creds.access_token.clone();
        let snippet = serde_json::json!({
            "snippet": {
                "title": content.text.chars().take(90).collect::<String>(),
                "description": content.text,
                "tags": ["bizclaw"],
                "categoryId": "22"
            },
            "status": { "privacyStatus": "public" }
        });

        let response = self
            .client
            .post(url)
            .bearer_auth(access_token)
            .json(&snippet)
            .send()
            .await?;

        #[derive(Deserialize)]
        struct YTResponse {
            id: String,
        }
        let result: YTResponse = response.json().await?;
        let post_id = result.id.clone();

        Ok(PostResponse {
            post_id: post_id.clone(),
            platform: "youtube".to_string(),
            permalink: format!("https://youtube.com/watch?v={}", post_id),
            posted_at: Utc::now(),
        })
    }

    async fn post_pinterest(&self, account_id: &str, content: PostContent) -> Result<PostResponse> {
        let creds = self
            .get_credentials("pinterest", account_id)
            .await
            .context("Pinterest account not found")?;

        let image_url = content.media_urls.first().cloned().unwrap_or_default();
        let link_url = content.link_url.unwrap_or_default();

        let url = format!("https://api.pinterest.com/v5/pins");
        let access_token = creds.access_token.clone();
        let body = serde_json::json!({
            "board_id": account_id,
            "link": link_url,
            "description": content.text,
            "image_url": image_url,
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        #[derive(Deserialize)]
        struct PinterestResponse {
            id: String,
        }
        let result: PinterestResponse = response.json().await?;
        let post_id = result.id.clone();

        Ok(PostResponse {
            post_id: post_id.clone(),
            platform: "pinterest".to_string(),
            permalink: format!("https://pinterest.com/pin/{}", post_id),
            posted_at: Utc::now(),
        })
    }

    async fn post_threads(&self, account_id: &str, content: PostContent) -> Result<PostResponse> {
        let creds = self
            .get_credentials("threads", account_id)
            .await
            .context("Threads account not found")?;

        let url = format!("https://graph.facebook.com/v18.0/{}/threads", account_id);
        let access_token = creds.access_token.clone();
        let body = serde_json::json!({
            "message": content.text,
            "access_token": access_token,
        });

        let response = self.client.post(&url).json(&body).send().await?;

        #[derive(Deserialize)]
        struct ThreadsResponse {
            id: String,
        }
        let result: ThreadsResponse = response.json().await?;
        let post_id = result.id.clone();

        Ok(PostResponse {
            post_id: post_id.clone(),
            platform: "threads".to_string(),
            permalink: format!("https://threads.net/@{}/post/{}", account_id, post_id),
            posted_at: Utc::now(),
        })
    }

    async fn post_bluesky(&self, account_id: &str, content: PostContent) -> Result<PostResponse> {
        let creds = self
            .get_credentials("bluesky", account_id)
            .await
            .context("Bluesky account not found")?;

        let did = format!("did:plc:{}", account_id);
        let url = format!("https://bsky.social/xrpc/com.atproto.repo.createRecord");
        let access_token = creds.access_token.clone();
        let record = serde_json::json!({
            "repo": did,
            "collection": "app.bsky.feed.post",
            "record": {
                "$type": "app.bsky.feed.post",
                "text": content.text,
                "createdAt": Utc::now().to_rfc3339(),
            }
        });

        let response = self
            .client
            .post(&url)
            .bearer_auth(access_token)
            .json(&record)
            .send()
            .await?;

        #[derive(Deserialize)]
        struct BlueskyResponse {
            uri: String,
        }
        let result: BlueskyResponse = response.json().await?;
        let post_id = result.uri.split("/").last().unwrap_or("").to_string();

        let uri = result.uri.clone();
        let post_id = uri.split("/").last().unwrap_or("").to_string();

        Ok(PostResponse {
            post_id: post_id.clone(),
            platform: "bluesky".to_string(),
            permalink: format!("https://bsky.app/profile/{}/post/{}", did, post_id),
            posted_at: Utc::now(),
        })
    }

    async fn post_google_business(
        &self,
        account_id: &str,
        content: PostContent,
    ) -> Result<PostResponse> {
        let creds = self
            .get_credentials("google_business", account_id)
            .await
            .context("Google Business account not found")?;

        let access_token = creds.access_token.clone();
        let url = format!(
            "https://mybusiness.googleapis.com/v4/accounts/{}/locations/{}/localPosts",
            account_id.split(":").next().unwrap_or(account_id),
            account_id
        );
        let body = serde_json::json!({
            "languageCode": "vi",
            "summary": content.text,
            "callToAction": {
                "actionType": "LEARN_MORE",
                "url": content.link_url.unwrap_or_default()
            }
        });

        let response = self
            .client
            .post(&url)
            .bearer_auth(access_token)
            .json(&body)
            .send()
            .await?;

        #[derive(Deserialize)]
        struct GBResponse {
            name: String,
        }
        let result: GBResponse = response.json().await?;
        let post_id = result.name.clone();

        Ok(PostResponse {
            post_id: post_id.clone(),
            platform: "google_business".to_string(),
            permalink: format!("https://business.google.com/posts/{}", post_id),
            posted_at: Utc::now(),
        })
    }

    pub async fn get_comments(
        &self,
        platform: &str,
        account_id: &str,
        post_id: &str,
    ) -> Result<Vec<Comment>> {
        match platform {
            "facebook" => self.get_facebook_comments(account_id, post_id).await,
            "instagram" | "instagram_personal" => {
                self.get_instagram_comments(account_id, post_id).await
            }
            "linkedin_personal" | "linkedin_company" => {
                self.get_linkedin_comments(platform, account_id, post_id)
                    .await
            }
            "tiktok" => self.get_tiktok_comments(account_id, post_id).await,
            "youtube" => self.get_youtube_comments(account_id, post_id).await,
            "threads" => self.get_threads_comments(account_id, post_id).await,
            "bluesky" => self.get_bluesky_comments(account_id, post_id).await,
            _ => Ok(vec![]),
        }
    }

    async fn get_facebook_comments(
        &self,
        post_id: &str,
        _comment_id: &str,
    ) -> Result<Vec<Comment>> {
        let creds = match self
            .get_credentials("facebook", post_id.split('_').next().unwrap_or("").into())
            .await
        {
            Some(c) => c,
            None => return Ok(vec![]),
        };

        let url = format!(
            "https://graph.facebook.com/v18.0/{}/comments?access_token={}",
            post_id, creds.access_token
        );

        let response = self.client.get(&url).send().await?;
        #[derive(Deserialize)]
        struct FBCommentsResponse {
            data: Vec<FBCommentData>,
        }
        #[derive(Deserialize)]
        struct FBCommentData {
            id: String,
            from: FBFrom,
            message: String,
            created_time: String,
            like_count: Option<u64>,
        }
        #[derive(Deserialize)]
        struct FBFrom {
            id: String,
            name: String,
        }

        let result: Result<FBCommentsResponse, _> = response.json().await;
        if let Ok(data) = result {
            let comments: Vec<Comment> = data
                .data
                .into_iter()
                .map(|c| {
                    let timestamp = chrono::DateTime::parse_from_rfc3339(&c.created_time)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());
                    Comment {
                        id: c.id,
                        post_id: post_id.to_string(),
                        author_id: c.from.id,
                        author_name: c.from.name,
                        text: c.message,
                        created_at: timestamp,
                        likes: c.like_count.unwrap_or(0),
                        replies: vec![],
                    }
                })
                .collect();
            return Ok(comments);
        }
        Ok(vec![])
    }

    async fn get_instagram_comments(
        &self,
        post_id: &str,
        _comment_id: &str,
    ) -> Result<Vec<Comment>> {
        let creds = match self.get_credentials("instagram", "").await {
            Some(c) => c,
            None => return Ok(vec![]),
        };

        let url = format!(
            "https://graph.facebook.com/v18.0/{}/comments?access_token={}",
            post_id, creds.access_token
        );

        let response = self.client.get(&url).send().await?;
        #[derive(Deserialize)]
        struct IGCommentsResponse {
            data: Vec<IGCommentData>,
        }
        #[derive(Deserialize)]
        struct IGCommentData {
            id: String,
            from: IGFrom,
            text: String,
            timestamp: String,
            like_count: Option<u64>,
        }
        #[derive(Deserialize)]
        struct IGFrom {
            id: String,
            username: String,
        }

        let result: Result<IGCommentsResponse, _> = response.json().await;
        if let Ok(data) = result {
            let comments: Vec<Comment> = data
                .data
                .into_iter()
                .map(|c| {
                    let ts = chrono::DateTime::parse_from_rfc3339(&c.timestamp)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());
                    Comment {
                        id: c.id,
                        post_id: post_id.to_string(),
                        author_id: c.from.id,
                        author_name: c.from.username,
                        text: c.text,
                        created_at: ts,
                        likes: c.like_count.unwrap_or(0),
                        replies: vec![],
                    }
                })
                .collect();
            return Ok(comments);
        }
        Ok(vec![])
    }

    async fn get_linkedin_comments(
        &self,
        _platform: &str,
        _account_id: &str,
        post_id: &str,
    ) -> Result<Vec<Comment>> {
        let creds = match self.get_credentials("linkedin_company", "").await {
            Some(c) => c,
            None => return Ok(vec![]),
        };

        let url = format!("https://api.linkedin.com/v2/socialMetadata/{}", post_id);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", creds.access_token))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .send()
            .await;

        if let Ok(resp) = response {
            #[derive(Deserialize)]
            struct LICommentsResponse {
                results: std::collections::HashMap<String, LIComment>,
            }
            #[derive(Deserialize)]
            struct LIComment {
                id: String,
                content: String,
                author: String,
                created: LICommentTime,
            }
            #[derive(Deserialize)]
            struct LICommentTime {
                time: i64,
            }

            let result: Result<LICommentsResponse, _> = resp.json().await;
            if let Ok(data) = result {
                let comments: Vec<Comment> = data
                    .results
                    .into_iter()
                    .map(|(id, c)| {
                        let author = c.author.clone();
                        Comment {
                            id,
                            post_id: post_id.to_string(),
                            author_id: author.clone(),
                            author_name: author,
                            text: c.content,
                            created_at: chrono::DateTime::from_timestamp(c.created.time, 0)
                                .unwrap_or_else(Utc::now),
                            likes: 0,
                            replies: vec![],
                        }
                    })
                    .collect();
                return Ok(comments);
            }
        }
        Ok(vec![])
    }

    async fn get_tiktok_comments(&self, post_id: &str, _comment_id: &str) -> Result<Vec<Comment>> {
        let creds = match self.get_credentials("tiktok", "").await {
            Some(c) => c,
            None => return Ok(vec![]),
        };

        let url = "https://open.tiktokapis.com/v2/video/comments/?fields=id,text,create_time,like_count,user.id,user.username".to_string();

        let body = serde_json::json!({ "video_id": post_id });
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", creds.access_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await;

        if let Ok(resp) = response {
            #[derive(Deserialize)]
            struct TTCommentsResponse {
                data: TTCommentList,
            }
            #[derive(Deserialize)]
            struct TTCommentList {
                comments: Vec<TTComment>,
            }
            #[derive(Deserialize)]
            struct TTComment {
                id: String,
                text: String,
                create_time: i64,
                like_count: u64,
                user: TTUser,
            }
            #[derive(Deserialize)]
            struct TTUser {
                id: String,
                username: String,
            }

            let result: Result<TTCommentsResponse, _> = resp.json().await;
            if let Ok(data) = result {
                let comments: Vec<Comment> = data
                    .data
                    .comments
                    .into_iter()
                    .map(|c| Comment {
                        id: c.id,
                        post_id: post_id.to_string(),
                        author_id: c.user.id,
                        author_name: c.user.username,
                        text: c.text,
                        created_at: chrono::DateTime::from_timestamp(c.create_time, 0)
                            .unwrap_or_else(Utc::now),
                        likes: c.like_count,
                        replies: vec![],
                    })
                    .collect();
                return Ok(comments);
            }
        }
        Ok(vec![])
    }

    async fn get_youtube_comments(
        &self,
        video_id: &str,
        _comment_id: &str,
    ) -> Result<Vec<Comment>> {
        let creds = match self.get_credentials("youtube", "").await {
            Some(c) => c,
            None => return Ok(vec![]),
        };

        let url = format!(
            "https://www.googleapis.com/youtube/v3/commentThreads?part=snippet&videoId={}&key={}",
            video_id, creds.access_token
        );

        let response = self.client.get(&url).send().await;
        if let Ok(resp) = response {
            #[derive(Deserialize)]
            struct YTCommentsResponse {
                items: Vec<YTCommentThread>,
            }
            #[derive(Deserialize)]
            struct YTCommentThread {
                snippet: YTSnippet,
            }
            #[derive(Deserialize)]
            struct YTSnippet {
                topLevelComment: YTSnippet2,
                totalReplyCount: u64,
            }
            #[derive(Deserialize)]
            struct YTSnippet2 {
                id: String,
                snippet: YTCommentSnippet,
            }
            #[derive(Deserialize)]
            struct YTCommentSnippet {
                authorDisplayName: String,
                authorChannelId: String,
                textDisplay: String,
                publishedAt: String,
                likeCount: u64,
            }

            let result: Result<YTCommentsResponse, _> = resp.json().await;
            if let Ok(data) = result {
                let comments: Vec<Comment> = data
                    .items
                    .into_iter()
                    .map(|item| {
                        let snippet = &item.snippet.topLevelComment.snippet;
                        let ts = chrono::DateTime::parse_from_rfc3339(&snippet.publishedAt)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now());
                        Comment {
                            id: item.snippet.topLevelComment.id,
                            post_id: video_id.to_string(),
                            author_id: snippet.authorChannelId.clone(),
                            author_name: snippet.authorDisplayName.clone(),
                            text: snippet.textDisplay.clone(),
                            created_at: ts,
                            likes: snippet.likeCount,
                            replies: vec![],
                        }
                    })
                    .collect();
                return Ok(comments);
            }
        }
        Ok(vec![])
    }

    async fn get_threads_comments(&self, post_id: &str, _comment_id: &str) -> Result<Vec<Comment>> {
        let creds = match self.get_credentials("threads", "").await {
            Some(c) => c,
            None => return Ok(vec![]),
        };

        let url = format!("https://graph.facebook.com/v18.0/{}", post_id);
        let params = format!(
            "?fields=comments{{id,from,message,created_time}}&access_token={}",
            creds.access_token
        );

        let response = self.client.get(&format!("{}{}", url, params)).send().await;
        if let Ok(resp) = response {
            #[derive(Deserialize)]
            struct ThreadsCommentsResponse {
                comments: ThreadsCommentsData,
            }
            #[derive(Deserialize)]
            struct ThreadsCommentsData {
                data: Vec<ThreadsComment>,
            }
            #[derive(Deserialize)]
            struct ThreadsComment {
                id: String,
                from: ThreadsFrom,
                message: String,
                created_time: String,
            }
            #[derive(Deserialize)]
            struct ThreadsFrom {
                id: String,
                name: String,
            }

            let result: Result<ThreadsCommentsResponse, _> = resp.json().await;
            if let Ok(data) = result {
                let comments: Vec<Comment> = data
                    .comments
                    .data
                    .into_iter()
                    .map(|c| {
                        let ts = chrono::DateTime::parse_from_rfc3339(&c.created_time)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now());
                        Comment {
                            id: c.id,
                            post_id: post_id.to_string(),
                            author_id: c.from.id,
                            author_name: c.from.name,
                            text: c.message,
                            created_at: ts,
                            likes: 0,
                            replies: vec![],
                        }
                    })
                    .collect();
                return Ok(comments);
            }
        }
        Ok(vec![])
    }

    async fn get_bluesky_comments(
        &self,
        post_uri: &str,
        _comment_id: &str,
    ) -> Result<Vec<Comment>> {
        let creds = match self.get_credentials("bluesky", "").await {
            Some(c) => c,
            None => return Ok(vec![]),
        };

        let url = "https://bsky.social/xrpc/app.bsky.feed.getPostThread";
        let params = format!("?uri={}", post_uri);

        let response = self
            .client
            .get(&format!("{}{}", url, params))
            .bearer_auth(&creds.access_token)
            .send()
            .await;

        if let Ok(resp) = response {
            #[derive(Deserialize)]
            struct BSkyThreadResponse {
                thread: BSkyThread,
            }
            #[derive(Deserialize)]
            struct BSkyThread {
                post: BSkyPost,
            }
            #[derive(Deserialize)]
            struct BSkyPost {
                uri: String,
                author: BSkyAuthor,
                record: BSkyRecord,
                like_count: u64,
            }
            #[derive(Deserialize)]
            struct BSkyAuthor {
                did: String,
                displayName: String,
            }
            #[derive(Deserialize)]
            struct BSkyRecord {
                createdAt: String,
                text: String,
            }

            let result: Result<BSkyThreadResponse, _> = resp.json().await;
            if let Ok(data) = result {
                let p = &data.thread.post;
                let ts = chrono::DateTime::parse_from_rfc3339(&p.record.createdAt)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                return Ok(vec![Comment {
                    id: p.uri.split("/").last().unwrap_or("").to_string(),
                    post_id: post_uri.to_string(),
                    author_id: p.author.did.clone(),
                    author_name: p.author.displayName.clone(),
                    text: p.record.text.clone(),
                    created_at: ts,
                    likes: p.like_count,
                    replies: vec![],
                }]);
            }
        }
        Ok(vec![])
    }

    pub async fn get_dms(&self, platform: &str, account_id: &str) -> Result<Vec<DirectMessage>> {
        match platform {
            "facebook" => self.get_facebook_dms(account_id).await,
            "instagram" | "instagram_personal" => self.get_instagram_dms(account_id).await,
            _ => Ok(vec![]),
        }
    }

    async fn get_facebook_dms(&self, account_id: &str) -> Result<Vec<DirectMessage>> {
        let creds = match self.get_credentials("facebook", account_id).await {
            Some(c) => c,
            None => return Ok(vec![]),
        };

        let url = format!(
            "https://graph.facebook.com/v18.0/{}/conversations?access_token={}",
            account_id, creds.access_token
        );

        let response = self.client.get(&url).send().await;
        if let Ok(resp) = response {
            #[derive(Deserialize)]
            struct FBDMsResponse {
                data: Vec<FBDMData>,
            }
            #[derive(Deserialize)]
            struct FBDMData {
                id: String,
                updated_time: String,
                messages: FBMessagesData,
            }
            #[derive(Deserialize)]
            struct FBMessagesData {
                data: Vec<FBMessageData>,
            }
            #[derive(Deserialize)]
            struct FBMessageData {
                id: String,
                from: FBDMFrom,
                to: FBDMTo,
                message: String,
                created_time: String,
            }
            #[derive(Deserialize)]
            struct FBDMFrom {
                id: String,
                name: String,
            }
            #[derive(Deserialize)]
            struct FBDMTo {
                data: Vec<FBDMToUser>,
            }
            #[derive(Deserialize)]
            struct FBDMToUser {
                id: String,
            }

            let result: Result<FBDMsResponse, _> = resp.json().await;
            if let Ok(data) = result {
                let dms: Vec<DirectMessage> = data
                    .data
                    .into_iter()
                    .flat_map(|conv| {
                        conv.messages
                            .data
                            .into_iter()
                            .map(|msg| {
                                let ts = chrono::DateTime::parse_from_rfc3339(&msg.created_time)
                                    .map(|dt| dt.with_timezone(&Utc))
                                    .unwrap_or_else(|_| Utc::now());
                                DirectMessage {
                                    id: msg.id,
                                    thread_id: conv.id.clone(),
                                    sender_id: msg.from.id.clone(),
                                    sender_name: msg.from.name.clone(),
                                    recipient_id: msg
                                        .to
                                        .data
                                        .first()
                                        .map(|u| u.id.clone())
                                        .unwrap_or_default(),
                                    text: msg.message,
                                    created_at: ts,
                                    attachments: vec![],
                                }
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect();
                return Ok(dms);
            }
        }
        Ok(vec![])
    }

    async fn get_instagram_dms(&self, account_id: &str) -> Result<Vec<DirectMessage>> {
        let creds = match self.get_credentials("instagram", account_id).await {
            Some(c) => c,
            None => return Ok(vec![]),
        };

        let url = format!(
            "https://graph.facebook.com/v18.0/{}/conversations?access_token={}",
            account_id, creds.access_token
        );

        let response = self.client.get(&url).send().await;
        if let Ok(resp) = response {
            #[derive(Deserialize)]
            struct IGDMsResponse {
                data: Vec<IGDMData>,
            }
            #[derive(Deserialize)]
            struct IGDMData {
                id: String,
                updated_time: String,
                messages: IGMessagesData,
            }
            #[derive(Deserialize)]
            struct IGMessagesData {
                data: Vec<IGMessageData>,
            }
            #[derive(Deserialize)]
            struct IGMessageData {
                id: String,
                from: IGDMFrom,
                to: IGDMTo,
                text: Option<String>,
                created_time: String,
            }
            #[derive(Deserialize)]
            struct IGDMFrom {
                id: String,
                username: String,
            }
            #[derive(Deserialize)]
            struct IGDMTo {
                data: Vec<IGDMToUser>,
            }
            #[derive(Deserialize)]
            struct IGDMToUser {
                id: String,
            }

            let result: Result<IGDMsResponse, _> = resp.json().await;
            if let Ok(data) = result {
                let dms: Vec<DirectMessage> = data
                    .data
                    .into_iter()
                    .flat_map(|conv| {
                        conv.messages
                            .data
                            .into_iter()
                            .filter_map(|msg| {
                                let ts = chrono::DateTime::parse_from_rfc3339(&msg.created_time)
                                    .map(|dt| dt.with_timezone(&Utc))
                                    .unwrap_or_else(|_| Utc::now());
                                msg.text.map(|text| DirectMessage {
                                    id: msg.id,
                                    thread_id: conv.id.clone(),
                                    sender_id: msg.from.id.clone(),
                                    sender_name: msg.from.username.clone(),
                                    recipient_id: msg
                                        .to
                                        .data
                                        .first()
                                        .map(|u| u.id.clone())
                                        .unwrap_or_default(),
                                    text,
                                    created_at: ts,
                                    attachments: vec![],
                                })
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect();
                return Ok(dms);
            }
        }
        Ok(vec![])
    }

    pub async fn get_insights(
        &self,
        platform: &str,
        account_id: &str,
        post_id: &str,
    ) -> Result<Insights> {
        match platform {
            "facebook" => self.get_facebook_insights(account_id, post_id).await,
            "instagram" | "instagram_personal" => {
                self.get_instagram_insights(account_id, post_id).await
            }
            "linkedin_personal" | "linkedin_company" => {
                self.get_linkedin_insights(platform, account_id, post_id)
                    .await
            }
            "tiktok" => self.get_tiktok_insights(account_id, post_id).await,
            "youtube" => self.get_youtube_insights(account_id, post_id).await,
            "pinterest" => self.get_pinterest_insights(account_id, post_id).await,
            "threads" => self.get_threads_insights(account_id, post_id).await,
            "bluesky" => self.get_bluesky_insights(account_id, post_id).await,
            "google_business" => self.get_google_business_insights(account_id, post_id).await,
            _ => Ok(Insights::default()),
        }
    }

    async fn get_facebook_insights(&self, post_id: &str, _metric: &str) -> Result<Insights> {
        let creds = match self
            .get_credentials("facebook", post_id.split('_').next().unwrap_or("").into())
            .await
        {
            Some(c) => c,
            None => return Ok(Insights::default()),
        };

        let url = format!(
            "https://graph.facebook.com/v18.0/{}?fields=insights.metric(impressions,reach,engagements,reactions,comments,shares,saves,clicks)&access_token={}",
            post_id, creds.access_token
        );

        let response = self.client.get(&url).send().await;
        if let Ok(resp) = response {
            #[derive(Deserialize)]
            struct FBInsightsResponse {
                insights: FBInsightsData,
            }
            #[derive(Deserialize)]
            struct FBInsightsData {
                data: Vec<FBInsightMetric>,
            }
            #[derive(Deserialize)]
            struct FBInsightMetric {
                name: String,
                values: Vec<FBInsightValue>,
            }
            #[derive(Deserialize)]
            struct FBInsightValue {
                value: u64,
            }

            let result: Result<FBInsightsResponse, _> = resp.json().await;
            if let Ok(data) = result {
                let mut insights = Insights::default();
                for metric in data.insights.data {
                    let val = metric.values.first().map(|v| v.value).unwrap_or(0);
                    match metric.name.as_str() {
                        "impressions" => insights.impressions = val,
                        "reach" => insights.reach = val,
                        "engagements" | "engagement" => insights.engagements = val,
                        "reactions" => insights.likes = val,
                        "comments" => insights.comments = val,
                        "shares" | "shared" => insights.shares = val,
                        "saves" => insights.saves = val,
                        "clicks" => insights.clicks = val,
                        _ => {}
                    }
                }
                return Ok(insights);
            }
        }
        Ok(Insights::default())
    }

    async fn get_instagram_insights(&self, post_id: &str, _metric: &str) -> Result<Insights> {
        let creds = match self.get_credentials("instagram", "").await {
            Some(c) => c,
            None => return Ok(Insights::default()),
        };

        let url = format!(
            "https://graph.facebook.com/v18.0/{}?fields=insights.metric(impressions,reach,engagement,likes,comments,saves,views)&access_token={}",
            post_id, creds.access_token
        );

        let response = self.client.get(&url).send().await;
        if let Ok(resp) = response {
            #[derive(Deserialize)]
            struct IGInsightsResponse {
                insights: IGInsightsData,
            }
            #[derive(Deserialize)]
            struct IGInsightsData {
                data: Vec<IGInsightMetric>,
            }
            #[derive(Deserialize)]
            struct IGInsightMetric {
                name: String,
                values: Vec<IGInsightValue>,
            }
            #[derive(Deserialize)]
            struct IGInsightValue {
                value: u64,
            }

            let result: Result<IGInsightsResponse, _> = resp.json().await;
            if let Ok(data) = result {
                let mut insights = Insights::default();
                for metric in data.insights.data {
                    let val = metric.values.first().map(|v| v.value).unwrap_or(0);
                    match metric.name.as_str() {
                        "impressions" => insights.impressions = val,
                        "reach" => insights.reach = val,
                        "engagement" => insights.engagements = val,
                        "likes" => insights.likes = val,
                        "comments" => insights.comments = val,
                        "saves" => insights.saves = val,
                        "views" => insights.clicks = val,
                        _ => {}
                    }
                }
                return Ok(insights);
            }
        }
        Ok(Insights::default())
    }

    async fn get_linkedin_insights(
        &self,
        _platform: &str,
        _post_id: &str,
        _metric: &str,
    ) -> Result<Insights> {
        let creds = match self.get_credentials("linkedin_company", "").await {
            Some(c) => c,
            None => return Ok(Insights::default()),
        };

        let url = format!(
            "https://api.linkedin.com/v2/organizationalEntityShareStatistics?q=organizationalEntity&organizationalEntity=urn:li:organization:{}",
            creds.account_id
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", creds.access_token))
            .header("X-Restli-Protocol-Version", "2.0.0")
            .send()
            .await;

        if let Ok(resp) = response {
            #[derive(Deserialize)]
            struct LIInsightsResponse {
                elements: Vec<LIInsight>,
            }
            #[derive(Deserialize)]
            struct LIInsight {
                totalShareStatistics: LITotalStats,
            }
            #[derive(Deserialize)]
            struct LITotalStats {
                shareCount: u64,
                likeCount: u64,
                commentCount: u64,
                clickCount: u64,
                impressionCount: u64,
            }

            let result: Result<LIInsightsResponse, _> = resp.json().await;
            if let Ok(data) = result {
                if let Some(stats) = data.elements.first() {
                    return Ok(Insights {
                        impressions: stats.totalShareStatistics.impressionCount,
                        reach: 0,
                        engagements: stats.totalShareStatistics.clickCount,
                        likes: stats.totalShareStatistics.likeCount,
                        comments: stats.totalShareStatistics.commentCount,
                        shares: stats.totalShareStatistics.shareCount,
                        saves: 0,
                        clicks: stats.totalShareStatistics.clickCount,
                        followers: None,
                        period_start: Utc::now(),
                        period_end: Utc::now(),
                    });
                }
            }
        }
        Ok(Insights::default())
    }

    async fn get_tiktok_insights(&self, post_id: &str, _metric: &str) -> Result<Insights> {
        let creds = match self.get_credentials("tiktok", "").await {
            Some(c) => c,
            None => return Ok(Insights::default()),
        };

        let url = "https://open.tiktokapis.com/v2/video/stats/?fields=video_id,view_count,like_count,comment_count,share_count".to_string();

        let body = serde_json::json!({ "video_ids": [post_id] });
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", creds.access_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await;

        if let Ok(resp) = response {
            #[derive(Deserialize)]
            struct TTInsightsResponse {
                data: Vec<TTInsight>,
            }
            #[derive(Deserialize)]
            struct TTInsight {
                video_id: String,
                view_count: u64,
                like_count: u64,
                comment_count: u64,
                share_count: u64,
            }

            let result: Result<TTInsightsResponse, _> = resp.json().await;
            if let Ok(data) = result {
                if let Some(stats) = data.data.into_iter().find(|s| s.video_id == post_id) {
                    return Ok(Insights {
                        impressions: stats.view_count,
                        reach: stats.view_count,
                        engagements: stats.like_count + stats.comment_count + stats.share_count,
                        likes: stats.like_count,
                        comments: stats.comment_count,
                        shares: stats.share_count,
                        saves: 0,
                        clicks: stats.view_count,
                        followers: None,
                        period_start: Utc::now(),
                        period_end: Utc::now(),
                    });
                }
            }
        }
        Ok(Insights::default())
    }

    async fn get_youtube_insights(&self, video_id: &str, _metric: &str) -> Result<Insights> {
        let creds = match self.get_credentials("youtube", "").await {
            Some(c) => c,
            None => return Ok(Insights::default()),
        };

        let url = format!(
            "https://www.googleapis.com/youtube/v3/videos?id={}&key={}&part=statistics,snippet",
            video_id, creds.access_token
        );

        let response = self.client.get(&url).send().await;
        if let Ok(resp) = response {
            #[derive(Deserialize)]
            struct YTInsightsResponse {
                items: Vec<YTVideo>,
            }
            #[derive(Deserialize)]
            struct YTVideo {
                statistics: YTStats,
            }
            #[derive(Deserialize)]
            struct YTStats {
                viewCount: String,
                likeCount: String,
                commentCount: String,
                subscriberCount: Option<String>,
            }

            let result: Result<YTInsightsResponse, _> = resp.json().await;
            if let Ok(data) = result {
                if let Some(video) = data.items.first() {
                    let parse = |s: &str| s.parse::<u64>().unwrap_or(0);
                    return Ok(Insights {
                        impressions: parse(&video.statistics.viewCount),
                        reach: parse(&video.statistics.viewCount),
                        engagements: parse(&video.statistics.likeCount)
                            + parse(&video.statistics.commentCount),
                        likes: parse(&video.statistics.likeCount),
                        comments: parse(&video.statistics.commentCount),
                        shares: 0,
                        saves: 0,
                        clicks: parse(&video.statistics.viewCount),
                        followers: video.statistics.subscriberCount.as_ref().map(|s| parse(s)),
                        period_start: Utc::now(),
                        period_end: Utc::now(),
                    });
                }
            }
        }
        Ok(Insights::default())
    }

    async fn get_pinterest_insights(&self, post_id: &str, _metric: &str) -> Result<Insights> {
        let creds = match self.get_credentials("pinterest", "").await {
            Some(c) => c,
            None => return Ok(Insights::default()),
        };

        let url = format!("https://api.pinterest.com/v5/pins/{}/stats", post_id);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", creds.access_token))
            .send()
            .await;

        if let Ok(resp) = response {
            #[derive(Deserialize)]
            struct PinterestInsights {
                impression_count: u64,
                click_count: u64,
                save_count: u64,
            }

            let result: Result<PinterestInsights, _> = resp.json().await;
            if let Ok(stats) = result {
                return Ok(Insights {
                    impressions: stats.impression_count,
                    reach: stats.impression_count,
                    engagements: stats.click_count + stats.save_count,
                    likes: 0,
                    comments: 0,
                    shares: 0,
                    saves: stats.save_count,
                    clicks: stats.click_count,
                    followers: None,
                    period_start: Utc::now(),
                    period_end: Utc::now(),
                });
            }
        }
        Ok(Insights::default())
    }

    async fn get_threads_insights(&self, post_id: &str, _metric: &str) -> Result<Insights> {
        let creds = match self.get_credentials("threads", "").await {
            Some(c) => c,
            None => return Ok(Insights::default()),
        };

        let url = format!(
            "https://graph.facebook.com/v18.0/{}?fields=insights.metric(replies,reactions)&access_token={}",
            post_id, creds.access_token
        );

        let response = self.client.get(&url).send().await;
        if let Ok(resp) = response {
            #[derive(Deserialize)]
            struct ThreadsInsightsResponse {
                insights: ThreadsInsightsData,
            }
            #[derive(Deserialize)]
            struct ThreadsInsightsData {
                data: Vec<ThreadsInsightMetric>,
            }
            #[derive(Deserialize)]
            struct ThreadsInsightMetric {
                name: String,
                values: Vec<ThreadsInsightValue>,
            }
            #[derive(Deserialize)]
            struct ThreadsInsightValue {
                value: u64,
            }

            let result: Result<ThreadsInsightsResponse, _> = resp.json().await;
            if let Ok(data) = result {
                let mut insights = Insights::default();
                for metric in data.insights.data {
                    let val = metric.values.first().map(|v| v.value).unwrap_or(0);
                    match metric.name.as_str() {
                        "replies" => insights.comments = val,
                        "reactions" => insights.likes = val,
                        _ => {}
                    }
                }
                return Ok(insights);
            }
        }
        Ok(Insights::default())
    }

    async fn get_bluesky_insights(&self, post_uri: &str, _metric: &str) -> Result<Insights> {
        let creds = match self.get_credentials("bluesky", "").await {
            Some(c) => c,
            None => return Ok(Insights::default()),
        };

        let url = "https://bsky.social/xrpc/app.bsky.feed.getPostThread";
        let params = format!("?uri={}", post_uri);

        let response = self
            .client
            .get(&format!("{}{}", url, params))
            .bearer_auth(&creds.access_token)
            .send()
            .await;

        if let Ok(resp) = response {
            #[derive(Deserialize)]
            struct BSkyThreadResponse {
                thread: BSkyThread,
            }
            #[derive(Deserialize)]
            struct BSkyThread {
                post: BSkyPost,
            }
            #[derive(Deserialize)]
            struct BSkyPost {
                like_count: u64,
                reply_count: u64,
                repost_count: u64,
            }

            let result: Result<BSkyThreadResponse, _> = resp.json().await;
            if let Ok(data) = result {
                return Ok(Insights {
                    impressions: 0,
                    reach: 0,
                    engagements: data.thread.post.like_count
                        + data.thread.post.reply_count
                        + data.thread.post.repost_count,
                    likes: data.thread.post.like_count,
                    comments: data.thread.post.reply_count,
                    shares: data.thread.post.repost_count,
                    saves: 0,
                    clicks: 0,
                    followers: None,
                    period_start: Utc::now(),
                    period_end: Utc::now(),
                });
            }
        }
        Ok(Insights::default())
    }

    async fn get_google_business_insights(
        &self,
        post_name: &str,
        _metric: &str,
    ) -> Result<Insights> {
        let creds = match self.get_credentials("google_business", "").await {
            Some(c) => c,
            None => return Ok(Insights::default()),
        };

        let url = format!(
            "https://mybusiness.googleapis.com/v4/{}?metrics=views,clicks,actions",
            post_name
        );

        let response = self
            .client
            .get(&url)
            .bearer_auth(&creds.access_token)
            .send()
            .await;

        if let Ok(resp) = response {
            #[derive(Deserialize)]
            struct GBInsights {
                metrics: Vec<GBMetric>,
            }
            #[derive(Deserialize)]
            struct GBMetric {
                name: String,
                value: u64,
            }

            let result: Result<GBInsights, _> = resp.json().await;
            if let Ok(data) = result {
                let mut insights = Insights::default();
                for metric in data.metrics {
                    match metric.name.as_str() {
                        "views" => {
                            insights.impressions = metric.value;
                            insights.reach = metric.value;
                        }
                        "clicks" => insights.clicks = metric.value,
                        "actions" => insights.engagements = metric.value,
                        _ => {}
                    }
                }
                return Ok(insights);
            }
        }
        Ok(Insights::default())
    }
}

impl Default for SocialMediaManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for Insights {
    fn default() -> Self {
        Self {
            impressions: 0,
            reach: 0,
            engagements: 0,
            likes: 0,
            comments: 0,
            shares: 0,
            saves: 0,
            clicks: 0,
            followers: None,
            period_start: Utc::now(),
            period_end: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_manager_creation() {
        let manager = SocialMediaManager::new();
        assert!(manager.credentials.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_register_account() {
        let manager = SocialMediaManager::new();
        let creds = PlatformCredentials {
            platform: "facebook".to_string(),
            client_id: "test".to_string(),
            client_secret: "test".to_string(),
            access_token: "test_token".to_string(),
            refresh_token: None,
            expires_at: None,
            account_id: "test_page".to_string(),
            account_name: "Test Page".to_string(),
            account_type: AccountType::Business,
        };

        let result = manager.register_account(creds).await;
        assert!(result.is_ok());
    }
}
