//! Web Dashboard — embedded HTML/JS/CSS served at / and /static/
//!
//! Supports two modes:
//! - **New Dashboard** (Preact + HTM): Served at `/` with modular components
//! - **Legacy Dashboard**: Served at `/legacy` for backward compatibility
//!
//! All files are embedded at compile time via `include_str!()` to maintain
//! the self-contained binary philosophy (no external file dependencies).

use std::collections::HashMap;

/// Legacy dashboard HTML page (monolithic 4,251 lines).
pub fn dashboard_html() -> &'static str {
    include_str!("dashboard.html")
}

/// New Preact-based dashboard shell.
pub fn dashboard_v2_html() -> &'static str {
    include_str!("dashboard/index.html")
}

/// Static files for the new dashboard (served at /static/dashboard/*).
/// Returns a HashMap of (path, content, content_type).
pub fn dashboard_static_files() -> HashMap<&'static str, (&'static str, &'static str)> {
    let mut files: HashMap<&str, (&str, &str)> = HashMap::new();

    // CSS
    files.insert(
        "/static/dashboard/styles.css",
        (include_str!("dashboard/styles.css"), "text/css; charset=utf-8"),
    );

    // Main app
    files.insert(
        "/static/dashboard/app.js",
        (
            include_str!("dashboard/app.js"),
            "application/javascript; charset=utf-8",
        ),
    );

    // i18n
    files.insert(
        "/static/dashboard/i18n/vi.js",
        (
            include_str!("dashboard/i18n/vi.js"),
            "application/javascript; charset=utf-8",
        ),
    );
    files.insert(
        "/static/dashboard/i18n/en.js",
        (
            include_str!("dashboard/i18n/en.js"),
            "application/javascript; charset=utf-8",
        ),
    );

    files
}
