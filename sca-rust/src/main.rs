mod analyzer;
mod config;
mod model;
mod sanitize;

use crate::analyzer::report::build_report;
use crate::analyzer::{count_dead_entities, Analyzer};
use crate::config::load_config;
use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
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
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let cli = Cli::parse();
    let start_time = Instant::now();

    let config = load_config(&cli.config_file, cli.threshold)
        .with_context(|| format!("Failed to load config {:?}", cli.config_file))?;

    tracing::info!("Using configuration {}", config.summary());

    let files_total = analyzer::files::count_files(&config)?;
    println!("FILES TO ANALYZE: {}", files_total);

    let mut analyzer = Analyzer::new(&config).await?;
    let files = analyzer::files::collect_files(&config)?;

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
