// BizClaw Dashboard — Vietnamese translations
// Extracted from dashboard.html for modular maintenance
export const vi = {
  // Navigation
  'nav.dashboard':'Bảng điều khiển','nav.webchat':'Trò chuyện','nav.settings':'Cài đặt',
  'nav.providers':'Nhà cung cấp','nav.channels':'Kênh liên lạc','nav.tools':'Công cụ',
  'nav.brain':'Brain Engine','nav.mcp':'Máy chủ MCP','nav.agents':'AI Agent',
  'nav.gallery':'Gallery','nav.knowledge':'Kho tri thức','nav.config':'Tệp cấu hình',
  'nav.scheduler':'Lịch trình','nav.orchestration':'Điều phối',
  // Orchestration
  'orch.title':'Điều phối','orch.subtitle':'Ủy quyền, Chuyển giao, Vòng lặp đánh giá, Liên kết quyền, LLM Traces',
  'orch.delegations':'Ủy quyền','orch.links':'Liên kết quyền','orch.traces':'LLM Traces',
  'orch.delegate_title':'Giao việc (Delegate)','orch.from_agent':'Agent giao việc','orch.to_agent':'Agent nhận việc',
  'orch.task':'Nội dung công việc','orch.history':'Lịch sử Delegation',
  'orch.perm_links':'Liên kết quyền','orch.llm_traces':'LLM Traces',
  'gallery.title':'Gallery','gallery.subtitle':'50 mẫu agent template + tự tạo. Upload file MD mở rộng khả năng.',
  // Dashboard
  'dash.title':'Bảng điều khiển','dash.subtitle':'Trung tâm quản lý Agent Gateway',
  'dash.status':'Trạng thái','dash.version':'Phiên bản','dash.provider':'Nhà cung cấp',
  'dash.model':'Mô hình AI','dash.uptime':'Thời gian hoạt động','dash.platform':'Nền tảng',
  'dash.channels':'Kênh đang hoạt động','dash.clock':'Đồng hồ','dash.system':'Hệ thống',
  'dash.online':'Trực tuyến','dash.quickactions':'Thao tác nhanh',
  'sys.os':'Hệ điều hành:','sys.arch':'Kiến trúc:','sys.memory':'Bộ nhớ:',
  // Table headers
  'th.channel':'Kênh','th.type':'Loại','th.status':'Trạng thái','th.configured':'Đã cấu hình',
  // Chat
  'chat.title':'Trò chuyện','chat.welcome':'Đã kết nối BizClaw. Nhập tin nhắn để bắt đầu trò chuyện.',
  'chat.thinking':'BizClaw đang suy nghĩ','chat.placeholder':'Nhập tin nhắn...','chat.send':'Gửi ↑',
  'chat.history_cleared':'💬 Đã xóa lịch sử trò chuyện',
  'chat.help':'Lệnh có sẵn:\n/status — Xem trạng thái agent\n/reset — Xóa lịch sử chat\n/help — Hiện trợ giúp',
  // Status
  'status.connected':'Đã kết nối','status.disconnected':'Đã ngắt kết nối',
  // Settings
  'settings.title':'Cài đặt Agent','settings.subtitle':'Cấu hình AI agent — nhà cung cấp, mô hình, danh tính, bảo mật',
  'settings.save':'💾 Lưu cài đặt',
  'set.provider_section':'Nhà cung cấp AI','set.provider':'Nhà cung cấp','set.model':'Mô hình',
  'set.apikey':'Khóa API','set.baseurl':'URL API tùy chỉnh','set.loading_models':'Đang tải mô hình...',
  'set.model_path':'Đường dẫn mô hình','set.scan':'Quét','set.threads':'Luồng xử lý',
  'set.temperature':'Nhiệt độ','set.identity':'Danh tính','set.agent_name':'Tên Agent',
  'set.persona':'Vai trò','set.sysprompt':'Lời nhắc hệ thống','set.memory':'Bộ nhớ',
  'set.backend':'Backend','set.autosave':'Tự động lưu','set.security':'Bảo mật / Quyền tự chủ',
  'set.autonomy':'Mức tự chủ','set.readonly':'Chỉ đọc','set.supervised':'Giám sát',
  'set.full':'Toàn quyền','set.workspace_only':'Chỉ Workspace',
  // Providers
  'providers.title':'Nhà cung cấp','providers.subtitle':'Cấu hình nhà cung cấp AI — chọn provider mặc định, API key, model',
  'providers.active_label':'Provider đang dùng','providers.total_label':'Tổng providers',
  'providers.type_cloud':'Cloud','providers.type_local':'Nội bộ','providers.type_proxy':'Proxy',
  'providers.badge_active':'Đang dùng','providers.btn_activate':'⚡ Kích hoạt','providers.btn_active':'✅ Đang sử dụng',
  // Channels
  'channels.title':'Cấu hình kênh liên lạc','channels.subtitle':'Kết nối agent với các nền tảng nhắn tin',
  // Tools
  'tools.title':'Công cụ','tools.subtitle':'15 công cụ tích hợp + công cụ MCP bên ngoài',
  'tool.active':'Hoạt động','tool.available':'Sẵn sàng',
  'tool.file':'Tệp tin','tool.editfile':'Sửa tệp','tool.websearch':'Tìm kiếm Web',
  'tool.calendar':'Lịch','tool.docreader':'Đọc tài liệu','tool.summarizer':'Tóm tắt nhóm',
  'tool.httpreq':'HTTP Request','tool.configmgr':'Quản lý cấu hình','tool.memsearch':'Tìm trí nhớ',
  'tool.shell_desc':'Thực thi lệnh hệ thống (sandbox)','tool.file_desc':'Đọc/ghi/liệt kê tệp tin',
  'tool.editfile_desc':'Thay thế chính xác nội dung trong tệp',
  'tool.glob_desc':'Tìm tệp theo mẫu (*.rs, **/*.toml)',
  'tool.grep_desc':'Tìm nội dung trong tệp bằng regex',
  'tool.httpreq_desc':'Gọi API bên ngoài (GET/POST/PUT/DELETE)',
  'tool.configmgr_desc':'Đọc/ghi config.toml lúc runtime',
  'tool.memsearch_desc':'Tìm hội thoại cũ bằng FTS5',
  'tool.execcode':'Chạy mã','tool.execcode_desc':'Chạy code Python, JS, Go, Rust, C...',
  'tool.plan':'Kế hoạch','tool.plan_desc':'Phân rã tác vụ với phụ thuộc và độ phức tạp',
  'tool.sessionctx':'Ngữ cảnh phiên','tool.sessionctx_desc':'Thông tin phiên: provider, token, tools',
  'tool.calendar_desc':'Tích hợp Google Calendar','tool.docreader_desc':'Trích xuất PDF, DOCX, Excel, CSV',
  'tool.summarizer_desc':'Tóm tắt tin nhắn nhóm Zalo/Telegram',
  // MCP
  'mcp.title':'Máy chủ MCP','mcp.subtitle':'Model Context Protocol — mở rộng agent với công cụ bên ngoài',
  'mcp.add':'Thêm máy chủ','mcp.total':'Tổng máy chủ','mcp.active':'Đang hoạt động',
  'mcp.protocol':'Giao thức','mcp.add_title':'Thêm máy chủ MCP','mcp.popular':'MCP phổ biến',
  // Multi-Agent
  'agents.title':'AI Agent','agents.subtitle':'Tạo và quản lý nhiều Agent AI — mỗi agent có nhà cung cấp & mô hình riêng',
  'agents.create':'Tạo Agent','agents.total':'Tổng Agent','agents.default':'Agent mặc định',
  'agents.messages':'Tin nhắn','agents.create_new':'Tạo Agent mới','agents.broadcast':'Phát tin nhắn',
  'agents.msg':'Tin nhắn','agents.send_all':'Gửi tất cả','agents.edit':'Sửa Agent',
  // Agent Groups
  'groups.title':'Nhóm Agent','groups.subtitle':'Tạo nhóm các Agent AI cùng trao đổi và phối hợp',
  'groups.create':'Tạo nhóm','groups.total':'Tổng nhóm','groups.agents_count':'Agent trong nhóm',
  'groups.conversations':'Cuộc hội thoại','groups.create_new':'Tạo nhóm mới',
  'groups.topic':'Chủ đề','groups.select_agents':'Chọn Agent tham gia:',
  'groups.agent_hint':'Chưa có agent? Vào trang AI Agent để tạo trước.',
  // Knowledge Base
  'kb.title':'Kho tri thức','kb.subtitle':'RAG cá nhân — thêm tài liệu để AI trả lời chính xác hơn',
  'kb.add_doc':'Thêm tài liệu','kb.documents':'Tài liệu','kb.chunks':'Đoạn văn',
  'kb.search_engine':'Công cụ tìm kiếm','kb.search':'Tìm kiếm tri thức','kb.search_btn':'Tìm kiếm',
  'kb.add_title':'Thêm tài liệu','kb.source':'Nguồn','kb.content':'Nội dung',
  'kb.doc_list':'Danh sách tài liệu',
  // Brain Engine
  'brain.title':'Brain Engine','brain.engine':'Động cơ','brain.quant':'Lượng tử hóa',
  'brain.features':'Tính năng','brain.models':'Mô hình có sẵn','brain.size':'Kích thước',
  'brain.download':'Tải về','brain.ws_sub':'Workspace não, tệp tính cách và kiểm tra sức khỏe hệ thống',
  'brain.health_btn':'Kiểm tra sức khỏe','brain.health_title':'Kiểm tra sức khỏe hệ thống',
  'brain.check_name':'Kiểm tra','brain.check_status':'Trạng thái','brain.check_detail':'Chi tiết',
  'brain.ws_title':'Workspace Não','brain.new_file':'Tệp mới','brain.save':'Lưu',
  'brain.files_count':'Brain Files',
  // Forms
  'form.name':'Tên','form.command':'Lệnh','form.args':'Tham số (phân cách bằng dấu phẩy)',
  'form.env':'Biến môi trường (KEY=VALUE, mỗi dòng một cặp)','form.save':'Lưu','form.cancel':'Hủy',
  'form.role':'Vai trò','form.description':'Mô tả',
  // Config page
  'config.title':'config.toml',
  // Onboarding
  'onboard.welcome_sub':'Hãy để AI tạo một trợ lý phù hợp riêng cho bạn. Chỉ mất 30 giây.',
  'onboard.your_name':'Tên bạn','onboard.about_you':'Bạn là ai, làm gì?',
  'onboard.agent_vibe':'Phong cách Agent',
  'onboard.provider_sub':'Chọn nguồn AI. Không cần API key? Dùng Ollama hoặc Brain Engine.',
  'onboard.done_title':'Agent đã sẵn sàng!',
  'onboard.done_sub':'AI đang tạo tính cách riêng cho agent của bạn...',
  // Pairing
  'pairing.title':'BizClaw Agent','pairing.desc':'Nhập mã Pairing Code để truy cập Dashboard',
  'pairing.confirm':'🔓 Xác nhận','pairing.error':'Vui lòng nhập mã pairing',
  // Scheduler (new)
  'sched.title':'Lịch trình','sched.subtitle':'Quản lý tác vụ lên lịch với cơ chế retry tự động',
  'sched.retrying':'Đang retry','sched.failed':'Thất bại vĩnh viễn','sched.total_retries':'Tổng retry',
};
