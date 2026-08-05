# BizClaw Agent

> Trả lời tin nhắn Zalo/Messenger bằng AI chạy **hoàn toàn trên điện thoại**.
> Không server, không API key, không gửi tin nhắn khách ra ngoài.

**v2.0.0** · Android 12+ · Gemma 4 qua LiteRT-LM

---

## Nó làm gì

Khách nhắn tin → app đọc thông báo → Gemma 4 soạn câu trả lời dựa trên thông tin cửa hàng anh đã nạp → gửi lại vào đúng cuộc trò chuyện đó.

```
Khách nhắn Zalo/Messenger
        │
        ▼
MessageListenerService ──── đọc tên người gửi + nội dung từ thông báo
        │
        ▼
ReplyAgent ──── ghép prompt: giọng điệu + tài liệu cửa hàng khớp câu hỏi
        │
        ▼
GemmaEngine ──── Gemma 4 chạy on-device (GPU, fallback CPU)
        │
        ├─► thiếu dữ liệu ──► chờ anh duyệt (luôn luôn)
        └─► đủ dữ liệu ────► gửi thẳng hoặc chờ duyệt, tuỳ cài đặt
        │
        ▼
ReplySender ──── bắn vào ô trả lời nhanh của thông báo
```

## Vì sao đi qua thông báo, không dùng Accessibility

Trả lời qua **RemoteInput của thông báo** — đúng cơ chế đồng hồ thông minh dùng để nhắn tin. Đây là API công khai, ổn định: Zalo và Messenger đổi giao diện liên tục nhưng nút trả lời nhanh đó buộc phải giữ nguyên.

Accessibility thì đọc được cả màn hình, nhưng vỡ mỗi lần app chat cập nhật giao diện, tốn pin, và cần màn hình sáng.

**Đổi lại, có 3 giới hạn thật:**

| Giới hạn | Hệ quả |
|---|---|
| Chỉ thấy tin **có sinh thông báo** | Tắt thông báo Zalo là agent mù |
| Thông báo bị đóng thì hết trả lời được | Anh mở app đọc trước → tab Hộp thư báo "Hết hạn" |
| Không đọc được lịch sử chat | Mỗi tin nhắn xử lý độc lập, không nhớ ngữ cảnh trước đó |

## Chống bịa

Gemma 4 E4B không có dữ liệu sẽ **bịa giá, bịa phí ship, bịa chính sách bảo hành** — và câu đó bay thẳng tới khách đang trả tiền.

Nên agent bị ràng buộc:

1. Chỉ được dùng tài liệu trong tab **Cửa hàng**, và chỉ những tài liệu khớp câu hỏi mới được đưa vào prompt.
2. Thiếu dữ liệu → bắt buộc trả lời "để em kiểm tra rồi báo lại" và **tự chuyển sang chờ duyệt**, kể cả khi đã bật tự gửi.
3. Cấm hứa giảm giá, hoàn tiền, đền bù ngoài tài liệu.
4. `temperature = 0.3` — cùng câu hỏi cho cùng câu trả lời.

Chưa nạp tài liệu nào thì agent gần như luôn chuyển việc lại cho anh. Đó là chủ ý.

## Model

| | Gemma 4 E2B | Gemma 4 E4B |
|---|---|---|
| Tải về | 2.58 GB | 3.65 GB |
| RAM đỉnh | ~1.7 GB | ~3.3 GB |
| Máy phù hợp | RAM ≥ 4 GB | RAM ≥ 6 GB |

App tự chọn bản lớn nhất mà máy chịu được, đổi tay được ở tab Trạng thái. Model tải bằng DownloadManager nên tắt màn hình vẫn chạy, rớt Wi-Fi thì tự nối lại.

## Cài đặt

Xem [docs/setup.md](docs/setup.md) — build, ký APK, các bước bật trên máy.

Tóm tắt: tải model → cấp quyền đọc thông báo → nạp thông tin cửa hàng → bật agent → duyệt vài chục tin đầu → khi nào yên tâm mới bật tự gửi.

## Cấu trúc

```
app/src/main/java/vn/bizclaw/agent/
├── llm/          GemmaEngine · ModelCatalog · ModelDownloader
├── messaging/    MessageListenerService · ReplySender · SupportedApps
├── agent/        ReplyAgent · PromptBuilder · AgentService · BootReceiver
├── data/         Settings · ExchangeStore · KnowledgeStore · Models
└── ui/           HomeScreen · InboxScreen · KnowledgeScreen · Common
```

| | |
|---|---|
| Ngôn ngữ | Kotlin 2.2, Jetpack Compose, Material 3 |
| Inference | LiteRT-LM 0.15.0 (`com.google.ai.edge.litertlm`) |
| Lưu trữ | SharedPreferences + kotlinx.serialization |
| Min SDK | 31 (Android 12) · Target 35 · arm64-v8a |
| Kích thước | debug ~77 MB · release ~24 MB |
| Quyền | 7, không có `QUERY_ALL_PACKAGES` |

## Trạng thái

**Đã kiểm chứng:**

- `./gradlew :app:assembleDebug` ✅
- `./gradlew :app:assembleRelease` ✅ (24.4 MB, keystore truyền lúc build, không nằm trong repo)
- `./gradlew :app:testDebugUnitTest` ✅ 5/5
- Chữ ký API LiteRT-LM đối chiếu trực tiếp với AAR 0.15.0

**Chưa kiểm chứng — cần máy thật:**

- Gemma 4 nạp và sinh chữ trên thiết bị
- Zalo bản hiện tại có nút trả lời nhanh trong thông báo hay không (Messenger thì có)
- Chất lượng tiếng Việt của E4B cho chăm sóc khách hàng
- Tốc độ và mức tốn pin khi chạy cả ngày

## Riêng tư

Tin nhắn khách không rời khỏi máy. Model chạy local, không có backend, không telemetry. Thứ duy nhất đi ra Internet là lần tải model từ Hugging Face.

## Không phân phối qua Play Store

App đọc nội dung thông báo của app khác — Google Play sẽ không duyệt cho mục đích này. Đây là app sideload, cài bằng APK.

---

## Nguồn

- [LiteRT-LM Android — Google AI Edge](https://ai.google.dev/edge/litert-lm/android)
- [LiteRT-LM Kotlin API](https://github.com/google-ai-edge/LiteRT-LM/blob/main/docs/api/kotlin/getting_started.md)
- [litert-community/gemma-4-E4B-it-litert-lm](https://huggingface.co/litert-community/gemma-4-E4B-it-litert-lm)
- [Gemma 4 trên LiteRT-LM](https://developers.google.com/edge/litert-lm/models/gemma-4)
