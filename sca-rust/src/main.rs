mod analyzer;
mod config;
mod model;
mod sanitize;

use crate::analyzer::report::build_report;
use crate::analyzer::{count_dead_entities, Analyzer};
use crate::config::load_config;
use anyhow::{Context, Result};
use clap::Parser;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "sca")]
#[command(about = "Smells Code Analyzer (Rust edition)")]
struct Cli {
    /// Path to JSON configuration file
    #[arg(short = 'c', long = "config-file")]
    config_file: PathBuf,

    /// Override threshold value from configuration
    #[arg(short = 't', long = "threshold")]
    threshold: Option<usize>,

    /// Path to a file containing newline separated file paths to analyse
    #[arg(long = "files-from", value_name = "PATH")]
    files_from: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let cli = Cli::parse();
    let start_time = Instant::now();

    let config = load_config(&cli.config_file, cli.threshold)
        .with_context(|| format!("Failed to load config {:?}", cli.config_file))?;

    let only_files = if let Some(files_list) = cli.files_from {
        Some(
            load_target_file_set(&files_list)
                .with_context(|| format!("Failed to read file list {}", files_list.display()))?,
        )
    } else {
        None
    };

    tracing::info!("Using configuration {}", config.summary());

    let mut analyzer = Analyzer::new(&config).await?;
    let files = analyzer::files::collect_files(&config, only_files.as_ref())?;

    println!("FILES TO ANALYZE: {}", files.len());

    let mut all_nodes = Vec::new();

    for (index, path) in files.iter().enumerate() {
        if config.show_progress {
            println!("Analyze [{}/{}] {}", index + 1, files.len(), path.display());
        }
        let nodes = analyzer
            .analyze_file(path)
            .await
            .with_context(|| format!("Failed to analyze {}", path.display()))?;

        if !nodes.is_empty() {
            let report = build_report(&nodes, config.show_passed);
            if !report.is_empty() {
                println!("{report}");
            }
        }
        all_nodes.extend(nodes);
    }

    let dead_count = count_dead_entities(&all_nodes);
    println!("Found {} dead entities", dead_count);

    if let Some(threshold) = config.threshold {
        if dead_count > threshold {
            analyzer.shutdown().await?;
            anyhow::bail!(
                "Found {} dead entities, threshold is {}",
                dead_count,
                threshold
            );
        }
    }

    analyzer.shutdown().await?;
    let elapsed = start_time.elapsed().as_secs_f64();
    println!("Analyze took {elapsed:.3} s");
    Ok(())
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .try_init();
}

fn load_target_file_set(path: &Path) -> Result<HashSet<PathBuf>> {
    let list_dir = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let raw = fs::read_to_string(path)?;
    let mut targets = HashSet::new();

    for (idx, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let joined = if Path::new(trimmed).is_absolute() {
            PathBuf::from(trimmed)
        } else {
            list_dir.join(trimmed)
        };

        match fs::canonicalize(&joined) {
            Ok(abs) => {
                targets.insert(abs);
            }
            Err(err) => {
                tracing::debug!("Skipping target '{}' (line {}): {err}", trimmed, idx + 1);
            }
        }
    }

    Ok(targets)
}
