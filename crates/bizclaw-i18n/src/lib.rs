use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum I18nError {
    #[error("Translation key not found: {0}")]
    KeyNotFound(String),
    #[error("Locale not supported: {0}")]
    LocaleNotSupported(String),
    #[error("Failed to load translations: {0}")]
    LoadError(String),
    #[error("Invalid locale format: {0}")]
    InvalidFormat(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextDirection {
    Ltr,
    Rtl,
}

impl Default for TextDirection {
    fn default() -> Self {
        Self::Ltr
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocaleConfig {
    pub code: String,
    pub name: String,
    pub native_name: String,
    pub direction: TextDirection,
    pub date_format: String,
    pub time_format: String,
    pub decimal_separator: String,
    pub thousands_separator: String,
}

impl LocaleConfig {
    pub fn vi() -> Self {
        Self {
            code: "vi".to_string(),
            name: "Vietnamese".to_string(),
            native_name: "Tiếng Việt".to_string(),
            direction: TextDirection::Ltr,
            date_format: "dd/MM/yyyy".to_string(),
            time_format: "HH:mm".to_string(),
            decimal_separator: ",".to_string(),
            thousands_separator: ".".to_string(),
        }
    }

    pub fn en() -> Self {
        Self {
            code: "en".to_string(),
            name: "English".to_string(),
            native_name: "English".to_string(),
            direction: TextDirection::Ltr,
            date_format: "MM/dd/yyyy".to_string(),
            time_format: "HH:mm".to_string(),
            decimal_separator: ".".to_string(),
            thousands_separator: ",".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TranslationMap {
    #[serde(flatten)]
    pub strings: HashMap<String, String>,
}

impl TranslationMap {
    pub fn get(&self, key: &str) -> Option<&String> {
        self.strings.get(key)
    }

    pub fn insert(&mut self, key: String, value: String) {
        self.strings.insert(key, value);
    }
}

pub static I18N: Lazy<RwLock<I18n>> = Lazy::new(|| RwLock::new(I18n::new()));

pub struct I18n {
    locales: HashMap<String, LocaleConfig>,
    translations: HashMap<String, TranslationMap>,
    fallback: String,
    current: RwLock<String>,
}

impl Default for I18n {
    fn default() -> Self {
        Self::new()
    }
}

impl I18n {
    pub fn new() -> Self {
        let mut i18n = Self {
            locales: HashMap::new(),
            translations: HashMap::new(),
            fallback: "en".to_string(),
            current: RwLock::new("vi".to_string()),
        };
        i18n.register_builtin_locales();
        i18n
    }

    fn register_builtin_locales(&mut self) {
        self.register_locale(LocaleConfig::vi());
        self.register_locale(LocaleConfig::en());
        self.load_builtin_translations();
    }

    pub fn register_locale(&mut self, config: LocaleConfig) {
        self.locales.insert(config.code.clone(), config);
    }

    pub fn register_translation(&mut self, locale: &str, translations: TranslationMap) {
        self.translations.insert(locale.to_string(), translations);
    }

    pub fn set_current_locale(&self, locale: &str) -> Result<(), I18nError> {
        if !self.locales.contains_key(locale) {
            return Err(I18nError::LocaleNotSupported(locale.to_string()));
        }
        let mut current = self.current.write();
        *current = locale.to_string();
        Ok(())
    }

    pub fn get_current_locale(&self) -> String {
        self.current.read().clone()
    }

    pub fn get_locale_config(&self, locale: &str) -> Option<&LocaleConfig> {
        self.locales.get(locale)
    }

    pub fn list_locales(&self) -> Vec<&LocaleConfig> {
        self.locales.values().collect()
    }

    pub fn t(&self, key: &str) -> String {
        self.t_with_locale(key, &self.get_current_locale())
    }

    pub fn t_with_locale(&self, key: &str, locale: &str) -> String {
        self.translations
            .get(locale)
            .and_then(|t| t.get(key))
            .or_else(|| {
                self.translations
                    .get(&self.fallback)
                    .and_then(|t| t.get(key))
            })
            .cloned()
            .unwrap_or_else(|| {
                tracing::warn!("Translation key not found: {} (locale: {})", key, locale);
                key.to_string()
            })
    }

    pub fn t_args(&self, key: &str, args: &[(&str, &str)]) -> String {
        let template = self.t(key);
        let mut result = template;
        for (key_arg, value) in args {
            result = result.replace(&format!("{{{}}}", key_arg), value);
        }
        result
    }

    pub fn load_from_json(&mut self, locale: &str, path: &Path) -> Result<(), I18nError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| I18nError::LoadError(e.to_string()))?;
        let translations: TranslationMap =
            serde_json::from_str(&content).map_err(|e| I18nError::InvalidFormat(e.to_string()))?;
        self.register_translation(locale, translations);
        Ok(())
    }

    pub fn load_from_directory(&mut self, path: &Path) -> Result<(), I18nError> {
        if !path.is_dir() {
            return Err(I18nError::LoadError("Path is not a directory".to_string()));
        }

        let pattern = path.join("*.json");
        let entries = glob::glob(pattern.to_string_lossy().as_ref())
            .map_err(|e| I18nError::LoadError(e.to_string()))?;

        for entry in entries.filter_map(|e| e.ok()) {
            if let Some(filename) = entry.file_stem() {
                let locale = filename.to_string_lossy();
                if let Err(e) = self.load_from_json(&locale, &entry) {
                    tracing::warn!("Failed to load {}: {}", entry.display(), e);
                }
            }
        }
        Ok(())
    }

    fn load_builtin_translations(&mut self) {
        let vi_translations = self.create_vietnamese_translations();
        let en_translations = self.create_english_translations();
        self.register_translation("vi", vi_translations);
        self.register_translation("en", en_translations);
    }

    fn create_vietnamese_translations(&self) -> TranslationMap {
        let mut map = TranslationMap::default();
        map.insert("common.loading".to_string(), "Đang tải...".to_string());
        map.insert("common.error".to_string(), "Lỗi".to_string());
        map.insert("common.success".to_string(), "Thành công".to_string());
        map.insert("common.cancel".to_string(), "Hủy".to_string());
        map.insert("common.save".to_string(), "Lưu".to_string());
        map.insert("common.delete".to_string(), "Xóa".to_string());
        map.insert("common.edit".to_string(), "Sửa".to_string());
        map.insert("common.create".to_string(), "Tạo mới".to_string());
        map.insert("common.search".to_string(), "Tìm kiếm".to_string());
        map.insert("common.filter".to_string(), "Lọc".to_string());
        map.insert("common.export".to_string(), "Xuất".to_string());
        map.insert("common.import".to_string(), "Nhập".to_string());
        map.insert("common.back".to_string(), "Quay lại".to_string());
        map.insert("common.next".to_string(), "Tiếp theo".to_string());
        map.insert("common.prev".to_string(), "Trước".to_string());
        map.insert("common.close".to_string(), "Đóng".to_string());
        map.insert("common.confirm".to_string(), "Xác nhận".to_string());
        map.insert("common.yes".to_string(), "Có".to_string());
        map.insert("common.no".to_string(), "Không".to_string());
        map.insert("common.ok".to_string(), "OK".to_string());
        map.insert("common.wait".to_string(), "Vui lòng đợi...".to_string());
        map.insert("common.retry".to_string(), "Thử lại".to_string());
        map.insert("auth.login".to_string(), "Đăng nhập".to_string());
        map.insert("auth.register".to_string(), "Đăng ký".to_string());
        map.insert("auth.logout".to_string(), "Đăng xuất".to_string());
        map.insert("auth.email".to_string(), "Email".to_string());
        map.insert("auth.password".to_string(), "Mật khẩu".to_string());
        map.insert(
            "auth.forgot_password".to_string(),
            "Quên mật khẩu?".to_string(),
        );
        map.insert(
            "auth.welcome_back".to_string(),
            "Chào mừng trở lại".to_string(),
        );
        map.insert(
            "auth.enter_credentials".to_string(),
            "Nhập thông tin đăng nhập".to_string(),
        );
        map.insert(
            "auth.invalid_credentials".to_string(),
            "Email hoặc mật khẩu không đúng".to_string(),
        );
        map.insert(
            "auth.session_expired".to_string(),
            "Phiên đã hết hạn. Vui lòng đăng nhập lại.".to_string(),
        );
        map.insert("dashboard.title".to_string(), "Bảng điều khiển".to_string());
        map.insert(
            "dashboard.subtitle".to_string(),
            "Trung tâm quản lý BizClaw".to_string(),
        );
        map.insert("dashboard.status".to_string(), "Trạng thái".to_string());
        map.insert("dashboard.version".to_string(), "Phiên bản".to_string());
        map.insert("dashboard.online".to_string(), "Trực tuyến".to_string());
        map.insert("dashboard.offline".to_string(), "Ngoại tuyến".to_string());
        map.insert(
            "dashboard.uptime".to_string(),
            "Thời gian hoạt động".to_string(),
        );
        map.insert("content.title".to_string(), "Tạo Nội Dung".to_string());
        map.insert("content.generate".to_string(), "Tạo nội dung".to_string());
        map.insert("content.script".to_string(), "Kịch bản".to_string());
        map.insert("content.image".to_string(), "Hình ảnh".to_string());
        map.insert("content.video".to_string(), "Video".to_string());
        map.insert("content.voice".to_string(), "Giọng đọc".to_string());
        map.insert("content.style".to_string(), "Phong cách".to_string());
        map.insert(
            "content.placeholder".to_string(),
            "Nhập mô tả nội dung...".to_string(),
        );
        map.insert("scheduler.title".to_string(), "Lịch Đăng Bài".to_string());
        map.insert("scheduler.schedule".to_string(), "Đặt lịch".to_string());
        map.insert("scheduler.platforms".to_string(), "Nền tảng".to_string());
        map.insert("scheduler.date".to_string(), "Ngày".to_string());
        map.insert("scheduler.time".to_string(), "Giờ".to_string());
        map.insert("scheduler.scheduled".to_string(), "Đã đặt lịch".to_string());
        map.insert("scheduler.pending".to_string(), "Đang chờ".to_string());
        map.insert("scheduler.published".to_string(), "Đã đăng".to_string());
        map.insert("scheduler.failed".to_string(), "Thất bại".to_string());
        map.insert("social.title".to_string(), "Mạng Xã Hội".to_string());
        map.insert("social.connect".to_string(), "Kết nối".to_string());
        map.insert("social.disconnect".to_string(), "Ngắt kết nối".to_string());
        map.insert("social.facebook".to_string(), "Facebook".to_string());
        map.insert("social.instagram".to_string(), "Instagram".to_string());
        map.insert("social.tiktok".to_string(), "TikTok".to_string());
        map.insert("social.zalo".to_string(), "Zalo OA".to_string());
        map.insert("social.shopee".to_string(), "Shopee".to_string());
        map.insert("social.followers".to_string(), "Người theo dõi".to_string());
        map.insert("social.engagement".to_string(), "Tương tác".to_string());
        map.insert("analytics.title".to_string(), "Phân Tích".to_string());
        map.insert("analytics.views".to_string(), "Lượt xem".to_string());
        map.insert("analytics.clicks".to_string(), "Lượt nhấn".to_string());
        map.insert("analytics.shares".to_string(), "Lượt chia sẻ".to_string());
        map.insert("analytics.comments".to_string(), "Bình luận".to_string());
        map.insert("analytics.reactions".to_string(), "Cảm xúc".to_string());
        map.insert("analytics.growth".to_string(), "Tăng trưởng".to_string());
        map.insert("settings.title".to_string(), "Cài Đặt".to_string());
        map.insert("settings.profile".to_string(), "Hồ sơ".to_string());
        map.insert("settings.account".to_string(), "Tài khoản".to_string());
        map.insert(
            "settings.notifications".to_string(),
            "Thông báo".to_string(),
        );
        map.insert("settings.integrations".to_string(), "Tích hợp".to_string());
        map.insert("settings.api_keys".to_string(), "API Keys".to_string());
        map.insert("settings.language".to_string(), "Ngôn ngữ".to_string());
        map.insert("settings.theme".to_string(), "Giao diện".to_string());
        map.insert("settings.dark".to_string(), "Tối".to_string());
        map.insert("settings.light".to_string(), "Sáng".to_string());
        map.insert("settings.system".to_string(), "Hệ thống".to_string());
        map.insert("error.not_found".to_string(), "Không tìm thấy".to_string());
        map.insert(
            "error.unauthorized".to_string(),
            "Không có quyền truy cập".to_string(),
        );
        map.insert("error.forbidden".to_string(), "Bị cấm truy cập".to_string());
        map.insert(
            "error.internal".to_string(),
            "Lỗi máy chủ nội bộ".to_string(),
        );
        map.insert(
            "error.bad_request".to_string(),
            "Yêu cầu không hợp lệ".to_string(),
        );
        map.insert(
            "error.validation".to_string(),
            "Dữ liệu không hợp lệ".to_string(),
        );
        map.insert("error.network".to_string(), "Lỗi kết nối mạng".to_string());
        map.insert("error.timeout".to_string(), "Hết thời gian chờ".to_string());
        map
    }

    fn create_english_translations(&self) -> TranslationMap {
        let mut map = TranslationMap::default();
        map.insert("common.loading".to_string(), "Loading...".to_string());
        map.insert("common.error".to_string(), "Error".to_string());
        map.insert("common.success".to_string(), "Success".to_string());
        map.insert("common.cancel".to_string(), "Cancel".to_string());
        map.insert("common.save".to_string(), "Save".to_string());
        map.insert("common.delete".to_string(), "Delete".to_string());
        map.insert("common.edit".to_string(), "Edit".to_string());
        map.insert("common.create".to_string(), "Create New".to_string());
        map.insert("common.search".to_string(), "Search".to_string());
        map.insert("common.filter".to_string(), "Filter".to_string());
        map.insert("common.export".to_string(), "Export".to_string());
        map.insert("common.import".to_string(), "Import".to_string());
        map.insert("common.back".to_string(), "Back".to_string());
        map.insert("common.next".to_string(), "Next".to_string());
        map.insert("common.prev".to_string(), "Previous".to_string());
        map.insert("common.close".to_string(), "Close".to_string());
        map.insert("common.confirm".to_string(), "Confirm".to_string());
        map.insert("common.yes".to_string(), "Yes".to_string());
        map.insert("common.no".to_string(), "No".to_string());
        map.insert("common.ok".to_string(), "OK".to_string());
        map.insert("common.wait".to_string(), "Please wait...".to_string());
        map.insert("common.retry".to_string(), "Retry".to_string());
        map.insert("auth.login".to_string(), "Login".to_string());
        map.insert("auth.register".to_string(), "Register".to_string());
        map.insert("auth.logout".to_string(), "Logout".to_string());
        map.insert("auth.email".to_string(), "Email".to_string());
        map.insert("auth.password".to_string(), "Password".to_string());
        map.insert(
            "auth.forgot_password".to_string(),
            "Forgot password?".to_string(),
        );
        map.insert("auth.welcome_back".to_string(), "Welcome back".to_string());
        map.insert(
            "auth.enter_credentials".to_string(),
            "Enter your credentials".to_string(),
        );
        map.insert(
            "auth.invalid_credentials".to_string(),
            "Invalid email or password".to_string(),
        );
        map.insert(
            "auth.session_expired".to_string(),
            "Session expired. Please login again.".to_string(),
        );
        map.insert("dashboard.title".to_string(), "Dashboard".to_string());
        map.insert(
            "dashboard.subtitle".to_string(),
            "BizClaw Control Center".to_string(),
        );
        map.insert("dashboard.status".to_string(), "Status".to_string());
        map.insert("dashboard.version".to_string(), "Version".to_string());
        map.insert("dashboard.online".to_string(), "Online".to_string());
        map.insert("dashboard.offline".to_string(), "Offline".to_string());
        map.insert("dashboard.uptime".to_string(), "Uptime".to_string());
        map.insert("content.title".to_string(), "Content Creation".to_string());
        map.insert(
            "content.generate".to_string(),
            "Generate content".to_string(),
        );
        map.insert("content.script".to_string(), "Script".to_string());
        map.insert("content.image".to_string(), "Image".to_string());
        map.insert("content.video".to_string(), "Video".to_string());
        map.insert("content.voice".to_string(), "Voice".to_string());
        map.insert("content.style".to_string(), "Style".to_string());
        map.insert(
            "content.placeholder".to_string(),
            "Enter content description...".to_string(),
        );
        map.insert("scheduler.title".to_string(), "Post Scheduler".to_string());
        map.insert("scheduler.schedule".to_string(), "Schedule".to_string());
        map.insert("scheduler.platforms".to_string(), "Platforms".to_string());
        map.insert("scheduler.date".to_string(), "Date".to_string());
        map.insert("scheduler.time".to_string(), "Time".to_string());
        map.insert("scheduler.scheduled".to_string(), "Scheduled".to_string());
        map.insert("scheduler.pending".to_string(), "Pending".to_string());
        map.insert("scheduler.published".to_string(), "Published".to_string());
        map.insert("scheduler.failed".to_string(), "Failed".to_string());
        map.insert("social.title".to_string(), "Social Media".to_string());
        map.insert("social.connect".to_string(), "Connect".to_string());
        map.insert("social.disconnect".to_string(), "Disconnect".to_string());
        map.insert("social.facebook".to_string(), "Facebook".to_string());
        map.insert("social.instagram".to_string(), "Instagram".to_string());
        map.insert("social.tiktok".to_string(), "TikTok".to_string());
        map.insert("social.zalo".to_string(), "Zalo OA".to_string());
        map.insert("social.shopee".to_string(), "Shopee".to_string());
        map.insert("social.followers".to_string(), "Followers".to_string());
        map.insert("social.engagement".to_string(), "Engagement".to_string());
        map.insert("analytics.title".to_string(), "Analytics".to_string());
        map.insert("analytics.views".to_string(), "Views".to_string());
        map.insert("analytics.clicks".to_string(), "Clicks".to_string());
        map.insert("analytics.shares".to_string(), "Shares".to_string());
        map.insert("analytics.comments".to_string(), "Comments".to_string());
        map.insert("analytics.reactions".to_string(), "Reactions".to_string());
        map.insert("analytics.growth".to_string(), "Growth".to_string());
        map.insert("settings.title".to_string(), "Settings".to_string());
        map.insert("settings.profile".to_string(), "Profile".to_string());
        map.insert("settings.account".to_string(), "Account".to_string());
        map.insert(
            "settings.notifications".to_string(),
            "Notifications".to_string(),
        );
        map.insert(
            "settings.integrations".to_string(),
            "Integrations".to_string(),
        );
        map.insert("settings.api_keys".to_string(), "API Keys".to_string());
        map.insert("settings.language".to_string(), "Language".to_string());
        map.insert("settings.theme".to_string(), "Theme".to_string());
        map.insert("settings.dark".to_string(), "Dark".to_string());
        map.insert("settings.light".to_string(), "Light".to_string());
        map.insert("settings.system".to_string(), "System".to_string());
        map.insert("error.not_found".to_string(), "Not found".to_string());
        map.insert("error.unauthorized".to_string(), "Unauthorized".to_string());
        map.insert("error.forbidden".to_string(), "Forbidden".to_string());
        map.insert(
            "error.internal".to_string(),
            "Internal server error".to_string(),
        );
        map.insert("error.bad_request".to_string(), "Bad request".to_string());
        map.insert("error.validation".to_string(), "Invalid data".to_string());
        map.insert("error.network".to_string(), "Network error".to_string());
        map.insert("error.timeout".to_string(), "Request timeout".to_string());
        map
    }
}

pub fn t(key: &str) -> String {
    I18N.read().t(key)
}

pub fn t_with_locale(key: &str, locale: &str) -> String {
    I18N.read().t_with_locale(key, locale)
}

pub fn t_args(key: &str, args: &[(&str, &str)]) -> String {
    I18N.read().t_args(key, args)
}

pub fn set_locale(locale: &str) -> Result<(), I18nError> {
    I18N.write().set_current_locale(locale)
}

pub fn get_locale() -> String {
    I18N.read().get_current_locale()
}

pub fn get_available_locales() -> Vec<LocaleConfig> {
    I18N.read().list_locales().into_iter().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translation_vietnamese() {
        let i18n = I18n::new();
        i18n.set_current_locale("vi").unwrap();

        assert_eq!(i18n.t("common.save"), "Lưu");
        assert_eq!(i18n.t("auth.login"), "Đăng nhập");
        assert_eq!(i18n.t("dashboard.title"), "Bảng điều khiển");
    }

    #[test]
    fn test_translation_english() {
        let i18n = I18n::new();
        i18n.set_current_locale("en").unwrap();

        assert_eq!(i18n.t("common.save"), "Save");
        assert_eq!(i18n.t("auth.login"), "Login");
        assert_eq!(i18n.t("dashboard.title"), "Dashboard");
    }

    #[test]
    fn test_translation_with_args() {
        let i18n = I18n::new();
        i18n.set_current_locale("en").unwrap();

        let result = i18n.t_args("Hello {name}", &[("name", "World")]);
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_locale_config() {
        let vi_config = LocaleConfig::vi();
        assert_eq!(vi_config.code, "vi");
        assert_eq!(vi_config.name, "Vietnamese");
        assert_eq!(vi_config.native_name, "Tiếng Việt");
        assert_eq!(vi_config.direction, TextDirection::Ltr);

        let en_config = LocaleConfig::en();
        assert_eq!(en_config.code, "en");
        assert_eq!(en_config.name, "English");
        assert_eq!(en_config.direction, TextDirection::Ltr);
    }

    #[test]
    fn test_fallback() {
        let i18n = I18n::new();
        i18n.set_current_locale("vi").unwrap();

        let result = i18n.t_with_locale("nonexistent.key", "xx");
        assert_eq!(result, "nonexistent.key");
    }

    #[test]
    fn test_invalid_locale() {
        let i18n = I18n::new();
        let result = i18n.set_current_locale("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_global_functions() {
        set_locale("vi").unwrap();
        assert_eq!(get_locale(), "vi");

        assert_eq!(t("common.save"), "Lưu");

        set_locale("en").unwrap();
        assert_eq!(t("common.save"), "Save");
    }
}
