pub mod cdp;
pub mod tools;
pub mod skills;
pub mod session;
pub mod stealth;
pub mod captcha;
pub mod proxy;
pub mod behavior;
pub mod error;
pub mod lightpanda;

pub use cdp::CdpClient;
pub use tools::{BrowserTools, BrowserToolResult};
pub use skills::{BrowserSkill, SkillRegistry};
pub use session::{BrowserSession, SessionConfig, ViewportConfig, SessionInfo, SessionManager};
pub use stealth::{StealthConfig, StealthManager};
pub use captcha::{
    CaptchaSolver, CaptchaHandler, CaptchaType, CaptchaSolution, CaptchaError,
    CaptchaProviderConfig, LlmProviderConfig, LlmProvider,
};
pub use proxy::{ProxyManager, ProxyConfig, ProxyProtocol, ProxyRotation, RotationStrategy};
pub use behavior::{
    SessionState, SessionPersistence, SessionManagerV2, HumanBehaviorEngine,
    HumanBehaviorConfig, Viewport as BehaviorViewport, Cookie,
    export_cookies_as_netscape, import_cookies_from_netscape,
};
pub use lightpanda::{
    LightpandaClient, LightpandaConfig, SemanticNode, PageContent, Link, Form, FormField,
    Image, MCPTool, Cookie as LightpandaCookie, CookieInput as LightpandaCookieInput,
    ViewportConfig as LightpandaViewport,
};
pub use error::{BrowserError, Result};
