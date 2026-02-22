//! # BizClaw Platform — Multi-Tenant Admin Server
//!
//! Manages multiple BizClaw AI Agent instances on a single VPS.
//! Provides admin dashboard, REST API, tenant lifecycle, and audit logging.
//!
//! Usage:
//!   bizclaw-platform                     # Start admin server (default port 3000)
//!   bizclaw-platform --port 8080         # Custom port
//!   bizclaw-platform --init-admin        # Create default admin user

use anyhow::Result;
use clap::Parser;
use std::sync::{Arc, Mutex};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "bizclaw-platform",
    version,
    about = "🏢 BizClaw Platform — Multi-Tenant Admin Server"
)]
struct Cli {
    /// Admin panel port
    #[arg(short, long, default_value = "3000")]
    port: u16,

    /// Path to bizclaw binary (for starting tenants)
    #[arg(long, default_value = "bizclaw")]
    bizclaw_bin: String,

    /// Base port for tenant instances
    #[arg(long, default_value = "10001")]
    base_port: u16,

    /// Data directory
    #[arg(long, default_value = "~/.bizclaw/tenants")]
    data_dir: String,

    /// Database path
    #[arg(long, default_value = "~/.bizclaw/platform.db")]
    db_path: String,

    /// JWT secret
    #[arg(long, default_value = "bizclaw-platform-secret-2026")]
    jwt_secret: String,

    /// Create default admin user and exit
    #[arg(long)]
    init_admin: bool,

    /// Admin email (used with --init-admin)
    #[arg(long, default_value = "admin@bizclaw.vn")]
    admin_email: String,

    /// Admin password (used with --init-admin)
    #[arg(long, default_value = "BizClaw@2026")]
    admin_password: String,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

fn expand_path(p: &str) -> String {
    shellexpand::tilde(p).to_string()
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let filter = if cli.verbose {
        "bizclaw_platform=debug,tower_http=debug"
    } else {
        "bizclaw_platform=info"
    };
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter)))
        .with_target(false)
        .init();

    // Expand paths
    let data_dir = expand_path(&cli.data_dir);
    let db_path = expand_path(&cli.db_path);

    // Ensure directories exist
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::create_dir_all(&data_dir)?;

    // Open database
    let db = bizclaw_platform::PlatformDb::open(std::path::Path::new(&db_path))?;

    // --init-admin: create admin user and exit
    if cli.init_admin {
        println!("🏢 BizClaw Platform — Admin Setup\n");

        // Check if admin already exists
        match db.get_user_by_email(&cli.admin_email) {
            Ok(Some(_)) => {
                println!("⚠️  Admin '{}' already exists.", cli.admin_email);
            }
            _ => {
                let hash = bizclaw_platform::auth::hash_password(&cli.admin_password)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                let id = db.create_user(&cli.admin_email, &hash, "admin")?;
                db.log_event("admin_created", "system", &id, Some(&format!("email={}", cli.admin_email))).ok();
                println!("✅ Admin user created:");
                println!("   Email:    {}", cli.admin_email);
                println!("   Password: {}", cli.admin_password);
                println!("   Role:     admin");
            }
        }
        return Ok(());
    }

    // Ensure at least one admin exists — auto-create on first run
    let users = db.list_users().unwrap_or_default();
    if users.is_empty() {
        println!("📝 No admin users found. Creating default admin...");
        let hash = bizclaw_platform::auth::hash_password("BizClaw@2026")
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        db.create_user("admin@bizclaw.vn", &hash, "admin")?;
        println!("   Email:    admin@bizclaw.vn");
        println!("   Password: BizClaw@2026");
        println!("   ⚠️  Change this password after first login!\n");
    }

    // Build admin state
    let state = Arc::new(bizclaw_platform::admin::AdminState {
        db: Mutex::new(db),
        manager: Mutex::new(bizclaw_platform::TenantManager::new(&data_dir)),
        jwt_secret: cli.jwt_secret.clone(),
        bizclaw_bin: cli.bizclaw_bin.clone(),
        base_port: cli.base_port,
        login_attempts: Mutex::new(std::collections::HashMap::new()),
    });

    // Start server
    println!("🏢 BizClaw Platform v{}", env!("CARGO_PKG_VERSION"));
    println!("   🌐 Admin Dashboard: http://0.0.0.0:{}", cli.port);
    println!("   📡 API:             http://0.0.0.0:{}/api/admin/stats", cli.port);
    println!("   🗄️  Database:        {db_path}");
    println!("   📂 Data Dir:        {data_dir}");
    println!("   🔧 BizClaw Binary:  {}", cli.bizclaw_bin);
    println!("   🔌 Tenant Base Port: {}", cli.base_port);
    println!();

    // Auto-restart tenants that were previously running
    {
        let db_lock = state.db.lock().unwrap();
        match db_lock.list_tenants() {
            Ok(tenants) => {
                let running: Vec<_> = tenants.iter()
                    .filter(|t| t.status == "running")
                    .collect();
                if !running.is_empty() {
                    println!("🔄 Auto-restarting {} tenant(s)...", running.len());
                    // We need to keep our own Vec since we're about to drop the lock
                    let running_owned: Vec<_> = running.into_iter().cloned().collect();
                    drop(db_lock); // Release lock before starting tenants
                    for tenant in &running_owned {
                        let db = state.db.lock().unwrap();
                        let mut mgr = state.manager.lock().unwrap();
                        match mgr.start_tenant(tenant, &state.bizclaw_bin, &db) {
                            Ok(pid) => {
                                println!("   ✅ {} (port {}) → pid {}", tenant.name, tenant.port, pid);
                                db.update_tenant_status(&tenant.id, "running", Some(pid)).ok();
                            }
                            Err(e) => {
                                println!("   ❌ {} failed: {}", tenant.name, e);
                                db.update_tenant_status(&tenant.id, "error", None).ok();
                            }
                        }
                    }
                }
            }
            Err(e) => println!("⚠️ Failed to load tenants: {e}"),
        }
    }

    bizclaw_platform::AdminServer::start(state, cli.port).await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    Ok(())
}
