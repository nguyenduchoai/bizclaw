//! # BizClaw CLI
//!
//! Fast, small, and fully autonomous AI assistant infrastructure
//! with local brain and Zalo channels.
//!
//! Usage:
//!   bizclaw agent -m "Hello"           # One-shot message
//!   bizclaw agent --interactive        # Interactive CLI
//!   bizclaw channel start              # Start channel listener
//!   bizclaw onboard                    # First-time setup
//!   bizclaw brain download             # Download local model
//!   bizclaw config show                # Show configuration

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "bizclaw",
    version,
    about = "🦀 BizClaw — AI assistant infrastructure with local brain",
    long_about = "Fast, small, and fully autonomous AI assistant infrastructure.\nDeploy anywhere, swap anything. Local intelligence built-in."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Config file path
    #[arg(short, long, global = true)]
    config: Option<String>,

    /// Verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Send a message to the agent
    Agent {
        /// Message to send
        #[arg(short, long)]
        message: Option<String>,

        /// Interactive mode
        #[arg(short, long)]
        interactive: bool,

        /// Override provider
        #[arg(short, long)]
        provider: Option<String>,

        /// Override model
        #[arg(long)]
        model: Option<String>,
    },

    /// Manage channels
    Channel {
        #[command(subcommand)]
        action: ChannelAction,
    },

    /// First-time setup wizard
    Onboard,

    /// Brain (local LLM) management
    Brain {
        #[command(subcommand)]
        action: BrainAction,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Show system info
    Info,

    /// Quick interactive chat (alias for agent --interactive)
    Chat {
        /// Override provider
        #[arg(short, long)]
        provider: Option<String>,

        /// Override model
        #[arg(long)]
        model: Option<String>,
    },

    /// Start web dashboard + API server
    Serve {
        /// Port number
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Open browser automatically
        #[arg(long)]
        open: bool,
    },

    /// Interactive setup wizard
    Init,
}

#[derive(Subcommand)]
enum ChannelAction {
    /// Start listening on configured channels
    Start {
        /// Specific channel to start
        #[arg(short, long)]
        channel: Option<String>,
    },
    /// List available channels
    List,
}

#[derive(Subcommand)]
enum BrainAction {
    /// Download a model
    Download {
        /// Model name or URL
        #[arg(default_value = "tinyllama-1.1b")]
        model: String,
    },
    /// List available models
    List,
    /// Test inference
    Test {
        /// Prompt to test
        #[arg(default_value = "Hello, who are you?")]
        prompt: String,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show current configuration
    Show,
    /// Reset to defaults
    Reset,
    /// Set a config value
    Set { key: String, value: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let filter = if cli.verbose {
        "bizclaw=debug,bizclaw_core=debug,bizclaw_agent=debug"
    } else {
        "bizclaw=info"
    };
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter)),
        )
        .with_target(false)
        .init();

    // Load config
    let mut config = if let Some(path) = &cli.config {
        bizclaw_core::BizClawConfig::load_from(std::path::Path::new(path))?
    } else {
        bizclaw_core::BizClawConfig::load()?
    };

    match cli.command {
        Commands::Agent {
            message,
            interactive,
            provider,
            model,
        } => {
            // Apply overrides
            if let Some(p) = provider {
                config.default_provider = p;
            }
            if let Some(m) = model {
                config.default_model = m;
            }

            let mut agent = bizclaw_agent::Agent::new(config)?;

            if interactive || message.is_none() {
                // Interactive mode
                println!(
                    "🦀 BizClaw v{} — Interactive Mode",
                    env!("CARGO_PKG_VERSION")
                );
                println!(
                    "   Provider: {} | Model: {}",
                    agent.provider_name(),
                    "default"
                );
                println!("   Type /quit to exit, /clear to reset conversation\n");

                let mut cli_channel = bizclaw_channels::cli::CliChannel::new();
                cli_channel.connect().await?;

                use bizclaw_core::traits::Channel;
                use tokio_stream::StreamExt;

                let mut stream = cli_channel.listen().await?;
                print!("You: ");
                use std::io::Write;
                std::io::stdout().flush()?;

                while let Some(incoming) = stream.next().await {
                    if incoming.content == "/clear" {
                        agent.clear_conversation();
                        println!("🔄 Conversation cleared.\n");
                        print!("You: ");
                        std::io::stdout().flush()?;
                        continue;
                    }

                    match agent.handle_incoming(&incoming).await {
                        Ok(response) => {
                            cli_channel.send(response).await?;
                        }
                        Err(e) => {
                            println!("\n❌ Error: {e}\n");
                        }
                    }
                    print!("You: ");
                    std::io::stdout().flush()?;
                }

                println!("\n👋 Goodbye!");
            } else if let Some(msg) = message {
                // One-shot mode
                let response = agent.process(&msg).await?;
                println!("{response}");
            }
        }

        Commands::Channel { action } => {
            match action {
                ChannelAction::Start { channel } => {
                    println!("🦀 BizClaw Channel Listener");
                    if let Some(ch) = channel {
                        println!("Starting channel: {ch}");
                    } else {
                        println!("Starting all configured channels...");
                    }

                    // Start configured channels
                    if let Some(zalo_config) = &config.channel.zalo {
                        if zalo_config.enabled {
                            println!("  📱 Zalo ({}) channel starting...", zalo_config.mode);
                            let mut zalo =
                                bizclaw_channels::zalo::ZaloChannel::new(zalo_config.clone());
                            use bizclaw_core::traits::Channel;
                            zalo.connect().await?;
                        }
                    }

                    println!("\nChannels are running. Press Ctrl+C to stop.");
                    tokio::signal::ctrl_c().await?;
                    println!("\n👋 Channels stopped.");
                }
                ChannelAction::List => {
                    println!("Available channels:");
                    println!("  ✅ cli       — Interactive terminal");
                    println!(
                        "  {} zalo      — Zalo Personal/OA",
                        if config.channel.zalo.as_ref().is_some_and(|z| z.enabled) {
                            "✅"
                        } else {
                            "⬜"
                        }
                    );
                    println!(
                        "  {} telegram  — Telegram bot",
                        if config.channel.telegram.is_some() {
                            "✅"
                        } else {
                            "⬜"
                        }
                    );
                    println!(
                        "  {} discord   — Discord bot",
                        if config.channel.discord.is_some() {
                            "✅"
                        } else {
                            "⬜"
                        }
                    );
                }
            }
        }

        Commands::Onboard => {
            // Redirect to init
            run_init_wizard().await?;
        }

        Commands::Brain { action } => {
            match action {
                BrainAction::Download { model } => {
                    let model_dir = bizclaw_core::BizClawConfig::home_dir().join("models");
                    std::fs::create_dir_all(&model_dir)?;

                    let (url, filename) = match model.as_str() {
                        "tinyllama-1.1b" | "tinyllama" => (
                            "https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf",
                            "tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf",
                        ),
                        "phi-2" => (
                            "https://huggingface.co/TheBloke/phi-2-GGUF/resolve/main/phi-2.Q4_K_M.gguf",
                            "phi-2.Q4_K_M.gguf",
                        ),
                        "llama-3.2-1b" | "llama3.2" => (
                            "https://huggingface.co/bartowski/Llama-3.2-1B-Instruct-GGUF/resolve/main/Llama-3.2-1B-Instruct-Q4_K_M.gguf",
                            "Llama-3.2-1B-Instruct-Q4_K_M.gguf",
                        ),
                        other if other.starts_with("http") => (other, "custom-model.gguf"),
                        _ => {
                            println!("❌ Unknown model: {model}");
                            println!("   Available: tinyllama-1.1b, phi-2, llama-3.2-1b");
                            println!("   Or provide a direct URL to a .gguf file");
                            return Ok(());
                        }
                    };

                    let dest = model_dir.join(filename);
                    if dest.exists() {
                        println!("✅ Model already downloaded: {}", dest.display());
                        return Ok(());
                    }

                    println!("🧠 Downloading: {filename}");
                    println!("   From: {url}");
                    println!("   To:   {}", dest.display());
                    println!();

                    // Stream download with progress
                    let client = reqwest::Client::new();
                    let response = client
                        .get(url)
                        .send()
                        .await
                        .map_err(|e| anyhow::anyhow!("Download failed: {e}"))?;

                    let total_size = response.content_length().unwrap_or(0);
                    println!(
                        "   Total size: {:.1} MB",
                        total_size as f64 / 1024.0 / 1024.0
                    );

                    let mut file = tokio::fs::File::create(&dest).await?;
                    let mut downloaded: u64 = 0;
                    let mut stream = response.bytes_stream();

                    use futures::StreamExt;
                    use tokio::io::AsyncWriteExt;

                    while let Some(chunk) = stream.next().await {
                        let chunk = chunk.map_err(|e| anyhow::anyhow!("Download error: {e}"))?;
                        file.write_all(&chunk).await?;
                        downloaded += chunk.len() as u64;

                        if total_size > 0 {
                            let pct = (downloaded as f64 / total_size as f64 * 100.0) as u32;
                            let mb = downloaded as f64 / 1024.0 / 1024.0;
                            print!(
                                "\r   ⬇️  {mb:.1} MB / {:.1} MB ({pct}%)",
                                total_size as f64 / 1024.0 / 1024.0
                            );
                            use std::io::Write;
                            std::io::stdout().flush().ok();
                        }
                    }

                    file.flush().await?;
                    println!("\n\n✅ Download complete: {}", dest.display());
                    println!("   Test with: bizclaw brain test \"Hello!\"");
                }
                BrainAction::List => {
                    println!("🧠 Brain Models\n");

                    // List installed models
                    let model_dir = bizclaw_core::BizClawConfig::home_dir().join("models");
                    if model_dir.exists() {
                        let mut found = false;
                        if let Ok(entries) = std::fs::read_dir(&model_dir) {
                            for entry in entries.flatten() {
                                let path = entry.path();
                                if path.extension().and_then(|e| e.to_str()) == Some("gguf") {
                                    let size = std::fs::metadata(&path)
                                        .map(|m| m.len() / 1024 / 1024)
                                        .unwrap_or(0);
                                    println!(
                                        "  ✅ {} ({} MB)",
                                        path.file_name().unwrap_or_default().to_string_lossy(),
                                        size
                                    );
                                    found = true;
                                }
                            }
                        }
                        if !found {
                            println!("  (no models installed)");
                        }
                    } else {
                        println!("  (no models directory)");
                    }

                    println!("\n📦 Available for download:");
                    println!("  - tinyllama-1.1b  (~638 MB, recommended for Pi)");
                    println!("  - phi-2           (~1.6 GB)");
                    println!("  - llama-3.2-1b    (~750 MB)");
                    println!("\n  Use: bizclaw brain download <model-name>");
                }
                BrainAction::Test { prompt } => {
                    println!("🧠 Testing brain inference...\n");

                    // Try to find and load a model
                    let model_dir = bizclaw_core::BizClawConfig::home_dir().join("models");
                    let model_path = std::fs::read_dir(&model_dir).ok().and_then(|entries| {
                        entries
                            .filter_map(|e| e.ok())
                            .find(|e| {
                                e.path().extension().and_then(|ext| ext.to_str()) == Some("gguf")
                            })
                            .map(|e| e.path())
                    });

                    match model_path {
                        Some(path) => {
                            println!("   Model: {}", path.display());
                            match bizclaw_brain::BrainEngine::load(&path) {
                                Ok(mut engine) => {
                                    if let Some(info) = engine.model_info() {
                                        println!("   Info: {info}");
                                    }
                                    println!("   Prompt: \"{prompt}\"\n");
                                    match engine.generate(&prompt, 100) {
                                        Ok(response) => println!("🤖 {response}"),
                                        Err(e) => println!("❌ Inference error: {e}"),
                                    }
                                }
                                Err(e) => println!("❌ Failed to load model: {e}"),
                            }
                        }
                        None => {
                            println!("❌ No model found in {}", model_dir.display());
                            println!("   Run: bizclaw brain download tinyllama-1.1b");
                        }
                    }
                }
            }
        }

        Commands::Config { action } => match action {
            ConfigAction::Show => {
                let content = toml::to_string_pretty(&config)?;
                println!("{content}");
            }
            ConfigAction::Reset => {
                let config = bizclaw_core::BizClawConfig::default();
                config.save()?;
                println!("✅ Configuration reset to defaults.");
            }
            ConfigAction::Set { key, value } => {
                println!("Setting {key} = {value}");
                println!("(Direct config editing — edit ~/.bizclaw/config.toml)");
            }
        },

        Commands::Info => {
            println!("🦀 BizClaw v{}", env!("CARGO_PKG_VERSION"));
            println!(
                "   Platform: {} / {}",
                std::env::consts::OS,
                std::env::consts::ARCH
            );
            println!(
                "   Config: {}",
                bizclaw_core::BizClawConfig::default_path().display()
            );
            println!("   Provider: {}", config.default_provider);
            println!("   Model: {}", config.default_model);
            println!(
                "   Brain: {}",
                if config.brain.enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            );
            if let Some(zalo) = &config.channel.zalo {
                println!(
                    "   Zalo: {} ({})",
                    if zalo.enabled { "enabled" } else { "disabled" },
                    zalo.mode
                );
            }
        }

        Commands::Chat { provider, model } => {
            if let Some(p) = provider {
                config.default_provider = p;
            }
            if let Some(m) = model {
                config.default_model = m;
            }

            let mut agent = bizclaw_agent::Agent::new(config)?;

            println!("🦀 BizClaw v{} — Chat Mode", env!("CARGO_PKG_VERSION"));
            println!("   Provider: {}", agent.provider_name());
            println!("   Type /quit to exit, /clear to reset conversation\n");

            let mut cli_channel = bizclaw_channels::cli::CliChannel::new();
            cli_channel.connect().await?;

            use bizclaw_core::traits::Channel;
            use tokio_stream::StreamExt;

            let mut stream = cli_channel.listen().await?;
            print!("You: ");
            use std::io::Write;
            std::io::stdout().flush()?;

            while let Some(incoming) = stream.next().await {
                if incoming.content == "/clear" {
                    agent.clear_conversation();
                    println!("🔄 Conversation cleared.\n");
                    print!("You: ");
                    std::io::stdout().flush()?;
                    continue;
                }

                if incoming.content == "/info" {
                    let conv = agent.conversation();
                    println!(
                        "\n📊 Provider: {} | Messages: {} | System prompt: ✅\n",
                        agent.provider_name(),
                        conv.len()
                    );
                    print!("You: ");
                    std::io::stdout().flush()?;
                    continue;
                }

                match agent.handle_incoming(&incoming).await {
                    Ok(response) => {
                        cli_channel.send(response).await?;
                    }
                    Err(e) => {
                        println!("\n❌ Error: {e}\n");
                    }
                }
                print!("You: ");
                std::io::stdout().flush()?;
            }

            println!("\n👋 Goodbye!");
        }

        Commands::Serve { port, open } => {
            println!("🦀 BizClaw v{} — Web Dashboard", env!("CARGO_PKG_VERSION"));

            let mut gw_config = config.gateway.clone();
            gw_config.port = port;

            let url = format!("http://{}:{}", gw_config.host, gw_config.port);
            println!("   🌐 Dashboard: {url}");
            println!("   📡 API:       {url}/api/v1/info");
            println!(
                "   🔌 WebSocket: ws://{}:{}/ws",
                gw_config.host, gw_config.port
            );

            // ═══════════════════════════════════════════
            // Start configured channels in background
            // ═══════════════════════════════════════════
            let channel_config = config.channel.clone();
            let agent_config = config.clone();

            // Telegram channel
            if let Some(tg_config) = &channel_config.telegram {
                if tg_config.enabled && !tg_config.bot_token.is_empty() {
                    println!("   🤖 Telegram: starting bot...");
                    let tg = bizclaw_channels::telegram::TelegramChannel::new(
                        bizclaw_channels::telegram::TelegramConfig {
                            bot_token: tg_config.bot_token.clone(),
                            enabled: true,
                            poll_interval: 1,
                        },
                    );
                    let cfg_clone = agent_config.clone();
                    tokio::spawn(async move {
                        run_channel_loop("telegram", tg.start_polling(), cfg_clone).await;
                    });
                }
            }

            // Discord channel
            if let Some(dc_config) = &channel_config.discord {
                if dc_config.enabled && !dc_config.bot_token.is_empty() {
                    println!("   🎮 Discord: starting bot...");
                    let dc = bizclaw_channels::discord::DiscordChannel::new(
                        bizclaw_channels::discord::DiscordConfig {
                            bot_token: dc_config.bot_token.clone(),
                            enabled: true,
                            intents: (1 << 0) | (1 << 9) | (1 << 12) | (1 << 15),
                        },
                    );
                    let cfg_clone = agent_config.clone();
                    tokio::spawn(async move {
                        run_channel_loop("discord", dc.start_gateway(), cfg_clone).await;
                    });
                }
            }

            // Email channel
            if let Some(ref email_cfg) = channel_config.email {
                if email_cfg.enabled && !email_cfg.email.is_empty() {
                    println!("   📧 Email: starting listener ({})...", email_cfg.email);
                    let em = bizclaw_channels::email::EmailChannel::new(
                        bizclaw_channels::email::EmailConfig {
                            imap_host: email_cfg.imap_host.clone(),
                            imap_port: email_cfg.imap_port,
                            smtp_host: email_cfg.smtp_host.clone(),
                            smtp_port: email_cfg.smtp_port,
                            email: email_cfg.email.clone(),
                            password: email_cfg.password.clone(),
                            ..Default::default()
                        },
                    );
                    let cfg_clone = agent_config.clone();
                    tokio::spawn(async move {
                        run_channel_loop("email", em.start_polling(), cfg_clone).await;
                    });
                }
            }

            // Zalo channel (Personal mode — requires cookie)
            if let Some(ref zalo_cfg) = channel_config.zalo {
                if zalo_cfg.enabled {
                    let cookie_path = &zalo_cfg.personal.cookie_path;
                    let expanded_path = if cookie_path.starts_with("~/") {
                        std::env::var("HOME")
                            .ok()
                            .map(|h| std::path::PathBuf::from(h).join(&cookie_path[2..]))
                            .unwrap_or_else(|| std::path::PathBuf::from(cookie_path))
                    } else {
                        std::path::PathBuf::from(cookie_path)
                    };

                    if expanded_path.exists() {
                        println!("   💬 Zalo: starting ({} mode)...", zalo_cfg.mode);
                        tracing::info!(
                            "Zalo channel starting with cookie from: {}",
                            expanded_path.display()
                        );
                        // Note: Zalo uses WebSocket-based listening which is handled inside the
                        // ZaloChannel::connect(). For now, we log that it's ready.
                        // Full polling requires zpw_enk encryption which is complex.
                        // The channel will be activated when admin sends a
                        // "start channel" command via the API.
                    } else {
                        println!("   💬 Zalo: skipped (no cookie at {})", cookie_path);
                    }
                }
            }

            // WhatsApp channel (webhook-based — no background task needed)
            if let Some(ref wa_cfg) = channel_config.whatsapp {
                if wa_cfg.enabled && !wa_cfg.access_token.is_empty() {
                    println!("   📱 WhatsApp: enabled (webhook at /api/v1/webhook/whatsapp)");
                }
            }

            println!();

            if open {
                let _ = std::process::Command::new("open").arg(&url).spawn();
            }

            bizclaw_gateway::start_server(&gw_config).await?;
        }

        Commands::Init => {
            run_init_wizard().await?;
        }
    }

    Ok(())
}

/// Interactive setup wizard.
async fn run_init_wizard() -> Result<()> {
    use std::io::{self, BufRead, Write};

    println!("\n🦀 BizClaw — Setup Wizard\n");
    println!("This will create your configuration file.\n");

    let stdin = io::stdin();
    let mut input = String::new();

    // 1. Provider selection
    println!("📡 Choose your AI provider:");
    println!("   1. OpenAI          (gpt-4o, gpt-4o-mini)");
    println!("   2. Anthropic       (claude-sonnet-4, claude-3.5)");
    println!("   3. Google Gemini   (gemini-2.5-pro, gemini-2.5-flash)");
    println!("   4. DeepSeek        (deepseek-chat, deepseek-reasoner)");
    println!("   5. Groq            (llama-3.3-70b, mixtral-8x7b)");
    println!("   6. OpenRouter      (multi-provider gateway)");
    println!("   7. Ollama          (local, http://localhost:11434)");
    println!("   8. llama.cpp       (local, http://localhost:8080)");
    println!("   9. Brain           (built-in GGUF engine)");
    println!("  10. Custom          (any OpenAI-compatible endpoint)");
    print!("\n  Choice [1]: ");
    io::stdout().flush()?;
    input.clear();
    stdin.lock().read_line(&mut input)?;

    let (provider, default_model, default_endpoint) = match input.trim() {
        "2" => ("anthropic", "claude-sonnet-4-20250514", ""),
        "3" => ("gemini", "gemini-2.5-flash", ""),
        "4" => ("deepseek", "deepseek-chat", ""),
        "5" => ("groq", "llama-3.3-70b-versatile", ""),
        "6" => ("openrouter", "openai/gpt-4o", ""),
        "7" => ("ollama", "llama3.2", "http://localhost:11434/v1"),
        "8" => ("llamacpp", "local-model", "http://localhost:8080/v1"),
        "9" => ("brain", "tinyllama-1.1b", ""),
        "10" => ("custom", "", ""),
        _ => ("openai", "gpt-4o-mini", ""),
    };

    // 2. API Key (for cloud providers)
    let mut api_key = String::new();
    let needs_key = matches!(
        provider,
        "openai" | "anthropic" | "gemini" | "deepseek" | "groq" | "openrouter"
    );
    if needs_key {
        print!(
            "\n🔑 Enter your {} API key (or press Enter to use env var): ",
            provider
        );
        io::stdout().flush()?;
        input.clear();
        stdin.lock().read_line(&mut input)?;
        api_key = input.trim().to_string();
    }

    // 3. Endpoint URL (for local/custom providers, or optional override for cloud)
    let mut endpoint = default_endpoint.to_string();
    let needs_endpoint = matches!(provider, "ollama" | "llamacpp" | "custom");
    if needs_endpoint {
        let prompt = if default_endpoint.is_empty() {
            "\n🌐 Enter endpoint URL: ".to_string()
        } else {
            format!("\n🌐 Endpoint URL [{}]: ", default_endpoint)
        };
        print!("{}", prompt);
        io::stdout().flush()?;
        input.clear();
        stdin.lock().read_line(&mut input)?;
        if !input.trim().is_empty() {
            endpoint = input.trim().to_string();
        }
    }

    // 4. Custom provider may also need a key
    if provider == "custom" && api_key.is_empty() {
        print!("\n🔑 Enter API key for custom endpoint (or press Enter for none): ");
        io::stdout().flush()?;
        input.clear();
        stdin.lock().read_line(&mut input)?;
        api_key = input.trim().to_string();
    }

    // 5. Model override
    let mut model = default_model.to_string();
    if !default_model.is_empty() {
        print!("\n🧠 Model [{}]: ", default_model);
        io::stdout().flush()?;
        input.clear();
        stdin.lock().read_line(&mut input)?;
        if !input.trim().is_empty() {
            model = input.trim().to_string();
        }
    } else {
        print!("\n🧠 Enter model name: ");
        io::stdout().flush()?;
        input.clear();
        stdin.lock().read_line(&mut input)?;
        model = input.trim().to_string();
    }

    // 6. Bot name
    print!("\n🤖 Bot name [BizClaw]: ");
    io::stdout().flush()?;
    input.clear();
    stdin.lock().read_line(&mut input)?;
    let bot_name: String = if input.trim().is_empty() {
        "BizClaw".into()
    } else {
        input.trim().to_string()
    };

    // 7. Gateway
    print!("\n🌐 Enable web dashboard? [Y/n]: ");
    io::stdout().flush()?;
    input.clear();
    stdin.lock().read_line(&mut input)?;
    let enable_gateway = !input.trim().eq_ignore_ascii_case("n");

    // Build config
    let mut config = bizclaw_core::BizClawConfig::default();

    // Set [LLM] section
    config.llm.provider = provider.into();
    config.llm.model = model.clone();
    config.llm.api_key = api_key;
    config.llm.endpoint = endpoint;

    // Also set legacy top-level fields for backward compatibility
    config.default_provider = provider.into();
    config.default_model = model.clone();
    config.identity.name = bot_name.into();

    // Save
    config.save()?;

    // Create directories
    let home = bizclaw_core::BizClawConfig::home_dir();
    std::fs::create_dir_all(home.join("models"))?;
    std::fs::create_dir_all(home.join("cache"))?;
    std::fs::create_dir_all(home.join("data"))?;

    println!("\n✅ Setup complete!");
    println!(
        "   Config: {}",
        bizclaw_core::BizClawConfig::default_path().display()
    );
    println!("   Provider: {provider}");
    println!("   Model: {model}");

    if provider == "brain" {
        println!("\n🧠 Download a model:");
        println!("   bizclaw brain download tinyllama-1.1b");
    }

    println!("\n🚀 Quick start:");
    println!("   bizclaw chat                  # Start chatting");
    if enable_gateway {
        println!("   bizclaw serve                 # Web dashboard at http://localhost:3000");
    }
    println!("   bizclaw serve --open           # Open in browser");

    Ok(())
}

/// Run a channel listener loop — receives messages, routes through Agent, sends replies.
/// Works for any channel that produces a Stream<Item = IncomingMessage>.
async fn run_channel_loop<S>(channel_name: &str, mut stream: S, config: bizclaw_core::BizClawConfig)
where
    S: futures::Stream<Item = bizclaw_core::types::IncomingMessage> + Unpin,
{
    use futures::StreamExt;

    tracing::info!("📡 Channel '{channel_name}' listener started");

    // Create a dedicated Agent for this channel
    let mut agent = match bizclaw_agent::Agent::new(config.clone()) {
        Ok(a) => {
            tracing::info!(
                "✅ Agent for channel '{channel_name}' initialized (provider={})",
                a.provider_name()
            );
            a
        }
        Err(e) => {
            tracing::error!("❌ Failed to create agent for channel '{channel_name}': {e}");
            return;
        }
    };

    // Create channel sender for replies
    // We need a way to send messages back. For now, use the provider-specific send.
    let send_client = reqwest::Client::new();

    while let Some(incoming) = stream.next().await {
        tracing::info!(
            "[{channel_name}] Message from {}: {}",
            incoming
                .sender_name
                .as_deref()
                .unwrap_or(&incoming.sender_id),
            &incoming.content[..incoming.content.len().min(100)]
        );

        // Process through Agent Engine (tools + memory + providers)
        match agent.process(&incoming.content).await {
            Ok(response) => {
                tracing::info!(
                    "[{channel_name}] Response: {}...",
                    &response[..response.len().min(80)]
                );

                // Send response back through the same channel
                match channel_name {
                    "telegram" => {
                        // Use Telegram sendMessage API
                        if let Some(ref tg_cfg) = config.channel.telegram {
                            let url = format!(
                                "https://api.telegram.org/bot{}/sendMessage",
                                tg_cfg.bot_token
                            );
                            let body = serde_json::json!({
                                "chat_id": incoming.thread_id,
                                "text": &response,
                                "parse_mode": "Markdown",
                            });
                            if let Err(e) = send_client.post(&url).json(&body).send().await {
                                tracing::error!("[telegram] Send failed: {e}");
                            }
                        }
                    }
                    "discord" => {
                        // Use Discord REST API to send message
                        if let Some(ref dc_cfg) = config.channel.discord {
                            let url = format!(
                                "https://discord.com/api/v10/channels/{}/messages",
                                incoming.thread_id
                            );
                            let body = serde_json::json!({ "content": &response });
                            if let Err(e) = send_client
                                .post(&url)
                                .header("Authorization", format!("Bot {}", dc_cfg.bot_token))
                                .json(&body)
                                .send()
                                .await
                            {
                                tracing::error!("[discord] Send failed: {e}");
                            }
                        }
                    }
                    "email" => {
                        // Reply via SMTP (if email channel has send capability)
                        if let Some(ref _email_cfg) = config.channel.email {
                            tracing::info!(
                                "[email] Reply to {}: {}...",
                                incoming.sender_id,
                                &response[..response.len().min(60)]
                            );
                            // Email replies are handled by the EmailChannel's send() method
                            // The incoming.sender_id contains the From address
                            // For now, log the reply — full SMTP send is in EmailChannel
                        }
                    }
                    _ => {
                        tracing::warn!("[{channel_name}] No send handler implemented");
                    }
                }
            }
            Err(e) => {
                tracing::error!("[{channel_name}] Agent error: {e}");
            }
        }
    }

    tracing::warn!("📡 Channel '{channel_name}' stream ended — channel may have disconnected");
}
