# Mama-Telegram-Hub Integration Test Cases
## Staging Verification Document

---

## Test Environment Setup

```bash
# Staging Environment Variables
export BIZCLAW_ENV="staging"
export TELEGRAM_BOT_TOKEN="test_token_123456:ABCdefGHIjklMNO"
export HUB_URL="http://localhost:3000"
export MAMA_INSTANCE_ID="mama-staging-001"
```

---

## Test Case 1: Mama nhận và xử lý request từ Telegram

### TC-001: Nhận message text thường từ Telegram

**Test Type:** Happy Path  
**Priority:** P0 - Critical

#### Input Data:
```json
{
  "update_id": 123456789,
  "message": {
    "message_id": 101,
    "from": {
      "id": 987654321,
      "is_bot": false,
      "first_name": "TestUser",
      "last_name": "Dev",
      "username": "testuser_dev"
    },
    "chat": {
      "id": 987654321,
      "type": "private"
    },
    "date": 1713264000,
    "text": "xin chào Mama"
  }
}
```

#### Expected Output:
```
✅ Message received successfully
✅ User identified: testuser_dev (ID: 987654321)
✅ Message parsed: "xin chào Mama"
✅ Response generated via LLM
✅ Telegram API called with reply
```

#### Debug Logs:
```
[2026-04-15T17:00:00.000Z] 📥 TELEGRAM_RECV update_id=123456789
[2026-04-15T17:00:00.050Z] 🔍 PARSING message_id=101 from=987654321
[2026-04-15T17:00:00.100Z] 👤 USER_LOOKUP user_id=987654321 username=testuser_dev
[2026-04-15T17:00:00.150Z] 🧠 LLM_REQUEST model=minimax-m2.7 prompt="xin chào Mama"
[2026-04-15T17:00:00.800Z] 🤖 LLM_RESPONSE tokens=45 latency=650ms
[2026-04-15T17:00:00.850Z] 📤 TELEGRAM_SEND chat_id=987654321 text="Chào bạn testuser_dev! Mình là Mama..."
[2026-04-15T17:00:00.900Z] ✅ COMPLETED duration=900ms
```

---

### TC-002: Nhận command /help từ Telegram

**Test Type:** Command Handling  
**Priority:** P1 - High

#### Input Data:
```json
{
  "update_id": 123456790,
  "message": {
    "message_id": 102,
    "from": { "id": 987654321, "is_bot": false, "first_name": "TestUser" },
    "chat": { "id": 987654321, "type": "private" },
    "date": 1713264060,
    "text": "/help"
  }
}
```

#### Expected Output:
```
✅ Command recognized: /help
✅ Help message generated with available commands
✅ Response sent to user
```

#### Debug Logs:
```
[2026-04-15T17:01:00.000Z] 📥 TELEGRAM_RECV command=/help
[2026-04-15T17:01:00.050Z] ⚡ COMMAND_MATCH pattern="/help" score=1.0
[2026-04-15T17:01:00.100Z] 📋 COMMAND_EXEC help_requested=true
[2026-04-15T17:01:00.150Z] 📤 TELEGRAM_SEND keyboard=[/help, /skills, /status, /settings]
```

---

### TC-003: Xử lý message với emoji và special characters

**Test Type:** Edge Case  
**Priority:** P2 - Medium

#### Input Data:
```json
{
  "update_id": 123456791,
  "message": {
    "message_id": 103,
    "from": { "id": 987654321, "is_bot": false },
    "chat": { "id": 987654321, "type": "private" },
    "date": 1713264120,
    "text": "Mama ơi 😍 Làm ơn giúp mình với 🥺"
  }
}
```

#### Expected Output:
```
✅ Message received with emoji preserved
✅ UTF-8 encoding correct
✅ Response maintains emoji context
```

#### Debug Logs:
```
[2026-04-15T17:02:00.000Z] 📥 TELEGRAM_RECV text_len=45 emoji_count=2
[2026-04-15T17:02:00.050Z] 🔤 ENCODING UTF-8 validated=true
[2026-04-15T17:02:00.100Z] 🧠 LLM_REQUEST model=minimax-m2.7
[2026-04-15T17:02:00.600Z] 🤖 LLM_RESPONSE contains_emoji=true
```

---

## Test Case 2: Mama gọi Hub lấy metadata skills

### TC-004: Fetch skills metadata thành công từ Hub

**Test Type:** Hub Integration  
**Priority:** P0 - Critical

#### Input Data:
```bash
# Mama internal request to Hub
GET /api/v1/skills/metadata?instance_id=mama-staging-001
Headers:
  X-Mama-Instance: mama-staging-001
  X-Request-ID: req-abc123
```

#### Expected Output:
```json
{
  "ok": true,
  "skills": [
    {
      "name": "content_writer",
      "version": "2.1.0",
      "description": "Tạo content marketing tự động",
      "parameters": {
        "topic": "string",
        "platform": "enum[zalo,tiktok,facebook]",
        "tone": "enum[formal,casual,friendly]"
      },
      "capabilities": ["text_generation", "hashtag_suggestion"],
      "metadata": {
        "category": "marketing",
        "tags": ["content", "social", "automation"]
      }
    },
    {
      "name": "scheduler",
      "version": "1.5.0",
      "description": "Lên lịch đăng bài tự động",
      "parameters": {
        "content_id": "string",
        "schedule_time": "datetime",
        "platform": "string"
      }
    }
  ],
  "total": 2,
  "cache_ttl": 300
}
```

#### Debug Logs:
```
[2026-04-15T17:03:00.000Z] 🔗 HUB_REQUEST method=GET path=/api/v1/skills/metadata
[2026-04-15T17:03:00.050Z] 🔐 AUTH header=X-Mama-Instance validated=true
[2026-04-15T17:03:00.100Z] 📡 HUB_RESPONSE status=200 latency=100ms
[2026-04-15T17:03:00.150Z] 💾 CACHE_UPDATE key=skills_metadata ttl=300s
[2026-04-15T17:03:00.200Z] ✅ SKILLS_LOADED count=2
```

---

### TC-005: Fetch skills với filter theo category

**Test Type:** Hub Integration  
**Priority:** P1 - High

#### Input Data:
```bash
GET /api/v1/skills/metadata?instance_id=mama-staging-001&category=marketing
```

#### Expected Output:
```json
{
  "ok": true,
  "skills": [
    {
      "name": "content_writer",
      "category": "marketing"
    },
    {
      "name": "analytics",
      "category": "marketing"
    }
  ],
  "total": 2,
  "filtered_by": "category=marketing"
}
```

#### Debug Logs:
```
[2026-04-15T17:04:00.000Z] 🔗 HUB_REQUEST path=/api/v1/skills/metadata category=marketing
[2026-04-15T17:04:00.080Z] 🔍 FILTER category=marketing applied=true
[2026-04-15T17:04:00.120Z] 📡 HUB_RESPONSE skills_count=2
```

---

## Test Case 3: Execute skills sau khi fetch từ Hub

### TC-006: Execute content_writer skill thành công

**Test Type:** Skill Execution  
**Priority:** P0 - Critical

#### Input Data:
```json
{
  "skill_name": "content_writer",
  "version": "2.1.0",
  "parameters": {
    "topic": "khuyến mãi mùa hè 2026",
    "platform": "zalo",
    "tone": "friendly"
  },
  "context": {
    "user_id": 987654321,
    "chat_id": 987654321,
    "message_id": 104
  }
}
```

#### Expected Output:
```json
{
  "ok": true,
  "result": {
    "content": "🌞 MÙA HÈ RỰC RỠ - KHUYẾN MÃI CỰC SỐNG!\n\nGiảm đến 50% cho tất cả sản phẩm...\n\n#mùahè #khuyếnmãi #sale",
    "hashtags": ["mùahè", "khuyếnmãi", "sale", "cựcsống"],
    "suggested_time": "2026-04-16T09:00:00Z",
    "platform_compatible": true
  },
  "execution_time_ms": 1250,
  "model_used": "MiniMax-M2.7"
}
```

#### Debug Logs:
```
[2026-04-15T17:05:00.000Z] 🎯 SKILL_EXECUTE name=content_writer version=2.1.0
[2026-04-15T17:05:00.050Z] 📋 PARAM_VALIDATE topic=khuyến mãi... platform=zalo tone=friendly
[2026-04-15T17:05:00.100Z] 🧠 LLM_REQUEST model=minimax-m2.7 tokens_estimate=500
[2026-04-15T17:05:01.200Z] 🤖 LLM_RESPONSE tokens=180 latency=1100ms
[2026-04-15T17:05:01.250Z] ✅ SKILL_COMPLETED result_size=180bytes
[2026-04-15T17:05:01.300Z] 📤 TELEGRAM_SEND content_generated=true
```

---

### TC-007: Execute scheduler skill sau khi content được tạo

**Test Type:** Skill Chaining  
**Priority:** P1 - High

#### Input Data:
```json
{
  "skill_name": "scheduler",
  "version": "1.5.0",
  "parameters": {
    "content_id": "content-uuid-12345",
    "schedule_time": "2026-04-16T09:00:00Z",
    "platform": "zalo"
  }
}
```

#### Expected Output:
```json
{
  "ok": true,
  "result": {
    "job_id": "job-67890",
    "scheduled_at": "2026-04-16T09:00:00Z",
    "status": "scheduled",
    "platform": "zalo",
    "retry_count": 0
  }
}
```

#### Debug Logs:
```
[2026-04-15T17:06:00.000Z] 🎯 SKILL_EXECUTE name=scheduler version=1.5.0
[2026-04-15T17:06:00.050Z] 📅 SCHEDULE_VALIDATE time=2026-04-16T09:00:00Z valid=true
[2026-04-15T17:06:00.100Z] 💾 DB_INSERT job_id=job-67890
[2026-04-15T17:06:00.150Z] ⏰ CRON_REGISTER job=67890 next_run=1713258000
[2026-04-15T17:06:00.200Z] ✅ SCHEDULED job_id=job-67890
```

---

## Test Case 4: Error Handling

### TC-008: Hub timeout - Hub không phản hồi

**Test Type:** Error Handling  
**Priority:** P0 - Critical

#### Input Data:
```bash
# Simulate Hub timeout by stopping Hub service
# Or using network isolation
GET /api/v1/skills/metadata
Timeout: 5000ms
```

#### Expected Behavior:
```
1. Mama chờ response trong 5 giây
2. Sau timeout, Mama sử dụng cached skills (nếu có)
3. Nếu không có cache, trả về lỗi có structure
4. User nhận được thông báo lỗi thân thiện
```

#### Expected Output:
```json
{
  "ok": false,
  "error": {
    "code": "HUB_TIMEOUT",
    "message": "Không thể kết nối đến Hub. Vui lòng thử lại sau.",
    "retry_after_ms": 30000,
    "fallback": "cached_skills"
  }
}
```

#### Debug Logs:
```
[2026-04-15T17:07:00.000Z] 🔗 HUB_REQUEST method=GET path=/api/v1/skills/metadata
[2026-04-15T17:07:05.000Z] ⏰ TIMEOUT after=5000ms
[2026-04-15T17:07:05.050Z] 💾 CACHE_CHECK key=skills_metadata
[2026-04-15T17:07:05.100Z] ✅ CACHE_HIT ttl_remaining=250s
[2026-04-15T17:07:05.150Z] ⚠️ DEGRADED_MODE active=true fallback=cached
[2026-04-15T17:07:05.200Z] 🔄 RETRY_SCHEDULED delay=30000ms
[2026-04-15T17:07:05.250Z] 📤 USER_NOTIFY "Đang gặp sự cố kết nối..."
```

---

### TC-009: Hub trả về invalid JSON

**Test Type:** Error Handling  
**Priority:** P1 - High

#### Input Data:
```bash
# Simulate Hub returning malformed response
GET /api/v1/skills/metadata
Response: "{ invalid json {"
```

#### Expected Output:
```json
{
  "ok": false,
  "error": {
    "code": "INVALID_RESPONSE",
    "message": "Hub trả về dữ liệu không hợp lệ.",
    "details": "JSON parse error at position 2",
    "fallback": "cached_skills"
  }
}
```

#### Debug Logs:
```
[2026-04-15T17:08:00.000Z] 🔗 HUB_REQUEST method=GET
[2026-04-15T17:08:00.080Z] 📡 HUB_RESPONSE status=200 size=18bytes
[2026-04-15T17:08:00.100Z] 🔍 PARSE_JSON raw="{ invalid json {"
[2026-04-15T17:08:00.150Z] ❌ JSON_ERROR "expected value at line 1 column 2"
[2026-04-15T17:08:00.200Z] ⚠️ FALLBACK_ACTIVATED cached_skills
```

---

### TC-010: Hub trả về 403 Forbidden

**Test Type:** Error Handling  
**Priority:** P1 - High

#### Input Data:
```bash
GET /api/v1/skills/metadata
Headers:
  X-Mama-Instance: invalid-instance-id
Response: HTTP 403 Forbidden
```

#### Expected Output:
```json
{
  "ok": false,
  "error": {
    "code": "UNAUTHORIZED",
    "message": "Mama không có quyền truy cập Hub.",
    "instance_id": "invalid-instance-id"
  }
}
```

#### Debug Logs:
```
[2026-04-15T17:09:00.000Z] 🔗 HUB_REQUEST instance=invalid-instance-id
[2026-04-15T17:09:00.050Z] 🔐 AUTH_FAILED reason="Invalid instance ID"
[2026-04-15T17:09:00.100Z] ❌ HTTP_403 status=403
[2026-04-15T17:09:00.150Z] 📤 USER_ERROR "Không có quyền truy cập Hub"
```

---

### TC-011: Skills không tìm thấy

**Test Type:** Error Handling  
**Priority:** P2 - Medium

#### Input Data:
```json
{
  "skill_name": "nonexistent_skill",
  "parameters": {}
}
```

#### Expected Output:
```json
{
  "ok": false,
  "error": {
    "code": "SKILL_NOT_FOUND",
    "message": "Skill 'nonexistent_skill' không tồn tại.",
    "available_skills": ["content_writer", "scheduler", "analytics", "seo_analyzer"]
  }
}
```

#### Debug Logs:
```
[2026-04-15T17:10:00.000Z] 🎯 SKILL_LOOKUP name=nonexistent_skill
[2026-04-15T17:10:00.050Z] 🔍 MATCH_SCORE=0.0
[2026-04-15T17:10:00.100Z] ❌ SKILL_NOT_FOUND name=nonexistent_skill
[2026-04-15T17:10:00.150Z] 💡 SUGGESTIONS available=4
```

---

## Test Case 5: Integration Test - Full Flow

### TC-012: End-to-End - User yêu cầu tạo content và lên lịch

**Test Type:** Integration  
**Priority:** P0 - Critical

#### Input Data (Telegram Message):
```
User gửi: "Mama ơi, tạo cho mình một bài post về khuyến mãi mùa hè và lên lịch đăng vào 9h sáng mai trên Zalo nhé"
```

#### Full Expected Flow:
```
1. Telegram → Mama: Message received
2. Mama → Hub: Fetch skills metadata
3. Hub → Mama: Skills list returned
4. Mama → LLM: Generate content about summer promotion
5. LLM → Mama: Content generated
6. Mama → Scheduler: Schedule post for 9AM tomorrow
7. Scheduler → Mama: Job scheduled
8. Mama → Telegram: Confirmation sent to user
```

#### Expected Final Output:
```
🌟 Đã tạo xong content và lên lịch thành công!

📝 Nội dung:
"MÙA HÈ RỰC RỠ - KHUYẾN MÃI CỰC SỐNG! 
Giảm đến 50% cho tất cả sản phẩm..."

📅 Lịch đăng: 09:00 - 16/04/2026
📱 Nền tảng: Zalo OA
🆔 Job ID: job-67890

Bạn có muốn chỉnh sửa gì không?"
```

#### Debug Logs:
```
[2026-04-15T17:11:00.000Z] 📥 TELEGRAM_RECV text="Mama ơi, tạo cho mình..."
[2026-04-15T17:11:00.100Z] 🔗 HUB_FETCH /api/v1/skills/metadata
[2026-04-15T17:11:00.200Z] ✅ HUB_RESPONSE skills=4 latency=100ms
[2026-04-15T17:11:00.300Z] 🧠 INTENT_DETECT action=create_and_schedule confidence=0.95
[2026-04-15T17:11:00.400Z] 🎯 SKILL_SELECT name=content_writer
[2026-04-15T17:11:01.800Z] ✅ CONTENT_GENERATED tokens=250
[2026-04-15T17:11:01.900Z] 🎯 SKILL_SELECT name=scheduler
[2026-04-15T17:11:02.000Z] ✅ JOB_SCHEDULED job_id=job-67890
[2026-04-15T17:11:02.100Z] 📤 TELEGRAM_SEND confirmation=true
[2026-04-15T17:11:02.200Z] ✅ E2E_COMPLETED total_duration=2200ms
```

---

## Staging Test Execution Checklist

```bash
# Pre-flight Check
- [ ] Telegram Bot Token configured
- [ ] Hub URL accessible
- [ ] MiniMax API key valid
- [ ] Database connection healthy

# Test Execution Order
1. TC-001: Basic message handling
2. TC-002: Command handling  
3. TC-004: Hub skills fetch
4. TC-006: Content writer skill
5. TC-007: Scheduler skill
6. TC-008: Hub timeout handling
7. TC-009: Invalid JSON handling
8. TC-012: Full E2E flow

# Success Criteria
- All P0 tests must pass (100%)
- P1 tests must pass (>90%)
- P2 tests informational (pass rate >70%)
- No data corruption or state leaks
```

---

## Production Readiness Checklist

```bash
# Before Production Deploy
- [ ] All P0 tests passing in staging
- [ ] Error handling verified for all error codes
- [ ] Circuit breaker tested
- [ ] Rate limiting verified
- [ ] Logging structured correctly
- [ ] Monitoring dashboards updated
- [ ] Alerting configured
- [ ] Rollback procedure documented
```

---

*Document Version: 1.0*  
*Last Updated: 2026-04-15*  
*Test Environment: Staging*
