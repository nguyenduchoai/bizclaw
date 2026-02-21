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
    Set {
        key: String,
        value: String,
    },
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
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter)))
        .with_target(false)
        .init();

    // Load config
    let mut config = if let Some(path) = &cli.config {
        bizclaw_core::BizClawConfig::load_from(std::path::Path::new(path))?
    } else {
        bizclaw_core::BizClawConfig::load()?
    };

    match cli.command {
        Commands::Agent { message, interactive, provider, model } => {
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
                println!("🦀 BizClaw v{} — Interactive Mode", env!("CARGO_PKG_VERSION"));
                println!("   Provider: {} | Model: {}", agent.provider_name(), "default");
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
                            let mut zalo = bizclaw_channels::zalo::ZaloChannel::new(zalo_config.clone());
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
                    println!("  {} zalo      — Zalo Personal/OA",
                        if config.channel.zalo.as_ref().is_some_and(|z| z.enabled) { "✅" } else { "⬜" });
                    println!("  {} telegram  — Telegram bot",
                        if config.channel.telegram.is_some() { "✅" } else { "⬜" });
                    println!("  {} discord   — Discord bot",
                        if config.channel.discord.is_some() { "✅" } else { "⬜" });
                }
            }
        }

        Commands::Onboard => {
            println!("🦀 BizClaw — First-time Setup\n");
            println!("Creating default configuration...");

            let config = bizclaw_core::BizClawConfig::default();
            config.save()?;
            println!("✅ Config saved to: {}", bizclaw_core::BizClawConfig::default_path().display());

            // Create directories
            let home = bizclaw_core::BizClawConfig::home_dir();
            std::fs::create_dir_all(home.join("models"))?;
            std::fs::create_dir_all(home.join("cache"))?;
            std::fs::create_dir_all(home.join("zalo"))?;
            println!("✅ Directories created");

            println!("\n📋 Next steps:");
            println!("  1. Set your API key: bizclaw config set api_key sk-...");
            println!("  2. Or use local brain: bizclaw brain download");
            println!("  3. Start chatting: bizclaw agent --interactive");
            println!("  4. For Zalo: configure ~/.bizclaw/config.toml [channel.zalo] section");
        }

        Commands::Brain { action } => {
            match action {
                BrainAction::Download { model } => {
                    println!("🧠 Downloading model: {model}");
                    println!("   (Model download not yet implemented — coming in Phase 2)");
                    println!("   Manual: download GGUF file to ~/.bizclaw/models/");
                }
                BrainAction::List => {
                    println!("🧠 Available models:");
                    println!("  - tinyllama-1.1b (recommended, ~638MB)");
                    println!("  - phi-2 (2.7B params)");
                    println!("  - llama-3.2-1b (1B params)");
                }
                BrainAction::Test { prompt } => {
                    println!("🧠 Testing brain inference...");
                    let engine = bizclaw_brain::BrainEngine::new(bizclaw_brain::BrainConfig::default());
                    match engine.generate(&prompt, 100) {
                        Ok(response) => println!("Response: {response}"),
                        Err(e) => println!("Error: {e}"),
                    }
                }
            }
        }

        Commands::Config { action } => {
            match action {
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
            }
        }

        Commands::Info => {
            println!("🦀 BizClaw v{}", env!("CARGO_PKG_VERSION"));
            println!("   Platform: {} / {}", std::env::consts::OS, std::env::consts::ARCH);
            println!("   Config: {}", bizclaw_core::BizClawConfig::default_path().display());
            println!("   Provider: {}", config.default_provider);
            println!("   Model: {}", config.default_model);
            println!("   Brain: {}", if config.brain.enabled { "enabled" } else { "disabled" });
            if let Some(zalo) = &config.channel.zalo {
                println!("   Zalo: {} ({})", if zalo.enabled { "enabled" } else { "disabled" }, zalo.mode);
            }
        }
    }

    Ok(())
}
