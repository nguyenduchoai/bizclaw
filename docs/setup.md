# BizClaw Agent — Build & chạy trên điện thoại

## Yêu cầu

| | |
|---|---|
| JDK | 17+ (máy này đang có Java 8 trên PATH → phải trỏ `JAVA_HOME`) |
| Android SDK | platform 35, build-tools 35.x |
| Điện thoại | Android 12+, arm64, RAM ≥ 4 GB (E2B) hoặc ≥ 6 GB (E4B) |
| Dung lượng trống | ≥ 3 GB cho model (E2B) hoặc ≥ 4 GB (E4B) |

App tự chọn model theo RAM máy: Xiaomi 17 Ultra 16 GB → mặc định **Gemma 4 E4B**.

Repo **không** hardcode đường dẫn JDK. Trên máy chưa có JDK 17 dùng bản đi kèm Android Studio:

```bash
export JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home"
export ANDROID_HOME="$HOME/Library/Android/sdk"
```

## Build

```bash
./gradlew :app:assembleDebug
```

APK ra ở `app/build/outputs/apk/debug/app-debug.apk` (~77 MB debug, ~24 MB release sau R8).

Cài lên máy đang cắm USB:

```bash
adb install -r app/build/outputs/apk/debug/app-debug.apk
```

## Build bản release

Keystore **không** nằm trong repo. Tự tạo một lần:

```bash
keytool -genkey -v -keystore release.jks -alias bizclaw -keyalg RSA -keysize 2048 -validity 10000
```

Rồi truyền vào lúc build:

```bash
./gradlew :app:assembleRelease -Pandroid.injected.signing.store.file=$PWD/release.jks -Pandroid.injected.signing.store.password=$PASS -Pandroid.injected.signing.key.alias=bizclaw -Pandroid.injected.signing.key.password=$PASS
```

## Thiết lập trên điện thoại

1. Mở app → tab **Trạng thái** → **Tải model** (E4B 3.65 GB, nên dùng Wi-Fi). Tải bằng DownloadManager nên tắt màn hình vẫn chạy tiếp, rớt mạng thì tự nối lại.
2. **Cấp quyền đọc thông báo** → chọn BizClaw Agent trong danh sách.
3. Tab **Cửa hàng** → nạp bảng giá, phí ship, chính sách bảo hành. Không có dữ liệu thì agent luôn trả lời "để em kiểm tra lại" thay vì bịa số.
4. Quay lại **Trạng thái** → bật **Tự động đọc tin nhắn đến**.
5. Nhờ người khác nhắn thử vào Zalo/Messenger → xem draft ở tab **Hộp thư** → bấm **Gửi**.
6. Khi thấy chất lượng ổn định mới bật **Tự gửi không cần duyệt**.

## Dùng Claude / OpenAI thay cho Gemma

Tab **Model** → chọn Claude hoặc OpenAI → dán API key.

| | Lấy key ở | Model mặc định |
|---|---|---|
| Claude | console.anthropic.com | `claude-opus-5` (rẻ hơn: `claude-sonnet-5`, `claude-haiku-4-5`) |
| OpenAI | platform.openai.com | gõ tay tên model có trong tài khoản |

Bật cloud thì agent gọi tool được: tự tra giá, tra chính sách, tạo đơn. Đổi lại nội dung tin nhắn khách đi lên cloud của nhà cung cấp và tính phí theo token. Chưa nhập key thì app tự chạy tạm bằng Gemma.

## Bật tự nhắn tin trước (Accessibility)

Tab **Model** → **Bật quyền Accessibility** → chọn "BizClaw — tự nhắn tin".

Sau đó ở tab **Đơn hàng**, đơn nào đến hạn hỏi thăm sẽ có nút **Nhắn ngay**: app soạn tin bằng model đang chọn, mở Zalo/Messenger, tìm khách và gửi.

Chỉ chạy khi anh bấm nút. Nếu Zalo đổi giao diện thì app dừng và báo đúng bước hỏng, anh gửi tay.

## Giới hạn cần biết

- Agent trả lời qua **nút trả lời nhanh của thông báo** (cùng cơ chế đồng hồ thông minh dùng). Nếu anh đã mở app chat và đọc tin đó thì thông báo biến mất → không gửi trực tiếp được nữa, tab Hộp thư sẽ báo "Hết hạn".
- Chỉ thấy tin nhắn **có sinh thông báo**. Tắt thông báo Zalo là agent mù.
- Không đọc được lịch sử hội thoại — mỗi tin nhắn được xử lý độc lập.
- Lần trả lời đầu tiên sau khi bật agent chậm hơn (phải nạp model vào RAM, ~5-10 giây).

## Xiaomi / HyperOS — bắt buộc làm

HyperOS kill service nền rất mạnh; không xử lý thì agent im lặng chết sau vài giờ. App có nút **Mở cài đặt app** ở tab Trạng thái, vào đó chỉnh:

| Mục | Đặt thành |
|---|---|
| Tiết kiệm pin / Battery saver | **Không giới hạn** (No restrictions) |
| Tự khởi động / Autostart | **Bật** |
| Khoá tác vụ nền | Vuốt đa nhiệm → giữ thẻ BizClaw → bấm ổ khoá |
| Thông báo | Cho phép, và **không** bật "Ẩn thông báo trên màn khoá" |

Với các hãng khác app tự hiện hướng dẫn tương ứng (Samsung, OPPO/Realme, Vivo, Huawei).
