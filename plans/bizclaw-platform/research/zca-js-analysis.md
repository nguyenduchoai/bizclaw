# ZCA-JS (Zalo Chat API) Analysis Report

> **Source**: https://github.com/RFS-ADRENO/zca-js
> **Language**: TypeScript (99.2%)
> **License**: MIT
> **API Docs**: https://zca-js.tdung.com
> **Tagline**: "Unofficial Zalo API for JavaScript"

## 1. Overview

ZCA-JS là thư viện **Unofficial Zalo API** cho JavaScript/TypeScript:
- Simulate browser để tương tác với **Zalo Web** (chat.zalo.me)
- Hỗ trợ cả **tài khoản cá nhân** (personal account) và **Zalo Business** (getBizAccount)
- Event-driven: listen messages, reactions, undo, group events
- **MIT License** → free to rewrite

### ⚠️ Warning
> Sử dụng API này có thể dẫn đến tài khoản bị khóa/ban. Đây là **unofficial API**.
> BizClaw nên cung cấp kèm warning rõ ràng cho người dùng.

## 2. Authentication Methods

### 2.1 Cookie Login
- Lấy Cookie từ trình duyệt (Zalo Web)
- Cần: `cookie` (JSON), `imei` (z_uuid), `userAgent`
- Dùng extension: ZaloDataExtractor, J2TEAM Cookies, Cookie-Editor

### 2.2 QR Code Login
- Scan QR code
- Tương tự: `zalo.loginQR()`

### 2.3 Multi-Account
- Hỗ trợ nhiều tài khoản đồng thời

### 2.4 Proxy Support
- Hỗ trợ kết nối qua proxy

## 3. Full API Inventory (85+ APIs)

### 3.1 Authentication & Account (5)
```
loginQR()           → QR Code login
login(cookie)       → Cookie login
fetchAccountInfo()  → Lấy thông tin tài khoản
getOwnId()          → Lấy ID người dùng
getCookie()         → Lấy cookie hiện tại
```

### 3.2 Messaging (12) — CORE cho BizClaw Channel
```
sendMessage()       → Gửi tin nhắn (text + quote)
sendSticker()       → Gửi sticker
sendLink()          → Gửi link
sendVideo()         → Gửi video
sendVoice()         → Gửi voice message
sendCard()          → Gửi contact card
sendBankCard()      → Gửi thẻ ngân hàng
forwardMessage()    → Chuyển tiếp tin nhắn
deleteMessage()     → Xóa tin nhắn
undo()              → Thu hồi tin nhắn
uploadAttachment()  → Upload file đính kèm
sendDeliveredEvent() → Gửi sự kiện "đã nhận"
sendSeenEvent()     → Gửi sự kiện "đã xem"
sendTypingEvent()   → Gửi sự kiện "đang gõ"
```

### 3.3 Friends & Contacts (10)
```
sendFriendRequest()     → Gửi lời mời kết bạn
acceptFriendRequest()   → Chấp nhận lời mời
undoFriendRequest()     → Hủy lời mời
removeFriend()          → Xóa bạn
blockUser()             → Chặn người dùng
unblockUser()           → Bỏ chặn
findUser()              → Tìm người dùng
getUserInfo()           → Lấy thông tin người dùng
getAllFriends()          → Lấy danh sách bạn bè
changeFriendAlias()     → Đặt nickname
removeFriendAlias()     → Xóa nickname
getFriendRecommendations() → Gợi ý kết bạn
getSentFriendRequest()  → Xem lời mời đã gửi
getFriendRequestStatus() → Trạng thái lời mời
lastOnline()            → Thời gian online cuối
```

### 3.4 Group Management (20+)
```
createGroup()            → Tạo nhóm
addUserToGroup()         → Thêm thành viên
removeUserFromGroup()    → Xóa thành viên
leaveGroup()             → Rời nhóm
disperseGroup()          → Giải tán nhóm
changeGroupName()        → Đổi tên nhóm
changeGroupAvatar()      → Đổi ảnh nhóm
changeGroupOwner()       → Chuyển quyền nhóm trưởng
addGroupDeputy()         → Thêm phó nhóm
removeGroupDeputy()      → Xóa phó nhóm
addGroupBlockedMember()  → Block thành viên
removeGroupBlockedMember() → Unblock
getGroupInfo()           → Thông tin nhóm
getGroupMembersInfo()    → Thông tin thành viên
getAllGroups()            → Danh sách nhóm
getPendingGroupMembers() → Yêu cầu gia nhập
reviewPendingMemberRequest() → Duyệt yêu cầu
updateGroupSettings()    → Cập nhật cài đặt
enableGroupLink()        → Bật link mời
disableGroupLink()       → Tắt link mời
getGroupLinkDetail()     → Chi tiết link
inviteUserToGroups()     → Mời vào nhóm
joinGroupLink()          → Tham gia qua link
getGroupInviteBoxList()  → Danh sách invite box
joinGroupInviteBox()     → Tham gia qua invite box
```

### 3.5 Polls & Reminders (5)
```
createPoll()        → Tạo bình chọn
getPollDetail()     → Chi tiết bình chọn
lockPoll()          → Khóa bình chọn
createReminder()    → Tạo nhắc nhở
editReminder()      → Sửa nhắc nhở
removeReminder()    → Xóa nhắc nhở
getReminder()       → Lấy nhắc nhở
getListReminder()   → Danh sách nhắc nhở
```

### 3.6 Reactions & Stickers (4)
```
addReaction()       → Thả reaction
getStickers()       → Tìm sticker
getStickersDetail() → Chi tiết sticker
sendSticker()       → Gửi sticker
```

### 3.7 Chat Management (10)
```
deleteChat()             → Xóa hội thoại
getArchivedChatList()    → Hội thoại lưu trữ
setHiddenConversations() → Ẩn hội thoại
getHiddenConversations() → Lấy hội thoại ẩn
setPinnedConversations()  → Ghim hội thoại
getPinConversations()     → Lấy hội thoại ghim
getAutoDeleteChat()       → Tự động xóa chat
updateAutoDeleteChat()    → Cập nhật tự động xóa
addUnreadMark()           → Đánh dấu chưa đọc
removeUnreadMark()        → Xóa đánh dấu
getMute()                 → Trạng thái tắt thông báo
setMute()                 → Tắt/bật thông báo
```

### 3.8 Profile & Settings (5)
```
changeAccountAvatar()    → Đổi ảnh đại diện
deleteAvatar()           → Xóa ảnh
reuseAvatar()            → Dùng lại ảnh cũ
getAvatarList()          → Danh sách ảnh
updateProfile()          → Cập nhật profile
updateSettings()         → Cập nhật cài đặt
updateLang()             → Đổi ngôn ngữ
```

### 3.9 Business Features — Zalo Business (6)
```
getBizAccount()          → Lấy thông tin business account
createCatalog()          → Tạo catalog
deleteCatalog()          → Xóa catalog
updateCatalog()          → Cập nhật catalog
getCatalogList()         → Danh sách catalog
createProductCatalog()   → Tạo sản phẩm catalog
deleteProductCatalog()   → Xóa sản phẩm
updateProductCatalog()   → Cập nhật sản phẩm
getProductCatalogList()  → Danh sách sản phẩm
uploadProductPhoto()     → Upload ảnh sản phẩm
```

### 3.10 Auto-Reply & Quick Messages (5)
```
createAutoReply()        → Tạo trả lời tự động
deleteAutoReply()        → Xóa
updateAutoReply()        → Cập nhật
getAutoReplyList()       → Danh sách
addQuickMessage()        → Thêm tin nhắn nhanh
removeQuickMessage()     → Xóa
updateQuickMessage()     → Cập nhật
getQuickMessageList()    → Danh sách
```

### 3.11 Labels & Board (5)
```
getLabels()              → Lấy label
updateLabels()           → Cập nhật label
getListBoard()           → Danh sách bảng
getFriendBoardList()     → Bảng bạn bè
createNote()             → Tạo ghi chú
editNote()               → Sửa ghi chú
```

### 3.12 Other (3)
```
parseLink()              → Parse link URL
keepAlive()              → Giữ kết nối
sendReport()             → Gửi báo cáo
blockViewFeed()          → Chặn xem feed
custom()                 → Custom API call
```

## 4. Event Listeners

```
message      → Tin nhắn mới (User / Group)
reaction     → Reaction mới
undo         → Thu hồi tin nhắn
group_event  → Sự kiện nhóm (member join/leave/kick, etc.)
```

### Message Structure
```typescript
{
  type: ThreadType.User | ThreadType.Group,
  threadId: string,
  isSelf: boolean,
  data: {
    content: string | object,
    // + attachments, stickers, etc.
  }
}
```

## 5. Data Models (19 models)

```
Attachment, AutoReply, Board, Catalog, DeliveredMessage,
Enum, FriendEvent, Group, GroupEvent, Label, Message,
ProductCatalog, QuickMessage, Reaction, Reminder,
SeenMessage, Typing, Undo, User, ZBusiness
```

## 6. Technical Architecture (How it works internally)

ZCA-JS hoạt động bằng cách:
1. **Simulate browser session** → Sử dụng cookie + userAgent + imei
2. **HTTP requests** → Gọi các internal Zalo Web API endpoints
3. **WebSocket / Long-polling** → Listen for real-time events
4. **Encryption** → Zalo sử dụng encrypted payloads (cần reverse-engineer)

### Key Technical Requirements cho Rust rewrite:
- **HTTP client**: reqwest (async HTTP)
- **WebSocket**: tokio-tungstenite
- **Cookie management**: cookie jar implementation
- **Encryption**: Zalo's custom encryption (AES + RSA based)
- **QR Code**: qrcode crate (generate QR for login)
- **Image metadata**: image crate (for sending images)

## 7. Rust Rewrite Strategy

### Approach: Reverse-engineer HTTP protocol (NOT browser automation)

ZCA-JS đã reverse-engineer toàn bộ Zalo Web protocol. BizClaw sẽ:
1. **Study zca-js source** → Hiểu HTTP endpoints, encryption, auth flow
2. **Implement in Rust** → reqwest + tokio-tungstenite
3. **No browser needed** → Pure HTTP/WS client
4. **API parity** → Same functionality, Rust safety

### Module Mapping: zca-js → bizclaw-zalo

| zca-js (TypeScript) | bizclaw-zalo (Rust) |
|---------------------|---------------------|
| `Zalo` class | `ZaloClient` struct |
| `login()` / `loginQR()` | `auth.rs` module |
| `sendMessage()` + friends | `messaging.rs` |
| `group*()` APIs | `groups.rs` |
| `friend*()` APIs | `friends.rs` |
| `listener.on()` | `listener.rs` (async stream) |
| Encryption utils | `crypto.rs` |
| Cookie management | `session.rs` |
| Business APIs | `business.rs` |

## 8. BizClaw Integration: Zalo as Channel

### Two sub-channels within Zalo:

```
bizclaw-channels/
└── src/
    ├── zalo/
    │   ├── mod.rs              # ZaloChannel impl
    │   ├── personal.rs         # Zalo Personal (via zca protocol)
    │   ├── official.rs         # Zalo OA (Official Account API) — future
    │   └── client/
    │       ├── mod.rs
    │       ├── auth.rs         # Cookie/QR login
    │       ├── session.rs      # Session management
    │       ├── crypto.rs       # Zalo encryption
    │       ├── messaging.rs    # Send/receive messages
    │       ├── groups.rs       # Group management
    │       ├── friends.rs      # Friend management
    │       ├── business.rs     # Business features
    │       ├── listener.rs     # Event listener (WebSocket)
    │       └── models.rs       # Data models
```

### Channel Trait Implementation

```rust
// bizclaw-channels/src/zalo/mod.rs

pub struct ZaloChannel {
    client: ZaloClient,
    mode: ZaloMode, // Personal or OfficialAccount
}

pub enum ZaloMode {
    Personal {
        cookie_path: PathBuf,
        imei: String,
        user_agent: String,
    },
    OfficialAccount {
        app_id: String,
        secret_key: String,
    },
}

impl Channel for ZaloChannel {
    async fn connect(&mut self) -> Result<()> {
        match &self.mode {
            ZaloMode::Personal { .. } => self.client.login_cookie().await,
            ZaloMode::OfficialAccount { .. } => self.client.login_oa().await,
        }
    }

    async fn listen(&self) -> impl Stream<Item = IncomingMessage> {
        self.client.listener()
            .map(|zalo_msg| IncomingMessage {
                channel: "zalo",
                thread_id: zalo_msg.thread_id,
                sender_id: zalo_msg.sender_id,
                content: zalo_msg.content,
                thread_type: match zalo_msg.thread_type {
                    ThreadType::User => "direct",
                    ThreadType::Group => "group",
                },
            })
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        self.client.send_message(
            msg.content,
            msg.thread_id,
            msg.thread_type.into(),
        ).await
    }
}
```

## 9. Key Takeaways cho BizClaw

1. **85+ API functions** → BizClaw sẽ là Zalo client mạnh nhất trên Rust
2. **Personal + Business** → Dual mode support
3. **Event-driven** → Fits perfectly with trait-driven architecture
4. **QR Login** → Dễ setup cho CLI users
5. **Encryption** → Cần reverse-engineer kỹ từ zca-js source
6. **Rate limiting** → Cần implement để tránh bị ban
7. **Warning system** → Phải cảnh báo người dùng về risk

## 10. Effort Estimate

| Component | Effort | Notes |
|-----------|--------|-------|
| Auth (cookie + QR) | 3-4 days | Reverse-engineer from zca-js |
| Encryption/Crypto | 3-4 days | AES/RSA Zalo-specific |
| Messaging core | 3-4 days | send/receive/forward/delete |
| Event listener (WS) | 2-3 days | WebSocket real-time |
| Group management | 2-3 days | 20+ group APIs |
| Friend management | 1-2 days | 10+ friend APIs |
| Business features | 2-3 days | Catalog, products |
| Channel trait integration | 1-2 days | BizClaw Channel impl |
| **Total** | **~18-24 days** | |
