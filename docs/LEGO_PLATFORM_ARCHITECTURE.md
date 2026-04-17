# BizClaw "Lego" Platform Architecture
## SME AI Content & Automation Platform

---

## Executive Summary

**BizClaw Lego** is a modular AI platform for SMEs that combines:
- **Automated content creation** (text, image, video)
- **Multi-channel social media scheduling**
- **Multi-agent orchestration** for business workflows
- **i18n-first design** (Vietnamese/English, extensible)

### Platform Comparison Matrix

| Feature | Moyin | Huobao | Waoowaoo | Postiz | DeerFlow | **BizClaw Lego** |
|---------|-------|--------|----------|--------|----------|------------------|
| AI Video Generation | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ |
| Script → Video Pipeline | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ |
| Social Media Scheduler | ❌ | ❌ | ❌ | ✅ | ❌ | ✅ |
| Multi-Channel Posting | ❌ | ❌ | ❌ | ✅ | ❌ | ✅ |
| Agent Orchestration | ❌ | ✅ | ❌ | ❌ | ✅ | ✅ |
| IM Channels | ❌ | ❌ | ❌ | ❌ | ✅ | ✅ |
| Vietnamese UI | ❌ | ❌ | ✅ | ❌ | ❌ | ✅ |
| Self-Hosted | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Rust Backend** | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |

---

## 1. Platform Research Analysis

### 1.1 Moyin Creator (MemeCalculate)
**Strengths:**
- Production-grade AI video workflow
- Seedance 2.0 integration
- Batch production capability
- Electron desktop app

**Tech Stack:**
- Electron 30 + React 18 + TypeScript
- Zustand state management
- Radix UI + Tailwind CSS 4

**BizClaw Integration Points:**
- Video generation module
- Batch processing queue
- Multi-provider AI scheduling

### 1.2 Huobao Drama (Chatfire-AI)
**Strengths:**
- Complete short drama pipeline
- Mastra AI Agent framework
- Multi-vendor media adapters
- Docker deployment ready

**Tech Stack:**
- Nuxt 3 + Vue 3
- Hono + Drizzle ORM
- Mastra AI Agents
- FFmpeg integration

**BizClaw Integration Points:**
- Agent-based workflow orchestration
- Multi-vendor adapter pattern
- Script → Video pipeline

### 1.3 Waoowaoo AI Film Studio
**Strengths:**
- Full Vietnamese localization (2,500+ strings)
- Three-language support (VI/ZH/EN)
- Docker-first deployment
- Next.js 15 + React 19

**Tech Stack:**
- Next.js 15 + React 19
- MySQL + Prisma ORM
- Redis + BullMQ
- next-intl for i18n

**BizClaw Integration Points:**
- i18n architecture (next-intl)
- Vietnamese-first UX design
- Job queue management

### 1.4 Postiz (GitRoom)
**Strengths:**
- Social media scheduling (X, Bluesky, Discord)
- Team collaboration
- API-first design (N8N, Make, Zapier)
- Temporal workflow engine

**Tech Stack:**
- Next.js + NestJS
- Prisma + PostgreSQL
- Temporal
- pnpm workspaces monorepo

**BizClaw Integration Points:**
- Social media integration
- Scheduling calendar
- API marketplace
- Team permissions

### 1.5 DeerFlow (Bytedance)
**Strengths:**
- LangGraph agent orchestration
- MCP server support
- IM Channels (Telegram, Slack, Feishu, WeChat)
- Sandbox execution
- Memory system

**Tech Stack:**
- LangGraph + LangChain
- Python backend
- Docker/Kubernetes sandbox
- Multi-provider LLM

**BizClaw Integration Points:**
- Agent orchestration framework
- MCP tool system
- IM channel adapters
- Memory/RAG integration

---

## 2. BizClaw Lego Architecture

### 2.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     BizClaw Lego Platform                        │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐   │
│  │   Web UI     │  │  Mobile App  │  │   CLI / API          │   │
│  │  (Next.js)   │  │  (React)     │  │   (Rust)             │   │
│  └──────┬───────┘  └──────┬───────┘  └──────────┬───────────┘   │
│         │                 │                     │               │
│  ┌──────▼─────────────────▼─────────────────────▼───────────┐  │
│  │                    API Gateway (Rust)                       │  │
│  │  • JWT Auth  • Rate Limiting  • Multi-tenant  • WebSocket   │  │
│  └──────┬────────────────────────────────────────────┬────────┘  │
│         │                                             │          │
│  ┌──────▼────────────────────────────────────────────▼────────┐  │
│  │               Agent Orchestration Layer                   │  │
│  │  ┌─────────┐  ┌──────────┐  ┌─────────┐  ┌──────────────┐   │  │
│  │  │ Content │  │ Scheduler│  │ Social │  │ Video        │   │  │
│  │  │ Agent   │  │ Agent    │  │ Agent  │  │ Agent        │   │  │
│  │  └─────────┘  └──────────┘  └─────────┘  └──────────────┘   │  │
│  └──────┬────────────────────────────────────────────┬────────┘  │
│         │                                             │          │
│  ┌──────▼────────────────────────────────────────────▼────────┐  │
│  │                    Tool Layer                              │  │
│  │  ┌────────┐  ┌──────────┐  ┌─────────┐  ┌────────────────┐    │  │
│  │  │ MCP    │  │ Skills   │  │ RAG     │  │ Media         │    │  │
│  │  │ Tools  │  │ Registry │  │ Search  │  │ Generators    │    │  │
│  │  └────────┘  └──────────┘  └─────────┘  └────────────────┘    │  │
│  └─────────────────────────────────────────────────────────────┘  │
│         │                     │                     │          │
│  ┌──────▼─────────────────────▼─────────────────────▼────────┐  │
│  │                  External Integrations                     │  │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌─────────┐    │  │
│  │  │MiniMax │ │OpenAI  │ │Zalo OA │ │ TikTok │ │ Google  │    │  │
│  │  │ API    │ │ API    │ │ API    │ │ API    │ │ Calendar│    │  │
│  │  └────────┘ └────────┘ └────────┘ └────────┘ └─────────┘    │  │
│  └─────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 Core Modules

#### Module A: Content Generation (Inspired by Huobao/Waoowaoo)

```rust
// bizclaw-lego/src/content/mod.rs

pub struct ContentAgent {
    llm: Box<dyn LlmProvider>,
    script_writer: ScriptWriter,
    character_extractor: CharacterExtractor,
    scene_generator: SceneGenerator,
    voice_assigner: VoiceAssigner,
}

impl ContentAgent {
    /// Full pipeline: Script → Characters → Scenes → Video
    pub async fn generate_video(&self, prompt: &str) -> Result<VideoProject> {
        // 1. Generate formatted script
        let script = self.script_writer.write(prompt).await?;
        
        // 2. Extract and deduplicate characters
        let characters = self.character_extractor.extract(&script).await?;
        
        // 3. Generate scene images
        let scenes = self.scene_generator.create_scenes(&script).await?;
        
        // 4. Assign voices to characters
        let voice_assignments = self.voice_assigner.assign(&characters).await?;
        
        // 5. Generate video from scenes
        self.video_agent.composite(scenes, voice_assignments).await
    }
}
```

#### Module B: Social Media Scheduler (Inspired by Postiz)

```rust
// bizclaw-lego/src/scheduler/mod.rs

pub struct SchedulerAgent {
    calendar: Calendar,
    platforms: HashMap<Platform, SocialClient>,
    queue: JobQueue,
}

pub enum Platform {
    Facebook,
    Instagram,
    TikTok,
    ZaloOA,
    Shopee,
    Lazada,
    Twitter,
    LinkedIn,
}

impl SchedulerAgent {
    /// Schedule content across multiple platforms
    pub async fn schedule(
        &self,
        content: Content,
        platforms: Vec<Platform>,
        schedule_time: DateTime,
    ) -> Result<Vec<ScheduledPost>> {
        let mut posts = Vec::new();
        
        for platform in platforms {
            let client = self.platforms.get(&platform).unwrap();
            let adapted = self.adapt_content_for_platform(&content, platform)?;
            let post = client.schedule(adapted, schedule_time).await?;
            posts.push(post);
        }
        
        Ok(posts)
    }
    
    /// Adaptive content for each platform
    fn adapt_content_for_platform(&self, content: &Content, platform: Platform) -> Result<PlatformContent> {
        match platform {
            Platform::TikTok => Ok(PlatformContent {
                text: truncate(content.text, 150),
                hashtags: extract_hashtags(content.text),
                video: content.video.as_ref().map(|v| v.tiktok_format()),
                ..Default::default()
            }),
            Platform::ZaloOA => Ok(PlatformContent {
                text: content.text,
                image: content.image.as_ref().map(|i| i.zalo_format()),
                link_preview: content.link,
                ..Default::default()
            }),
            // ... other platforms
        }
    }
}
```

#### Module C: Agent Orchestration (Inspired by DeerFlow)

```rust
// bizclaw-lego/src/orchestrator/mod.rs

use langgraph::prelude::*;

pub struct LegoOrchestrator {
    graph: StateGraph<AgentState>,
    memory: MemoryStore,
    mcp_registry: McpRegistry,
}

#[derive(State, Serialize, Deserialize)]
pub struct AgentState {
    pub messages: Vec<Message>,
    pub context: Context,
    pub current_agent: Option<AgentId>,
    pub tools_used: Vec<ToolCall>,
}

impl LegoOrchestrator {
    pub fn new() -> Self {
        let graph = StateGraph::new(AgentState::new)
            .add_edge("supervisor", "content_agent", |s| condition!(s.task == "create_content"))
            .add_edge("supervisor", "scheduler_agent", |s| condition!(s.task == "schedule_post"))
            .add_edge("supervisor", "social_agent", |s| condition!(s.task == "monitor_social"))
            .add_conditional_edges("supervisor", should_continue, &["content_agent", "scheduler_agent", "social_agent"])
            .add_node("supervisor", supervisor_node)
            .build();
            
        Self { graph, memory: MemoryStore::new(), mcp_registry: McpRegistry::new() }
    }
    
    /// Register custom MCP tools
    pub fn register_mcp_server(&mut self, server: McpServer) -> Result<()> {
        self.mcp_registry.register(server);
        Ok(())
    }
}
```

### 2.3 i18n System Architecture

```rust
// bizclaw-lego/src/i18n/mod.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Locale {
    pub code: String,      // "vi", "en", "zh"
    pub name: String,      // "Tiếng Việt", "English", "中文"
    pub direction: TextDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextDirection {
    Ltr,   // Left-to-right
    Rtl,   // Right-to-left (future: Arabic, Hebrew)
}

pub struct I18n {
    locales: HashMap<String, Locale>,
    translations: HashMap<String, TranslationMap>,
    fallback: String,
}

impl I18n {
    pub fn new() -> Self {
        let mut i18n = Self {
            locales: HashMap::new(),
            translations: HashMap::new(),
            fallback: "en".to_string(),
        };
        
        // Register built-in locales
        i18n.register_locale(Locale {
            code: "vi".to_string(),
            name: "Tiếng Việt".to_string(),
            direction: TextDirection::Ltr,
        });
        
        i18n.register_locale(Locale {
            code: "en".to_string(),
            name: "English".to_string(),
            direction: TextDirection::Ltr,
        });
        
        i18n
    }
    
    pub fn t(&self, key: &str, locale: &str) -> String {
        let translations = self.translations.get(locale)
            .or_else(|| self.translations.get(&self.fallback));
            
        translations
            .and_then(|t| t.get(key))
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }
}

// Translation files structure
// i18n/
// ├── vi.json
// ├── en.json
// └── zh.json
```

### 2.4 Vietnamese Translation File

```json
// i18n/vi.json
{
  "common": {
    "loading": "Đang tải...",
    "error": "Lỗi",
    "success": "Thành công",
    "cancel": "Hủy",
    "save": "Lưu",
    "delete": "Xóa",
    "edit": "Sửa",
    "create": "Tạo mới",
    "search": "Tìm kiếm",
    "filter": "Lọc",
    "export": "Xuất",
    "import": "Nhập"
  },
  "auth": {
    "login": "Đăng nhập",
    "register": "Đăng ký",
    "logout": "Đăng xuất",
    "email": "Email",
    "password": "Mật khẩu",
    "forgot_password": "Quên mật khẩu?",
    "welcome_back": "Chào mừng trở lại",
    "enter_credentials": "Nhập thông tin đăng nhập"
  },
  "content": {
    "title": "Tạo Nội Dung",
    "generate_script": "Tạo kịch bản",
    "generate_image": "Tạo hình ảnh",
    "generate_video": "Tạo video",
    "script_placeholder": "Nhập mô tả nội dung bạn muốn tạo...",
    "character": "Nhân vật",
    "scene": "Cảnh quay",
    "voice": "Giọng đọc",
    "duration": "Thời lượng",
    "style": "Phong cách"
  },
  "scheduler": {
    "title": "Lịch Đăng Bài",
    "schedule_post": "Đặt lịch đăng",
    "select_platforms": "Chọn nền tảng",
    "select_date": "Chọn ngày",
    "select_time": "Chọn giờ",
    "scheduled": "Đã đặt lịch",
    "pending": "Đang chờ",
    "published": "Đã đăng",
    "failed": "Thất bại"
  },
  "social": {
    "title": "Mạng Xã Hội",
    "connect": "Kết nối",
    "disconnect": "Ngắt kết nối",
    "facebook": "Facebook",
    "instagram": "Instagram",
    "tiktok": "TikTok",
    "zalo": "Zalo OA",
    "shopee": "Shopee",
    "followers": "Người theo dõi",
    "engagement": "Tương tác"
  },
  "analytics": {
    "title": "Phân Tích",
    "views": "Lượt xem",
    "clicks": "Lượt nhấn",
    "shares": "Lượt chia sẻ",
    "comments": "Bình luận",
    "reactions": "Cảm xúc",
    "growth": "Tăng trưởng",
    "best_performing": "Nội dung hiệu quả nhất"
  },
  "settings": {
    "title": "Cài Đặt",
    "profile": "Hồ sơ",
    "account": "Tài khoản",
    "notifications": "Thông báo",
    "integrations": "Tích hợp",
    "api_keys": "API Keys",
    "language": "Ngôn ngữ",
    "theme": "Giao diện"
  }
}
```

---

## 3. SME Lean Implementation Strategy

### 3.1 AI-First Operations Model

```
┌─────────────────────────────────────────────────────────────────┐
│              SME Operations - Before vs After BizClaw             │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  BEFORE (Traditional)         AFTER (BizClaw Lego)              │
│  ───────────────────         ─────────────────────              │
│                                                                  │
│  ┌─────────────────┐         ┌─────────────────────┐            │
│  │ Content Team    │         │ AI Content Agent    │            │
│  │ • Copywriter    │    →    │ (Automated)         │            │
│  │ • Designer      │         └─────────────────────┘            │
│  │ • Video Editor  │                                          │
│  └─────────────────┘         ┌─────────────────────┐            │
│         ↓                    │ Scheduler Agent     │            │
│  ┌─────────────────┐         │ (Auto-posting)      │            │
│  │ Scheduling      │    →    └─────────────────────┘            │
│  │ • Manual post   │                                          │
│  │ • Excel tracker │         ┌─────────────────────┐            │
│  └─────────────────┘         │ Analytics Agent     │            │
│         ↓                    │ (Auto-reporting)    │            │
│  ┌─────────────────┐         └─────────────────────┘            │
│  │ Analytics       │                                          │
│  │ • Manual export  │         ┌─────────────────────┐            │
│  │ • Weekly report  │    →   │ Customer Agent      │            │
│  └─────────────────┘         │ (Auto-response)     │            │
│         ↓                    └─────────────────────┘            │
│  ┌─────────────────┐                                           │
│  │ Customer Care   │         ┌─────────────────────┐            │
│  │ • 3-5 staff      │    →    │ Human Oversight     │            │
│  │ • 8-10h/day      │         │ (1-2 staff)         │            │
│  └─────────────────┘         └─────────────────────┘            │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│  Staff Reduction: 8-12 → 2-3 (75% reduction)                    │
│  Content Output: 10 posts/week → 50+ posts/week (5x increase)  │
│  Response Time: 2-4 hours → 5-15 minutes (10x faster)          │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 Modular Pricing Tiers

| Feature | Starter | Pro | Enterprise |
|---------|---------|-----|------------|
| Price (VND/month) | 299,000 | 799,000 | 2,999,000 |
| Content Generation | 50/month | 200/month | Unlimited |
| Social Channels | 2 | 5 | Unlimited |
| AI Agents | 1 | 3 | 10 |
| Team Members | 1 | 5 | 20 |
| API Access | ❌ | ✅ | ✅ |
| Custom Branding | ❌ | ❌ | ✅ |
| Priority Support | ❌ | ✅ | ✅ |

### 3.3 Technical Implementation Phases

```
Phase 1: Foundation (MVP)
├── bizclaw-lego-core (Rust)
│   ├── i18n system (VI/EN)
│   ├── Content agent (text)
│   └── Basic API gateway
├── Frontend (Next.js)
│   ├── Dashboard
│   ├── Content editor
│   └── Scheduler
└── Database: SQLite → PostgreSQL

Phase 2: Social Integration
├── Platform adapters (Zalo, TikTok, Shopee)
├── Multi-channel scheduler
├── Analytics dashboard
└── Webhook handlers

Phase 3: Media Generation
├── Image generation (MiniMax)
├── Video generation (MiniMax/Seedance)
├── Voice synthesis (TTS)
└── Media library

Phase 4: Agent Orchestration
├── LangGraph integration
├── MCP server support
├── IM channels (Telegram, Zalo)
└── Memory/RAG system

Phase 5: Enterprise
├── Multi-tenant SaaS
├── Team collaboration
├── White-label
└── On-premise deployment
```

---

## 4. Technology Stack Selection

### Backend (Rust)
```toml
# bizclaw-lego/Cargo.toml
[dependencies]
# Web framework
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "auth"] }

# Database
sqlx = { version = "0.7", features = ["runtime-tokio", "postgres", "sqlite"] }
prisma-client-rust = "0.6"

# AI/ML
langchain-openai = "0.1"
minimax-api = "0.1"
reqwest = { version = "0.11", features = ["json"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# i18n
fluent = "0.16"
intl-matcher = "0.5"

# Async
tokio = { version = "1", features = ["full"] }
futures = "0.3"

# Auth
jsonwebtoken = "9"
argon2 = "0.5"

# Utils
tracing = "0.1"
anyhow = "1"
thiserror = "1"
```

### Frontend (Next.js + React)
```typescript
// package.json
{
  "dependencies": {
    "next": "^15.0.0",
    "react": "^19.0.0",
    "@tanstack/react-query": "^5.0.0",
    "zustand": "^5.0.0",
    "next-intl": "^4.0.0",
    "tailwindcss": "^4.0.0",
    "@radix-ui/react-...": "latest",
    "lucide-react": "latest",
    "date-fns": "^4.0.0",
    "react-hook-form": "^7.0.0",
    "zod": "^3.0.0"
  }
}
```

---

## 5. Implementation Roadmap

### Q1 2025: Foundation
- [x] BizClaw core modules (ecommerce, content, office)
- [x] Gateway with auth & multi-tenancy
- [ ] **NEW:** bizclaw-lego crate
- [ ] i18n system (VI/EN)
- [ ] Basic API

### Q2 2025: Social Integration
- [ ] Zalo OA integration
- [ ] TikTok Shop integration
- [ ] Multi-channel scheduler
- [ ] Basic analytics

### Q3 2025: Media Generation
- [ ] MiniMax API integration
- [ ] Image generation pipeline
- [ ] Video generation pipeline
- [ ] Media library

### Q4 2025: Agent System
- [ ] LangGraph integration
- [ ] MCP server
- [ ] IM channels
- [ ] RAG knowledge base

---

## 6. Key Differentiators for BizClaw Lego

### vs. Moyin/Huobao/Waoowaoo
- **Multi-platform**: Not just video, but full content-to-distribution pipeline
- **Social native**: Built-in scheduling and analytics, not just generation
- **Rust backend**: Better performance, smaller footprint

### vs. Postiz
- **AI-first**: Not just scheduling, but AI-generated content
- **Agent orchestration**: Complex workflows beyond simple posting
- **Vietnamese market**: Deep integration with Zalo, TikTok Shop

### vs. DeerFlow
- **Business focus**: SME use cases, not research
- **Social media**: First-class platform integrations
- **Visual UI**: Dashboard, not just CLI
- **i18n**: Native Vietnamese support

---

## 7. Next Steps

1. **Create bizclaw-lego crate** with i18n foundation
2. **Implement content generation pipeline**
3. **Build social media adapters**
4. **Create Next.js dashboard with i18n**
5. **Integrate MiniMax API for media generation**

---

*Document Version: 1.0*
*Last Updated: 2025-01-26*
*Author: BizClaw Team*
