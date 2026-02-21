//! Google Calendar Tool — manage events via Google Calendar API.
//!
//! Supports listing today's events, creating events, and checking free/busy.
//! Uses Google Calendar REST API with API key or OAuth2 service account.

use async_trait::async_trait;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use bizclaw_core::error::{BizClawError, Result};
use serde::{Deserialize, Serialize};

/// Calendar event representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: Option<String>,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start: String,   // ISO 8601
    pub end: String,     // ISO 8601
    pub all_day: bool,
    pub attendees: Vec<String>,
}

/// Google Calendar Tool configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarConfig {
    /// Google Calendar API key (for read-only access)
    pub api_key: Option<String>,
    /// Calendar ID (default: primary)
    #[serde(default = "default_calendar_id")]
    pub calendar_id: String,
    /// OAuth2 access token (for write access)
    pub access_token: Option<String>,
    /// Timezone (e.g., Asia/Ho_Chi_Minh)
    #[serde(default = "default_timezone")]
    pub timezone: String,
}

fn default_calendar_id() -> String { "primary".into() }
fn default_timezone() -> String { "Asia/Ho_Chi_Minh".into() }

impl Default for CalendarConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            calendar_id: "primary".into(),
            access_token: None,
            timezone: "Asia/Ho_Chi_Minh".into(),
        }
    }
}

/// Google Calendar tool for the BizClaw agent.
pub struct CalendarTool {
    config: CalendarConfig,
    client: reqwest::Client,
}

impl CalendarTool {
    pub fn new(config: CalendarConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// List events for a specific date or date range.
    async fn list_events(&self, date: &str, days: u32) -> Result<Vec<CalendarEvent>> {
        let base_date = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|e| BizClawError::Tool(format!("Invalid date format: {e}. Use YYYY-MM-DD")))?;

        let time_min = format!("{}T00:00:00+07:00", base_date);
        let end_date = base_date + chrono::Duration::days(days as i64);
        let time_max = format!("{}T23:59:59+07:00", end_date);

        let mut url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events",
            urlencoding::encode(&self.config.calendar_id)
        );

        let mut params = vec![
            format!("timeMin={}", urlencoding::encode(&time_min)),
            format!("timeMax={}", urlencoding::encode(&time_max)),
            format!("timeZone={}", urlencoding::encode(&self.config.timezone)),
            "singleEvents=true".into(),
            "orderBy=startTime".into(),
            "maxResults=50".into(),
        ];

        if let Some(ref key) = self.config.api_key {
            params.push(format!("key={key}"));
        }

        url = format!("{url}?{}", params.join("&"));

        let mut req = self.client.get(&url);
        if let Some(ref token) = self.config.access_token {
            req = req.header("Authorization", format!("Bearer {token}"));
        }

        let response = req.send().await
            .map_err(|e| BizClawError::Tool(format!("Calendar API request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BizClawError::Tool(format!(
                "Calendar API error {status}: {body}"
            )));
        }

        let body: serde_json::Value = response.json().await
            .map_err(|e| BizClawError::Tool(format!("Parse response failed: {e}")))?;

        let events = body["items"]
            .as_array()
            .map(|arr| {
                arr.iter().filter_map(|item| {
                    let summary = item["summary"].as_str()?.to_string();
                    let start = item["start"]["dateTime"]
                        .as_str()
                        .or_else(|| item["start"]["date"].as_str())?
                        .to_string();
                    let end = item["end"]["dateTime"]
                        .as_str()
                        .or_else(|| item["end"]["date"].as_str())?
                        .to_string();
                    let all_day = item["start"]["date"].is_string();

                    Some(CalendarEvent {
                        id: item["id"].as_str().map(String::from),
                        summary,
                        description: item["description"].as_str().map(String::from),
                        location: item["location"].as_str().map(String::from),
                        start,
                        end,
                        all_day,
                        attendees: item["attendees"]
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .filter_map(|att| att["email"].as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default(),
                    })
                }).collect()
            })
            .unwrap_or_default();

        Ok(events)
    }

    /// Create a new calendar event.
    async fn create_event(&self, event: &CalendarEvent) -> Result<String> {
        let token = self.config.access_token.as_ref()
            .ok_or_else(|| BizClawError::Tool("OAuth2 access_token required to create events".into()))?;

        let url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events",
            urlencoding::encode(&self.config.calendar_id)
        );

        let body = if event.all_day {
            serde_json::json!({
                "summary": event.summary,
                "description": event.description,
                "location": event.location,
                "start": { "date": event.start },
                "end": { "date": event.end },
            })
        } else {
            serde_json::json!({
                "summary": event.summary,
                "description": event.description,
                "location": event.location,
                "start": { "dateTime": event.start, "timeZone": self.config.timezone },
                "end": { "dateTime": event.end, "timeZone": self.config.timezone },
                "attendees": event.attendees.iter().map(|e| serde_json::json!({"email": e})).collect::<Vec<_>>(),
            })
        };

        let response = self.client.post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .json(&body)
            .send()
            .await
            .map_err(|e| BizClawError::Tool(format!("Create event failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let err_body = response.text().await.unwrap_or_default();
            return Err(BizClawError::Tool(format!("Create event error {status}: {err_body}")));
        }

        let result: serde_json::Value = response.json().await
            .map_err(|e| BizClawError::Tool(format!("Parse create response: {e}")))?;

        let event_id = result["id"].as_str().unwrap_or("unknown").to_string();
        let html_link = result["htmlLink"].as_str().unwrap_or("");

        Ok(format!("Event created: {event_id}\nLink: {html_link}"))
    }

    /// Format events for human-readable output.
    fn format_events(&self, events: &[CalendarEvent], date: &str) -> String {
        if events.is_empty() {
            return format!("📅 Không có sự kiện nào vào ngày {date}.");
        }

        let mut output = format!("📅 Lịch ngày {date} ({} sự kiện):\n\n", events.len());

        for (i, event) in events.iter().enumerate() {
            let time = if event.all_day {
                "Cả ngày".to_string()
            } else {
                // Extract time part from ISO 8601
                let start_time = event.start.split('T').nth(1)
                    .and_then(|t| t.split('+').next())
                    .unwrap_or(&event.start);
                let end_time = event.end.split('T').nth(1)
                    .and_then(|t| t.split('+').next())
                    .unwrap_or(&event.end);
                format!("{} - {}", &start_time[..5], &end_time[..5.min(end_time.len())])
            };

            output.push_str(&format!(
                "{}. ⏰ {} | {}\n",
                i + 1, time, event.summary
            ));

            if let Some(ref loc) = event.location {
                output.push_str(&format!("   📍 {loc}\n"));
            }
            if let Some(ref desc) = event.description {
                let short_desc: String = desc.chars().take(100).collect();
                output.push_str(&format!("   📝 {short_desc}\n"));
            }
            output.push('\n');
        }
        output
    }
}

#[async_trait]
impl Tool for CalendarTool {
    fn name(&self) -> &str { "calendar" }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "calendar".into(),
            description: "Quản lý Google Calendar — xem lịch, tạo sự kiện, kiểm tra lịch rảnh.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["list", "create", "today"],
                        "description": "Action: list (xem lịch ngày cụ thể), create (tạo sự kiện), today (xem lịch hôm nay)"
                    },
                    "date": {
                        "type": "string",
                        "description": "Date in YYYY-MM-DD format (for 'list' action)"
                    },
                    "days": {
                        "type": "integer",
                        "description": "Number of days to show (default 1)"
                    },
                    "summary": {
                        "type": "string",
                        "description": "Event title (for 'create' action)"
                    },
                    "start": {
                        "type": "string",
                        "description": "Start time ISO 8601 (for 'create', e.g. 2026-02-21T09:00:00)"
                    },
                    "end": {
                        "type": "string",
                        "description": "End time ISO 8601 (for 'create')"
                    },
                    "description": {
                        "type": "string",
                        "description": "Event description (optional)"
                    },
                    "location": {
                        "type": "string",
                        "description": "Event location (optional)"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments)
            .unwrap_or_else(|_| serde_json::json!({"action": "today"}));

        let action = args["action"].as_str().unwrap_or("today");

        let output = match action {
            "today" => {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let events = self.list_events(&today, 1).await?;
                self.format_events(&events, &today)
            }
            "list" => {
                let date = args["date"].as_str()
                    .unwrap_or(&chrono::Utc::now().format("%Y-%m-%d").to_string())
                    .to_string();
                let days = args["days"].as_u64().unwrap_or(1) as u32;
                let events = self.list_events(&date, days).await?;
                self.format_events(&events, &date)
            }
            "create" => {
                let summary = args["summary"].as_str()
                    .ok_or_else(|| BizClawError::Tool("Missing 'summary' for create".into()))?;
                let start = args["start"].as_str()
                    .ok_or_else(|| BizClawError::Tool("Missing 'start' for create".into()))?;
                let end = args["end"].as_str()
                    .ok_or_else(|| BizClawError::Tool("Missing 'end' for create".into()))?;

                let event = CalendarEvent {
                    id: None,
                    summary: summary.into(),
                    description: args["description"].as_str().map(String::from),
                    location: args["location"].as_str().map(String::from),
                    start: start.into(),
                    end: end.into(),
                    all_day: !start.contains('T'),
                    attendees: vec![],
                };

                self.create_event(&event).await?
            }
            _ => format!("Unknown action: {action}"),
        };

        Ok(ToolResult {
            tool_call_id: String::new(),
            output,
            success: true,
        })
    }
}
