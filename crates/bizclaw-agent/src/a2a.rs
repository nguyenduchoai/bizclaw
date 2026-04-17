//! # A2A Protocol - Google Agent-to-Agent Communication
//!
//! Implements Google A2A v0.3 specification for cross-network agent collaboration.
//!
//! ## Features:
//! - Auto-discovery via `/.well-known/agent.json`
//! - JSON-RPC 2.0 task dispatch
//! - Streaming support with Server-Sent Events
//! - Task state management (submitted → working → completed/failed/input-required)
//!
//! ## Endpoints:
//! - `GET /.well-known/agent.json` - Agent card (capabilities, endpoint)
//! - `POST /a2a/v1/tasks/send` - Send task to agent
//! - `GET /a2a/v1/tasks/{id}` - Get task status
//! - `GET /a2a/v1/tasks/{id}/stream` - Stream task events

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCard {
    pub name: String,
    pub version: String,
    pub description: String,
    pub provider: AgentProvider,
    pub capabilities: AgentCapabilities,
    pub authentication: AuthenticationConfig,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProvider {
    pub organization: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCapabilities {
    pub streaming: bool,
    pub push_notifications: bool,
    pub state_transition_history: bool,
    pub skills: Vec<Skill>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticationConfig {
    pub schemes: Vec<String>,
    pub bearer: Option<BearerAuth>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BearerAuth {
    pub authorization_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Submitted,
    Working,
    Completed,
    Failed,
    InputRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatusUpdate {
    Submitted,
    Working,
    Completed,
    Failed { error: String },
    InputRequired { required: Vec<InputRequest> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputRequest {
    pub name: String,
    pub type_: String,
    pub description: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendTaskRequest {
    pub id: Option<String>,
    pub session_id: Option<String>,
    pub messages: Vec<Message>,
    pub skill_id: Option<String>,
    pub push_notification: Option<PushNotificationConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushNotificationConfig {
    pub url: String,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendTaskResponse {
    pub id: String,
    pub session_id: Option<String>,
    pub status: TaskStatus,
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTaskResponse {
    pub id: String,
    pub session_id: Option<String>,
    pub status: TaskStatus,
    pub messages: Vec<Message>,
    pub history: Vec<TaskStatusUpdate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskEvent {
    pub id: String,
    pub session_id: Option<String>,
    pub status: TaskStatusUpdate,
    pub message: Option<Message>,
}

pub struct A2AServer {
    agent_card: AgentCard,
    tasks: Arc<RwLock<HashMap<String, TaskState>>>,
}

struct TaskState {
    id: String,
    session_id: Option<String>,
    status: TaskStatus,
    messages: Vec<Message>,
    history: Vec<TaskStatusUpdate>,
}

impl A2AServer {
    pub fn new(name: String, url: String) -> Self {
        let agent_card = AgentCard {
            name: name.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            description: "BizClaw AI Agent - SME Business Automation".to_string(),
            provider: AgentProvider {
                organization: "BizClaw".to_string(),
                url: "https://bizclaw.vn".to_string(),
            },
            capabilities: AgentCapabilities {
                streaming: true,
                push_notifications: true,
                state_transition_history: true,
                skills: vec![
                    Skill {
                        id: "business-analysis".to_string(),
                        name: "Business Analysis".to_string(),
                        description: "Analyze business data and generate reports".to_string(),
                        tags: vec!["analytics".to_string(), "reporting".to_string()],
                    },
                    Skill {
                        id: "content-creation".to_string(),
                        name: "Content Creation".to_string(),
                        description: "Create marketing content and copy".to_string(),
                        tags: vec!["marketing".to_string(), "content".to_string()],
                    },
                    Skill {
                        id: "customer-support".to_string(),
                        name: "Customer Support".to_string(),
                        description: "Handle customer inquiries and support".to_string(),
                        tags: vec!["support".to_string(), "chatbot".to_string()],
                    },
                ],
            },
            authentication: AuthenticationConfig {
                schemes: vec!["bearer".to_string()],
                bearer: Some(BearerAuth {
                    authorization_url: format!("{}/auth", url),
                }),
            },
            url,
        };

        Self {
            agent_card,
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn agent_card(&self) -> &AgentCard {
        &self.agent_card
    }

    pub async fn send_task(&self, request: SendTaskRequest) -> SendTaskResponse {
        let task_id = request
            .id
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let task_state = TaskState {
            id: task_id.clone(),
            session_id: request.session_id.clone(),
            status: TaskStatus::Submitted,
            messages: request.messages.clone(),
            history: vec![TaskStatusUpdate::Submitted],
        };

        self.tasks.write().await.insert(task_id.clone(), task_state);

        SendTaskResponse {
            id: task_id,
            session_id: request.session_id,
            status: TaskStatus::Submitted,
            messages: request.messages,
        }
    }

    pub async fn get_task(&self, id: &str) -> Option<GetTaskResponse> {
        let tasks = self.tasks.read().await;
        tasks.get(id).map(|state| GetTaskResponse {
            id: state.id.clone(),
            session_id: state.session_id.clone(),
            status: state.status.clone(),
            messages: state.messages.clone(),
            history: state.history.clone(),
        })
    }

    pub async fn update_task_status(&self, id: &str, status: TaskStatusUpdate) {
        if let Some(task) = self.tasks.write().await.get_mut(id) {
            task.status = match &status {
                TaskStatusUpdate::Submitted => TaskStatus::Submitted,
                TaskStatusUpdate::Working => TaskStatus::Working,
                TaskStatusUpdate::Completed => TaskStatus::Completed,
                TaskStatusUpdate::Failed { .. } => TaskStatus::Failed,
                TaskStatusUpdate::InputRequired { .. } => TaskStatus::InputRequired,
            };
            task.history.push(status);
        }
    }

    pub async fn add_message(&self, id: &str, message: Message) {
        if let Some(task) = self.tasks.write().await.get_mut(id) {
            task.messages.push(message);
        }
    }

    pub async fn stream_events(&self, id: &str) -> mpsc::Receiver<TaskEvent> {
        let (tx, rx) = mpsc::channel(100);
        let tasks = self.tasks.clone();
        let task_id = id.to_string();

        tokio::spawn(async move {
            if let Some(task) = tasks.read().await.get(&task_id) {
                let _ = tx
                    .send(TaskEvent {
                        id: task.id.clone(),
                        session_id: task.session_id.clone(),
                        status: match task.status {
                            TaskStatus::Submitted => TaskStatusUpdate::Submitted,
                            TaskStatus::Working => TaskStatusUpdate::Working,
                            TaskStatus::Completed => TaskStatusUpdate::Completed,
                            TaskStatus::Failed => TaskStatusUpdate::Failed {
                                error: String::new(),
                            },
                            TaskStatus::InputRequired => {
                                TaskStatusUpdate::InputRequired { required: vec![] }
                            }
                        },
                        message: task.messages.last().cloned(),
                    })
                    .await;
            }
        });

        rx
    }
}

#[derive(Debug, Clone)]
pub struct A2AClient {
    base_url: String,
    auth_token: Option<String>,
}

impl A2AClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            auth_token: None,
        }
    }

    pub fn with_auth(mut self, token: String) -> Self {
        self.auth_token = Some(token);
        self
    }

    pub async fn discover_agent(&self, url: &str) -> Result<AgentCard, String> {
        let well_known_url = format!("{}/.well-known/agent.json", url.trim_end_matches('/'));
        let client = reqwest::Client::new();

        let mut request = client.get(&well_known_url);
        if let Some(token) = &self.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        request
            .send()
            .await
            .map_err(|e| format!("Discovery failed: {e}"))?
            .json::<AgentCard>()
            .await
            .map_err(|e| format!("Parse agent card failed: {e}"))
    }

    pub async fn send_task(
        &self,
        agent_url: &str,
        request: SendTaskRequest,
    ) -> Result<SendTaskResponse, String> {
        let url = format!("{}/a2a/v1/tasks/send", agent_url.trim_end_matches('/'));
        let client = reqwest::Client::new();

        let mut req_builder = client.post(&url).json(&request);
        if let Some(token) = &self.auth_token {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", token));
        }

        req_builder
            .send()
            .await
            .map_err(|e| format!("Send task failed: {e}"))?
            .json::<SendTaskResponse>()
            .await
            .map_err(|e| format!("Parse response failed: {e}"))
    }

    pub async fn get_task(
        &self,
        agent_url: &str,
        task_id: &str,
    ) -> Result<GetTaskResponse, String> {
        let url = format!(
            "{}/a2a/v1/tasks/{}",
            agent_url.trim_end_matches('/'),
            task_id
        );
        let client = reqwest::Client::new();

        let mut req_builder = client.get(&url);
        if let Some(token) = &self.auth_token {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", token));
        }

        req_builder
            .send()
            .await
            .map_err(|e| format!("Get task failed: {e}"))?
            .json::<GetTaskResponse>()
            .await
            .map_err(|e| format!("Parse response failed: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_card_creation() {
        let server = A2AServer::new(
            "BizClaw-Test".to_string(),
            "http://localhost:3000".to_string(),
        );
        let card = server.agent_card();

        assert_eq!(card.name, "BizClaw-Test");
        assert_eq!(card.capabilities.streaming, true);
        assert!(!card.capabilities.skills.is_empty());
    }

    #[tokio::test]
    async fn test_send_and_get_task() {
        let server = A2AServer::new("BizClaw".to_string(), "http://localhost:3000".to_string());

        let request = SendTaskRequest {
            id: None,
            session_id: Some("test-session".to_string()),
            messages: vec![Message {
                role: "user".to_string(),
                content: "Hello agent!".to_string(),
                name: None,
            }],
            skill_id: Some("business-analysis".to_string()),
            push_notification: None,
        };

        let response = server.send_task(request).await;
        assert_eq!(response.status, TaskStatus::Submitted);

        let task = server.get_task(&response.id).await;
        assert!(task.is_some());
        assert_eq!(task.unwrap().messages.len(), 1);
    }
}
