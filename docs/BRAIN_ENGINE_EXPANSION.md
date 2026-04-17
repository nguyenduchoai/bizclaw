# Brain Engine Expansion — Modular AI Orchestration

Tài liệu này hướng dẫn chi tiết về kiến trúc Modular AI mới của BizClaw, cho phép tích hợp linh hoạt giữa Local LLM và các Cloud Provider (OpenAI, Anthropic, Gemini, DeepSeek).

## 1. Kiến trúc Modular (Brain Router)

Hệ thống sử dụng một Unified Router ([router.rs](file:///Users/digits/Github/bizclaw/crates/bizclaw-providers/src/router.rs)) để điều phối các yêu cầu AI.

### Các thành phần chính:
- **Local Provider**: Chạy các mô hình GGUF thông qua `bizclaw-brain`. Ưu tiên cho các tác vụ nhạy cảm hoặc cần chi phí bằng 0.
- **Cloud Providers**: Tích hợp OpenAI (gpt-4o, o3-mini), Anthropic (Claude 3.5), Gemini (2.0 Flash).
- **Rate Limiting**: Quản lý lưu lượng bằng thuật toán Token Bucket (RPM/TPM).
- **Circuit Breaker**: Tự động ngắt kết nối khi Provider lỗi (Health -> Degraded -> Unhealthy).

## 2. Chiến lược Routing (Routing Strategies)

Người dùng có thể cấu hình chiến lược điều phối trong `bizclaw.toml`:

- `PriorityBased`: Chọn provider có độ ưu tiên cao nhất.
- `LeastLatency`: Chọn provider có tốc độ phản hồi nhanh nhất dựa trên lịch sử.
- `CostAware`: Chọn provider có chi phí thấp nhất cho model đang chọn.
- `RoundRobin`: Chia đều tải giữa các provider lành mạnh.

## 3. Bảo mật & Quyền riêng tư (Data Privacy)

Hệ thống tích hợp lớp **Secret Redactor**:
- Tự động nhận diện và ẩn danh các thông tin nhạy cảm (API Keys, Passwords, PII) trước khi gửi dữ liệu ra Cloud.
- Đảm bảo tuân thủ các quy định bảo mật cho SME.

## 4. Benchmark & So sánh Chi phí (USD/1M tokens)

| Provider | Model | Prompt Cost | Completion Cost | Strength |
| :--- | :--- | :--- | :--- | :--- |
| **OpenAI** | o3-mini | $1.10 | $4.40 | Reasoning nhanh, Code |
| **Anthropic**| Claude 3.5 Sonnet | $3.00 | $15.00 | Viết lách, Logic cao |
| **Anthropic**| Claude 4.6 (NextGen) | $15.00 | $75.00 | Sức mạnh xử lý cao nhất |
| **Gemini** | 2.0 Flash | $0.10 | $0.40 | Context cực lớn, Rẻ |
| **DeepSeek** | V3 | $0.14 | $0.28 | SME Lean Model (Tối ưu nhất) |
| **Local** | Qwen3.5-4B-Neo | $0.00 | $0.00 | Privacy tuyệt đối, 0 cost |

## 5. Hướng dẫn cấu hình cho SME

Thêm vào file `bizclaw.toml`:

```toml
[brain]
enabled = true
mode = "CloudFirst" # Hoặc: LocalOnly, LocalFirst, CloudOnly

# Cấu hình "đồng nghiệp" AI đám mây
[[brain.providers]]
name = "openai"
priority = 100
models = ["gpt-4o-mini"]
rate_limit = { requests_per_minute = 100, tokens_per_minute = 100000 }

[[brain.providers]]
name = "deepseek"
priority = 120 # Ưu tiên DeepSeek hơn vì rẻ
models = ["deepseek-chat"]

[brain.routing]
strategy = "CostAware"
local_fallback = true # Dùng Local LLM nếu Cloud sập
```

## 6. Tính năng Media (bizclaw-media)

Hệ thống hiện hỗ trợ tạo hình ảnh và video thông qua MiniMax API:
- **Image Generation**: Text-to-Image.
- **Video Generation**: Tạo video từ prompt (hỗ trợ polling trạng thái).
