# BizClaw: Two-Model Architectural Strategy

## Overview
This document officially records the dual-architecture strategy for the BizClaw Enterprise Platform, derived from our comprehensive analysis of modern open-source AI agent frameworks (GoClaw, CrawBot, OpenFang, SkyClaw, etc.).

We are splitting the product line into two distinct models to better serve different scales of customers:
1. **BizClaw Single Tenant (Local/On-Premise)**
2. **BizClaw Cloud (Multi-Tenant Enterprise)**

---

## 1. BizClaw Single Tenant (Inspired by CrawBot)

**Target Audience:** Small to Medium Enterprises (SMEs), individual consultants, and non-technical business owners.
**Core Philosophy:** Zero-Configuration Barrier & Cost Optimization.

### Key Architectural Decisions:
*   **Zero-Config UI:** Hide the complexity of CLI and environment variables. Provide an intuitive Setup Wizard.
*   **Native OS Security:** Store sensitive configs not in plaintext `.env` files, but directly inside the operating system's Keychain (similar to CrawBot's Electron implementation).
*   **API Cost Optimization (Anthropic Bypass):** 
    *   Implementing the `Web Session Hijacking` trick: using a headless browser/Chrome DevTools Protocol (CDP) to extract user session cookies (`sessionKey`) from `claude.ai`.
    *   Bypassing strict API calls by injecting the `anthropic-client-platform: web_claude_ai` header to make requests look like they come from the official Claude Web App.
    *   This allows SMEs to utilize their existing, potentially unlimited $20/mo Claude Pro subscriptions without paying per-token API usage.
*   **Omni-Channel via Web Automation (Zalo/FB on PC/VPS):** 
    *   Instead of relying on restricted Zalo OA APIs or FB Graph API, the system uses `CdpClient` (Chrome DevTools Protocol) to control a local Chromium browser window.
    *   Users log into Web Zalo (`chat.zalo.me`) or Web Messenger normally. The Agent reads the DOM and dispatches typing events identical to human behavior.
    *   (*Note: This applies to PC/VPS deployments. The Android client operates natively via `RemoteInput` and Accessibility Services.*)

## 2. BizClaw Cloud (Inspired by GoClaw & OpenFang)

**Target Audience:** Large Enterprises, B2B SaaS customers requiring strict compliance and isolation.
**Core Philosophy:** Security in Depth, Multi-Tenancy, and High Reliability.

### Key Architectural Decisions:
*   **Multi-tenant PostgreSQL Core:** Move entirely away from file-based config. Store all Tenant, Agent, and Workspace configurations in PostgreSQL with AES-256-GCM encryption for all secrets/API keys.
*   **5-Layer Security Framework:** Implement strict Role-Based Access Control (RBAC):
    `GatewayAuth` -> `TenantPolicy` -> `AgentPermissions` -> `ChannelRestrictions` -> `OwnerActions`
*   **Agent Teams & Orchestration:** Introduce a Shared Task Board where Autonomous Hands (Agents) can act synchronously or asynchronously, delegating sub-tasks across the team (e.g., Sales Agent ⭢ Inventory Agent).
*   **Code Isolation:** Use WASM Sandboxing (inspired by OpenFang) to safely execute dynamically generated logic and external tools without exposing the host OS to Remote Code Execution (RCE).
*   **Memory Management:** Implement Finite Brain / Lambda memory to control token expenditure and summarize long conversational graphs.
