-- ═══════════════════════════════════════════════════════════════
-- BizClaw Platform — PostgreSQL Schema v0.3
-- Includes: Core Platform + ReMe Memory + Heartbeat + Skills
-- ═══════════════════════════════════════════════════════════════

-- Enable extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";    -- For text search

-- ════════════════════════════════════════════════
-- 1. CORE PLATFORM TABLES (migrated from SQLite)
-- ════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS tenants (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(100) UNIQUE NOT NULL,
    status VARCHAR(20) DEFAULT 'stopped',
    port INTEGER UNIQUE,
    plan VARCHAR(20) DEFAULT 'free',
    provider VARCHAR(50) DEFAULT 'openai',
    model VARCHAR(100) DEFAULT 'gpt-4o-mini',
    max_messages_day INTEGER DEFAULT 100,
    max_channels INTEGER DEFAULT 3,
    max_members INTEGER DEFAULT 5,
    pairing_code VARCHAR(10),
    pid INTEGER,
    cpu_percent REAL DEFAULT 0,
    memory_bytes BIGINT DEFAULT 0,
    disk_bytes BIGINT DEFAULT 0,
    owner_id UUID,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    role VARCHAR(20) DEFAULT 'user',
    tenant_id UUID REFERENCES tenants(id) ON DELETE SET NULL,
    status VARCHAR(20) DEFAULT 'active',
    last_login TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS audit_log (
    id SERIAL PRIMARY KEY,
    event_type VARCHAR(100) NOT NULL,
    actor_type VARCHAR(50) NOT NULL,
    actor_id VARCHAR(255) NOT NULL,
    details TEXT,
    ip_address VARCHAR(50),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS tenant_members (
    tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(20) DEFAULT 'member',
    PRIMARY KEY (tenant_id, user_id)
);

CREATE TABLE IF NOT EXISTS tenant_channels (
    id VARCHAR(255) PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    channel_type VARCHAR(50) NOT NULL,
    instance_id VARCHAR(100) DEFAULT '',
    enabled BOOLEAN DEFAULT true,
    config_json JSONB DEFAULT '{}',
    status VARCHAR(20) DEFAULT 'disconnected',
    status_message TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(tenant_id, channel_type, instance_id)
);

CREATE TABLE IF NOT EXISTS tenant_configs (
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    key VARCHAR(255) NOT NULL,
    value TEXT NOT NULL DEFAULT '',
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (tenant_id, key)
);

CREATE TABLE IF NOT EXISTS tenant_agents (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    role VARCHAR(50) DEFAULT 'assistant',
    description TEXT DEFAULT '',
    provider VARCHAR(50) DEFAULT 'openai',
    model VARCHAR(100) DEFAULT 'gpt-4o-mini',
    system_prompt TEXT DEFAULT '',
    enabled BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(tenant_id, name)
);

CREATE TABLE IF NOT EXISTS password_resets (
    email VARCHAR(255) PRIMARY KEY,
    token VARCHAR(255) NOT NULL,
    expires_at BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS platform_configs (
    key VARCHAR(255) PRIMARY KEY,
    value TEXT NOT NULL DEFAULT '',
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- ════════════════════════════════════════════════
-- 2. ReMe-INSPIRED MEMORY SYSTEM
-- 4 types: Personal, Task, Tool, Working
-- ════════════════════════════════════════════════

-- Personal Memory — user preferences, learned context, personality
CREATE TABLE IF NOT EXISTS memory_personal (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    user_id VARCHAR(255),           -- external user (telegram id, etc.)
    category VARCHAR(100) NOT NULL, -- 'preference', 'context', 'personality', 'fact'
    key VARCHAR(500) NOT NULL,      -- e.g. 'language', 'timezone', 'name'
    value TEXT NOT NULL,
    confidence REAL DEFAULT 1.0,    -- how confident the AI is (0-1)
    source VARCHAR(100),            -- 'inferred', 'explicit', 'observed'
    last_accessed TIMESTAMPTZ DEFAULT NOW(),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_mem_personal_tenant ON memory_personal(tenant_id);
CREATE INDEX IF NOT EXISTS idx_mem_personal_user ON memory_personal(tenant_id, user_id);
CREATE INDEX IF NOT EXISTS idx_mem_personal_cat ON memory_personal(tenant_id, category);

-- Task Memory — past task patterns, success/failure, learned approaches
CREATE TABLE IF NOT EXISTS memory_task (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    task_type VARCHAR(200) NOT NULL,    -- 'code_review', 'translation', 'research'
    task_description TEXT NOT NULL,
    approach TEXT,                       -- what approach was used
    outcome VARCHAR(20) DEFAULT 'unknown', -- 'success', 'failure', 'partial'
    lessons_learned TEXT,               -- what worked, what didn't
    duration_seconds INTEGER,
    tokens_used INTEGER,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_mem_task_tenant ON memory_task(tenant_id);
CREATE INDEX IF NOT EXISTS idx_mem_task_type ON memory_task(tenant_id, task_type);

-- Tool Memory — tool usage patterns, success rates, optimization data
CREATE TABLE IF NOT EXISTS memory_tool (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    tool_name VARCHAR(200) NOT NULL,     -- 'web_search', 'code_exec', 'file_read'
    usage_count INTEGER DEFAULT 1,
    success_count INTEGER DEFAULT 0,
    failure_count INTEGER DEFAULT 0,
    avg_duration_ms INTEGER DEFAULT 0,
    last_error TEXT,
    tips TEXT,                           -- learned tips for this tool
    metadata JSONB DEFAULT '{}',
    last_used TIMESTAMPTZ DEFAULT NOW(),
    created_at TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_mem_tool_tenant ON memory_tool(tenant_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_mem_tool_unique ON memory_tool(tenant_id, tool_name);

-- Working Memory — short-term context, conversation summaries, compacted history
CREATE TABLE IF NOT EXISTS memory_working (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    session_id VARCHAR(255) NOT NULL,     -- conversation/session identifier
    channel VARCHAR(50),                  -- 'telegram', 'discord', etc.
    user_id VARCHAR(255),
    summary TEXT NOT NULL,                -- compressed conversation summary
    key_facts JSONB DEFAULT '[]',         -- extracted key facts as JSON array
    message_count INTEGER DEFAULT 0,
    token_count INTEGER DEFAULT 0,
    is_active BOOLEAN DEFAULT true,
    expires_at TIMESTAMPTZ,               -- auto-expire old working memory
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_mem_working_tenant ON memory_working(tenant_id);
CREATE INDEX IF NOT EXISTS idx_mem_working_session ON memory_working(tenant_id, session_id);
CREATE INDEX IF NOT EXISTS idx_mem_working_active ON memory_working(tenant_id, is_active);

-- Memory Search Index — for hybrid vector + keyword search
-- Stores embeddings and searchable text for all memory types
CREATE TABLE IF NOT EXISTS memory_embeddings (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    memory_type VARCHAR(20) NOT NULL,     -- 'personal', 'task', 'tool', 'working'
    memory_id UUID NOT NULL,              -- FK to specific memory table
    content_text TEXT NOT NULL,           -- searchable text
    embedding_vector REAL[],              -- embedding vector (float array)
    embedding_model VARCHAR(100),         -- which model generated the embedding
    created_at TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_mem_embed_tenant ON memory_embeddings(tenant_id);
CREATE INDEX IF NOT EXISTS idx_mem_embed_type ON memory_embeddings(tenant_id, memory_type);
-- GIN index for text search
CREATE INDEX IF NOT EXISTS idx_mem_embed_text ON memory_embeddings USING gin(content_text gin_trgm_ops);

-- ════════════════════════════════════════════════
-- 3. HEARTBEAT / CRON SYSTEM
-- Agent tự thức dậy, scheduled tasks
-- ════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS heartbeat_configs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    enabled BOOLEAN DEFAULT false,
    interval_seconds INTEGER DEFAULT 1800,  -- default 30 minutes
    notify_channel VARCHAR(50),              -- 'telegram', 'discord', etc.
    notify_target VARCHAR(255),              -- chat_id, channel_id, etc.
    last_heartbeat TIMESTAMPTZ,
    next_heartbeat TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(tenant_id)
);

CREATE TABLE IF NOT EXISTS heartbeat_tasks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    task_name VARCHAR(200) NOT NULL,         -- 'check_email', 'rss_digest', etc.
    task_type VARCHAR(50) NOT NULL,          -- 'heartbeat', 'cron', 'once'
    cron_expression VARCHAR(100),            -- '0 */30 * * *' for cron tasks
    handler VARCHAR(200) NOT NULL,           -- skill/function to call
    config JSONB DEFAULT '{}',               -- task-specific config
    enabled BOOLEAN DEFAULT true,
    last_run TIMESTAMPTZ,
    last_result VARCHAR(20),                 -- 'success', 'failure', 'skipped'
    last_error TEXT,
    run_count INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_hb_tasks_tenant ON heartbeat_tasks(tenant_id);

CREATE TABLE IF NOT EXISTS heartbeat_runs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    task_id UUID REFERENCES heartbeat_tasks(id) ON DELETE CASCADE,
    task_name VARCHAR(200) NOT NULL,
    status VARCHAR(20) NOT NULL,             -- 'running', 'success', 'failure'
    result TEXT,                             -- output/summary
    error TEXT,
    duration_ms INTEGER,
    tokens_used INTEGER,
    started_at TIMESTAMPTZ DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_hb_runs_tenant ON heartbeat_runs(tenant_id);
CREATE INDEX IF NOT EXISTS idx_hb_runs_task ON heartbeat_runs(task_id);

-- ════════════════════════════════════════════════
-- 4. SKILLS SYSTEM
-- Hot-reload, marketplace, per-tenant skills
-- ════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS skills (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE, -- NULL = global skill
    name VARCHAR(200) NOT NULL,
    slug VARCHAR(200) NOT NULL,
    description TEXT DEFAULT '',
    version VARCHAR(20) DEFAULT '1.0.0',
    language VARCHAR(20) DEFAULT 'python',   -- 'python', 'javascript', 'shell'
    category VARCHAR(50),                    -- 'social', 'productivity', 'creative', 'research', 'desktop'
    source_code TEXT,                        -- inline code (small skills)
    file_path VARCHAR(500),                  -- path to skill file (large skills)
    entry_point VARCHAR(200) DEFAULT 'main', -- function to call
    input_schema JSONB DEFAULT '{}',         -- expected input parameters
    output_schema JSONB DEFAULT '{}',        -- expected output format
    dependencies JSONB DEFAULT '[]',         -- required packages
    enabled BOOLEAN DEFAULT true,
    is_builtin BOOLEAN DEFAULT false,        -- built-in vs user-created
    usage_count INTEGER DEFAULT 0,
    avg_duration_ms INTEGER DEFAULT 0,
    last_used TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(tenant_id, slug)
);
CREATE INDEX IF NOT EXISTS idx_skills_tenant ON skills(tenant_id);
CREATE INDEX IF NOT EXISTS idx_skills_category ON skills(category);
CREATE INDEX IF NOT EXISTS idx_skills_name ON skills USING gin(name gin_trgm_ops);

CREATE TABLE IF NOT EXISTS skill_runs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    skill_id UUID REFERENCES skills(id) ON DELETE SET NULL,
    skill_name VARCHAR(200) NOT NULL,
    input JSONB DEFAULT '{}',
    output JSONB DEFAULT '{}',
    status VARCHAR(20) NOT NULL,             -- 'running', 'success', 'failure'
    error TEXT,
    duration_ms INTEGER,
    tokens_used INTEGER,
    triggered_by VARCHAR(50),                -- 'user', 'heartbeat', 'cron', 'agent'
    started_at TIMESTAMPTZ DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_skill_runs_tenant ON skill_runs(tenant_id);

-- ════════════════════════════════════════════════
-- 5. AGENT TEAMS / WORKFLOW RUNS (Enhanced)
-- ════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS workflow_runs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    team_template VARCHAR(100),              -- template slug
    task_description TEXT NOT NULL,
    status VARCHAR(20) DEFAULT 'pending',    -- 'pending', 'running', 'done', 'failed'
    current_step INTEGER DEFAULT 0,
    total_steps INTEGER DEFAULT 0,
    result TEXT,
    error TEXT,
    total_tokens INTEGER DEFAULT 0,
    total_duration_ms INTEGER DEFAULT 0,
    started_at TIMESTAMPTZ DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_wf_runs_tenant ON workflow_runs(tenant_id);

CREATE TABLE IF NOT EXISTS workflow_steps (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    run_id UUID NOT NULL REFERENCES workflow_runs(id) ON DELETE CASCADE,
    step_index INTEGER NOT NULL,
    agent_name VARCHAR(100) NOT NULL,
    step_name VARCHAR(200) NOT NULL,
    status VARCHAR(20) DEFAULT 'pending',
    input TEXT,
    output TEXT,
    error TEXT,
    tokens_used INTEGER DEFAULT 0,
    duration_ms INTEGER DEFAULT 0,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_wf_steps_run ON workflow_steps(run_id);

-- ════════════════════════════════════════════════
-- 6. SEED DATA — Default admin + built-in skills
-- ════════════════════════════════════════════════

-- Default admin user (password: BizClaw@2026)
-- bcrypt hash for 'BizClaw@2026'
INSERT INTO users (id, email, password_hash, role, status)
VALUES (
    uuid_generate_v4(),
    'admin@bizclaw.vn',
    '$2b$12$LJ3m4ys.xZKKEfKMO.gVNOWqH/Kg7WRtGG3FLxD.TA0eN0/XBYgfi',
    'superadmin',
    'active'
) ON CONFLICT (email) DO NOTHING;

-- Built-in skills
INSERT INTO skills (name, slug, description, category, language, is_builtin, entry_point) VALUES
    ('Web Search', 'web-search', 'Search the web using various search engines', 'research', 'python', true, 'web_search'),
    ('RSS Feed Reader', 'rss-reader', 'Read and summarize RSS feeds', 'productivity', 'python', true, 'rss_reader'),
    ('Email Checker', 'email-checker', 'Check for new emails and summarize', 'productivity', 'python', true, 'check_email'),
    ('News Digest', 'news-digest', 'Compile daily tech/AI news digest', 'research', 'python', true, 'news_digest'),
    ('File Organizer', 'file-organizer', 'Organize files in a directory', 'desktop', 'python', true, 'organize_files'),
    ('Code Review', 'code-review', 'Review code for bugs, style, security', 'productivity', 'python', true, 'code_review'),
    ('Calendar Reminder', 'calendar-reminder', 'Check calendar and send reminders', 'productivity', 'python', true, 'calendar_reminder'),
    ('Social Digest', 'social-digest', 'Digest posts from social platforms', 'social', 'python', true, 'social_digest'),
    ('Doc Summarizer', 'doc-summarizer', 'Read and summarize documents', 'research', 'python', true, 'summarize_doc'),
    ('Data Analyzer', 'data-analyzer', 'Analyze CSV/JSON data files', 'research', 'python', true, 'analyze_data')
ON CONFLICT (tenant_id, slug) DO NOTHING;
