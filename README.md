# BizClaw Agent

> Trả lời tin nhắn Zalo/Messenger, tư vấn, báo giá và **chốt đơn** bằng AI.
> Chạy on-device miễn phí, hoặc gắn API key Claude/OpenAI khi cần model mạnh.

**v2.1.0** · Android 12+ · Gemma 4 on-device, hoặc Claude / OpenAI qua API key

---

## Nó làm gì

Khách nhắn tin → app đọc thông báo → model soạn câu trả lời dựa trên bảng giá và chính sách anh đã nạp → gửi lại vào đúng cuộc trò chuyện đó. Khách chốt đơn thì agent bóc tách thành đơn hàng chờ anh duyệt.

```
Khách nhắn Zalo/Messenger
        │
        ▼
MessageListenerService ──── đọc tên người gửi + nội dung từ thông báo
        │
        ▼
ReplyAgent ──── chọn model theo cài đặt
        │
        ├─ Claude / OpenAI ──► agent gọi tool:
        │                      tra_cuu_san_pham · tra_cuu_chinh_sach · tao_don_hang
        │
        └─ Gemma on-device ──► nhét sẵn bảng giá + chính sách vào prompt,
                               bóc tách đơn bằng constrained JSON
        │
        ▼
        ├─► thiếu dữ liệu / có đơn ──► chờ anh duyệt (luôn luôn)
        └─► đủ dữ liệu ─────────────► gửi thẳng hoặc chờ duyệt, tuỳ cài đặt
        │
        ▼
ReplySender ──── bắn vào ô trả lời nhanh của thông báo
```

## Chọn model

| | Gemma 4 (trên máy) | Claude / OpenAI |
|---|---|---|
| Tin nhắn khách | Không rời khỏi điện thoại | Gửi lên cloud của nhà cung cấp |
| Chi phí | Miễn phí | Trả theo token |
| Tool calling | ❌ (4B gọi tool không đủ tin cậy) | ✅ |
| Chốt đơn | Bóc tách JSON 2 lượt | Agent tự gọi `tao_don_hang` |
| Cần mạng | Không | Có |

Đổi ở tab **Model**. Chọn cloud mà chưa nhập key thì app tự chạy tạm bằng Gemma chứ không đứng im.

## Tool calling

Với Claude/OpenAI, mọi con số agent nói ra đều phải đi qua tool — app trả số, model chỉ quyết định lúc nào cần hỏi:

| Tool | Làm gì |
|---|---|
| `tra_cuu_san_pham` | Giá + tồn kho từ bảng giá |
| `tra_cuu_chinh_sach` | Phí ship, bảo hành, đổi trả từ tài liệu cửa hàng |
| `tao_don_hang` | Tạo đơn nháp — chỉ khi đủ tên, sđt, địa chỉ, sản phẩm |

Schema đều bật `strict` + `additionalProperties: false`, và có unit test chặn schema sai ngay lúc build. Tool báo "không có dữ liệu" thì agent bắt buộc chuyển việc cho chủ shop chứ không được đoán.

## Chủ động nhắn trước

Trả lời qua thông báo chỉ chạy được **khi khách nhắn trước**. Để hỏi thăm sau bán, app dùng Accessibility mở Zalo/Messenger, tìm khách, gõ tin và gửi.

Chỉ chạy khi anh bấm **Nhắn ngay** ở tab Đơn hàng — agent không tự ý nhắn ai. Cách này bám vào giao diện Zalo/Messenger nên có thể hỏng khi hai app đó cập nhật; hỏng thì app báo đúng bước lỗi ("không tìm thấy ô tìm kiếm") để anh gửi tay.

## Vì sao trả lời đi qua thông báo

Đường **trả lời** dùng RemoteInput của thông báo — đúng cơ chế đồng hồ thông minh dùng để nhắn tin. Đây là API công khai, ổn định: Zalo và Messenger đổi giao diện liên tục nhưng nút trả lời nhanh đó buộc phải giữ nguyên.

Accessibility chỉ dùng cho đường **chủ động nhắn trước** ở trên, vì đó là việc thông báo không làm được. Nó vỡ mỗi lần app chat cập nhật giao diện, nên không dùng cho luồng trả lời hàng ngày.

**Đổi lại, đường trả lời có 3 giới hạn thật:**

| Giới hạn | Hệ quả |
|---|---|
| Chỉ thấy tin **có sinh thông báo** | Tắt thông báo Zalo là agent mù |
| Thông báo bị đóng thì hết trả lời được | Anh mở app đọc trước → tab Hộp thư báo "Hết hạn" |
| Không đọc được lịch sử chat | Mỗi tin nhắn xử lý độc lập, không nhớ ngữ cảnh trước đó |

## Chống bịa

Model nào cũng sẽ **bịa giá, bịa phí ship, bịa chính sách bảo hành** nếu không có dữ liệu — và câu đó bay thẳng tới khách đang trả tiền.

Nên agent bị ràng buộc:

1. Chỉ được dùng bảng giá và tài liệu trong tab **Cửa hàng**. Cloud thì bắt buộc gọi tool để lấy; on-device thì chỉ tài liệu khớp câu hỏi mới vào prompt.
2. Thiếu dữ liệu → bắt buộc trả lời "để em kiểm tra rồi báo lại" và **tự chuyển sang chờ duyệt**, kể cả khi đã bật tự gửi.
3. Đơn hàng nào cũng là bản nháp, luôn chờ anh xác nhận. Xác nhận rồi mới trừ tồn kho.
4. Cấm hứa giảm giá, hoàn tiền, đền bù ngoài tài liệu.
5. Sản phẩm hết hàng thì phải nói thật là hết, không nhận đơn.

Chưa nạp tài liệu nào thì agent gần như luôn chuyển việc lại cho anh. Đó là chủ ý.

## Model on-device

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
├── llm/          LlmProvider · AnthropicProvider · OpenAiProvider · OnDeviceProvider
│                 ProviderRegistry · ApiKeyStore · GemmaEngine · ModelDownloader
├── messaging/    MessageListenerService · ReplySender · ChatAutomationService
│                 ProactiveSender · SupportedApps
├── agent/        ReplyAgent · AgentTools · PromptBuilder · OrderExtractor
│                 AgentService · FollowUpWorker · BootReceiver
├── data/         Settings · Catalog · OrderStore · KnowledgeStore · ExchangeStore
└── ui/           Home · Inbox · Orders · Knowledge · Provider · Common
```

| | |
|---|---|
| Ngôn ngữ | Kotlin 2.2, Jetpack Compose, Material 3 |
| Inference | LiteRT-LM 0.15.0 on-device · Anthropic Messages API · OpenAI Chat Completions |
| Lưu trữ | SharedPreferences + kotlinx.serialization; API key mã hoá (EncryptedSharedPreferences) |
| Min SDK | 31 (Android 12) · Target 35 · arm64-v8a |
| Kích thước | debug ~88 MB · release ~24 MB |
| Quyền | 7, không có `QUERY_ALL_PACKAGES` |

## Trạng thái

**Đã kiểm chứng:**

- `./gradlew :app:assembleDebug` ✅
- `./gradlew :app:assembleRelease` ✅ (24.4 MB, keystore truyền lúc build, không nằm trong repo)
- `./gradlew :app:testDebugUnitTest` ✅ 20/20
- Chữ ký API LiteRT-LM đối chiếu trực tiếp với AAR 0.15.0

**Chưa kiểm chứng — cần máy thật:**

- Gemma 4 nạp và sinh chữ trên thiết bị
- Zalo bản hiện tại có nút trả lời nhanh trong thông báo hay không (Messenger thì có)
- Chất lượng tiếng Việt của E4B cho chăm sóc khách hàng
- Tốc độ và mức tốn pin khi chạy cả ngày
- Tool calling thật với Claude/OpenAI (schema đã test, đường mạng thì chưa)
- Accessibility có bám đúng giao diện Zalo/Messenger bản hiện tại không

## Riêng tư

Ở chế độ **Gemma on-device**: tin nhắn khách không rời khỏi máy, không backend, không telemetry — thứ duy nhất đi ra Internet là lần tải model.

Ở chế độ **Claude/OpenAI**: nội dung tin nhắn khách được gửi tới nhà cung cấp đó. Đây là đánh đổi có ý thức để lấy tool calling và chất lượng cao hơn — app nói rõ điều này ngay trên màn hình chọn model. API key lưu mã hoá và không bao giờ đi vào prompt.

## Không phân phối qua Play Store

App đọc nội dung thông báo của app khác — Google Play sẽ không duyệt cho mục đích này. Đây là app sideload, cài bằng APK.

---

## Nguồn

- [LiteRT-LM Android — Google AI Edge](https://ai.google.dev/edge/litert-lm/android)
- [LiteRT-LM Kotlin API](https://github.com/google-ai-edge/LiteRT-LM/blob/main/docs/api/kotlin/getting_started.md)
- [litert-community/gemma-4-E4B-it-litert-lm](https://huggingface.co/litert-community/gemma-4-E4B-it-litert-lm)
- [Gemma 4 trên LiteRT-LM](https://developers.google.com/edge/litert-lm/models/gemma-4)
