mod api;
mod config;

use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "checkly")]
#[command(about = "CLI for querying Checkly check failures")]
struct Cli {
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Configure API credentials
    Config {
        #[arg(long)]
        api_key: Option<String>,
        #[arg(long)]
        account_id: Option<String>,
    },
    /// List all checks
    Checks,
    /// Show current status of all checks
    Status {
        /// Only show checks with failures
        #[arg(long)]
        failures_only: bool,
    },
    /// Show failures for a specific check
    Failures {
        /// Check ID
        check_id: String,
        /// Time range (e.g., 1h, 6h, 24h, 7d)
        #[arg(long, default_value = "6h")]
        since: String,
    },
}

fn get_client() -> Result<api::Client> {
    let cfg = config::load_config()?;
    let api_key = cfg.api_key.ok_or_else(|| {
        anyhow::anyhow!("API key not configured. Run 'checkly config --api-key <key>' first")
    })?;
    let account_id = cfg.account_id.ok_or_else(|| {
        anyhow::anyhow!("Account ID not configured. Run 'checkly config --account-id <id>' first")
    })?;
    api::Client::new(&api_key, &account_id)
}

fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim().to_lowercase();
    let (num, unit) = s.split_at(s.len() - 1);
    let num: i64 = num.parse().context("Invalid duration number")?;

    match unit {
        "h" => Ok(Duration::hours(num)),
        "d" => Ok(Duration::days(num)),
        "m" => Ok(Duration::minutes(num)),
        _ => anyhow::bail!(
            "Invalid duration unit '{}'. Use h (hours), d (days), or m (minutes)",
            unit
        ),
    }
}

fn configure(api_key: Option<String>, account_id: Option<String>) -> Result<()> {
    let mut cfg = config::load_config()?;
    if let Some(key) = api_key {
        cfg.api_key = Some(key);
    }
    if let Some(id) = account_id {
        cfg.account_id = Some(id);
    }
    config::save_config(&cfg)?;
    println!("Configuration saved.");
    Ok(())
}

async fn list_checks(json: bool) -> Result<()> {
    let client = get_client()?;
    let checks = client.list_checks().await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&checks)?);
    } else {
        println!("{:<40} {:<10} {:<10} {}", "ID", "TYPE", "ACTIVE", "NAME");
        println!("{}", "-".repeat(80));
        for check in checks {
            println!(
                "{:<40} {:<10} {:<10} {}",
                check.id,
                check.check_type,
                if check.activated { "yes" } else { "no" },
                check.name
            );
        }
    }
    Ok(())
}

async fn show_status(failures_only: bool, json: bool) -> Result<()> {
    let client = get_client()?;
    let statuses = client.get_statuses().await?;

    let filtered: Vec<_> = if failures_only {
        statuses
            .into_iter()
            .filter(|s| s.has_failures || s.has_errors)
            .collect()
    } else {
        statuses
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&filtered)?);
    } else {
        println!("{:<40} {:<8} {}", "ID", "STATUS", "NAME");
        println!("{}", "-".repeat(70));
        for status in filtered {
            let state = if status.has_errors {
                "ERROR"
            } else if status.has_failures {
                "FAILED"
            } else if status.is_degraded {
                "DEGRADED"
            } else {
                "OK"
            };
            println!("{:<40} {:<8} {}", status.check_id, state, status.name);
        }
    }
    Ok(())
}

async fn show_failures(check_id: String, since: String, json: bool) -> Result<()> {
    let client = get_client()?;
    let duration = parse_duration(&since)?;

    let now = Utc::now();
    let start = (now - duration).timestamp();
    let end = now.timestamp();

    let six_hours = 6 * 60 * 60;
    let mut all_results = Vec::new();
    let mut chunk_start = start;

    let mut is_first = true;
    while chunk_start < end {
        if !is_first {
            tokio::time::sleep(std::time::Duration::from_millis(2100)).await;
        }
        is_first = false;

        let chunk_end = (chunk_start + six_hours).min(end);
        let results = client
            .get_results(&check_id, Some(chunk_start), Some(chunk_end))
            .await?;
        all_results.extend(results);
        chunk_start = chunk_end;
    }

    let failures: Vec<_> = all_results
        .into_iter()
        .filter(|r| r.has_failures || r.has_errors)
        .collect();

    if json {
        println!("{}", serde_json::to_string_pretty(&failures)?);
    } else if failures.is_empty() {
        println!("No failures in the last {}", since);
    } else {
        println!(
            "{:<24} {:<10} {:<8} {:<6} {}",
            "TIME", "LOCATION", "STATUS", "MS", "RUN ID"
        );
        println!("{}", "-".repeat(70));
        for result in &failures {
            let time = result.started_at.as_deref().unwrap_or("-");
            let location = result.run_location.as_deref().unwrap_or("-");
            let status = if result.has_errors { "ERROR" } else { "FAILED" };
            let response_time = result
                .response_time
                .map(|t| t.to_string())
                .unwrap_or_else(|| "-".to_string());
            let run_id = result
                .check_run_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "-".to_string());

            println!(
                "{:<24} {:<10} {:<8} {:<6} {}",
                time, location, status, response_time, run_id
            );
        }
        println!("\nTotal: {} failure(s)", failures.len());
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Config {
            api_key,
            account_id,
        } => configure(api_key, account_id)?,
        Commands::Checks => list_checks(cli.json).await?,
        Commands::Status { failures_only } => show_status(failures_only, cli.json).await?,
        Commands::Failures { check_id, since } => show_failures(check_id, since, cli.json).await?,
    }

    Ok(())
}
