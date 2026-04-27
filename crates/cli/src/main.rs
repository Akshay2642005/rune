mod client;
mod config;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use client::RuneClient;
use config::RuneConfig;

// ── CLI definition ────────────────────────────────────────────────────────────

#[derive(Debug, Parser)]
#[command(name = "rune", about = "Rune deployment CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Deploy a WASM function to the running rune-server.
    Deploy {
        /// Function identifier (unique, URL-safe).
        #[arg(long)]
        id: String,
        /// URL path to serve the function at (e.g. /hello).
        #[arg(long)]
        route: String,
        /// Subdomain label (e.g. "hello" → hello.<base_domain>).
        #[arg(long)]
        subdomain: Option<String>,
        /// Path to the compiled .wasm artifact.
        wasm: PathBuf,
    },

    /// List all deployed functions.
    List,

    /// Remove a deployed function by id.
    Remove { id: String },

    /// Manage authentication config and API keys.
    #[command(subcommand)]
    Auth(AuthCommands),
}

#[derive(Debug, Subcommand)]
enum AuthCommands {
    /// Save server URL and API key to ~/.config/rune/config.toml.
    Save {
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        key: String,
    },

    /// Show current config (key is masked).
    Show,

    /// Generate a new API key on the server.
    GenerateKey {
        #[arg(long, default_value = "cli")]
        name: String,
    },

    /// List API keys registered on the server.
    ListKeys,

    /// Revoke an API key by its id.
    RevokeKey { id: String },
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let cfg = RuneConfig::load()?;

    match cli.command {
        // ── Deploy ────────────────────────────────────────────────────────────
        Commands::Deploy {
            id,
            route,
            subdomain,
            wasm,
        } => {
            let c = rune_client(&cfg)?;
            let f = c.deploy(&id, &route, subdomain.as_deref(), &wasm).await?;
            println!("deployed '{}'", f.id);
            println!("  route:     {}", f.route);
            if let Some(sub) = f.subdomain {
                println!("  subdomain: {sub}");
            }
        }

        // ── List ──────────────────────────────────────────────────────────────
        Commands::List => {
            let c = rune_client(&cfg)?;
            let functions = c.list_functions().await?;
            if functions.is_empty() {
                println!("no functions deployed");
            } else {
                println!("{:<20} {:<25} {}", "ID", "ROUTE", "SUBDOMAIN");
                println!("{}", "─".repeat(60));
                for f in functions {
                    println!(
                        "{:<20} {:<25} {}",
                        f.id,
                        f.route,
                        f.subdomain.as_deref().unwrap_or("—")
                    );
                }
            }
        }

        // ── Remove ────────────────────────────────────────────────────────────
        Commands::Remove { id } => {
            let c = rune_client(&cfg)?;
            c.delete_function(&id).await?;
            println!("removed '{id}'");
        }

        // ── Auth subcommands ──────────────────────────────────────────────────
        Commands::Auth(auth_cmd) => match auth_cmd {
            AuthCommands::Save { url, key } => {
                let mut cfg = RuneConfig::load()?;
                if let Some(url) = url.as_ref() {
                    cfg.server_url = Some(url.clone());
                }
                cfg.api_key = Some(key);
                cfg.save()?;
                if let Some(url) = url {
                    println!("config saved (server: {url})");
                } else {
                    println!("config saved");
                }
            }

            AuthCommands::Show => {
                let url = cfg.server_url.as_deref().unwrap_or("(not set)");
                let key = cfg.api_key.as_deref().unwrap_or("(not set)");
                let masked = if key.len() > 12 {
                    format!("{}…{}", &key[..12], "*".repeat(8))
                } else {
                    key.to_string()
                };
                println!("server_url: {url}");
                println!("api_key:    {masked}");
            }

            AuthCommands::GenerateKey { name } => {
                let c = rune_client(&cfg)?;
                let k = c.create_key(&name).await?;
                println!("created key '{name}'");
                println!("  id:  {}", k.id);
                println!("  key: {}", k.key);
                println!();
                println!("Save it with:  rune auth save --key {}", k.key);
            }

            AuthCommands::ListKeys => {
                let c = rune_client(&cfg)?;
                let keys = c.list_keys().await?;
                if keys.is_empty() {
                    println!("no API keys");
                } else {
                    println!("{:<38} {}", "ID", "NAME");
                    println!("{}", "─".repeat(50));
                    for k in keys {
                        println!("{:<38} {}", k.id, k.name);
                    }
                }
            }

            AuthCommands::RevokeKey { id } => {
                let c = rune_client(&cfg)?;
                c.revoke_key(&id).await?;
                println!("revoked key '{id}'");
            }
        },
    }

    Ok(())
}

fn rune_client(cfg: &RuneConfig) -> anyhow::Result<RuneClient> {
    Ok(RuneClient::new(
        cfg.require_server_url()?,
        cfg.require_api_key()?,
    ))
}
