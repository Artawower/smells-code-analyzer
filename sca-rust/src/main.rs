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

    /// Generate snapshot of errors to JSON file
    #[arg(long = "generate-snapshot", value_name = "PATH")]
    generate_snapshot: Option<PathBuf>,

    /// Compare with previous snapshot and show only new errors
    #[arg(long = "compare-snapshot", value_name = "PATH")]
    compare_snapshot: Option<PathBuf>,
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

    if let Some(snapshot_path) = cli.generate_snapshot {
        generate_snapshot(&all_nodes, &snapshot_path)?;
        println!("Snapshot saved to {}", snapshot_path.display());
    }

    if let Some(snapshot_path) = cli.compare_snapshot {
        let new_errors = compare_with_snapshot(&all_nodes, &snapshot_path)?;
        if !new_errors.is_empty() {
            println!("\nNew errors found:");
            for error in &new_errors {
                println!("{}", format_error(error));
            }
            analyzer.shutdown().await?;
            anyhow::bail!("Found {} new errors", new_errors.len());
        } else {
            println!("No new errors found");
        }
    }

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

fn generate_snapshot(nodes: &[model::FullNodeInfo], path: &Path) -> Result<()> {
    let errors: Vec<_> = nodes
        .iter()
        .flat_map(|node| collect_errors(node))
        .collect();

    let json = serde_json::to_string_pretty(&errors)?;
    fs::write(path, json)?;
    Ok(())
}

fn collect_errors(node: &model::FullNodeInfo) -> Vec<model::FullNodeInfo> {
    let mut errors = Vec::new();

    if node.references == 0 || node.parent_name_prefix {
        errors.push(node.clone());
    }

    for child in &node.children {
        errors.extend(collect_errors(child));
    }

    errors
}

fn compare_with_snapshot(
    nodes: &[model::FullNodeInfo],
    snapshot_path: &Path,
) -> Result<Vec<model::FullNodeInfo>> {
    let snapshot_content = fs::read_to_string(snapshot_path)
        .with_context(|| format!("Failed to read snapshot {}", snapshot_path.display()))?;

    let old_errors: Vec<model::FullNodeInfo> = serde_json::from_str(&snapshot_content)
        .with_context(|| format!("Failed to parse snapshot {}", snapshot_path.display()))?;

    let current_errors: Vec<_> = nodes
        .iter()
        .flat_map(|node| collect_errors(node))
        .collect();

    let mut new_errors = Vec::new();

    for current in &current_errors {
        let is_new = !old_errors.iter().any(|old| {
            old.file_path == current.file_path
                && old.name == current.name
                && old.start_position.row == current.start_position.row
                && old.start_position.column == current.start_position.column
        });

        if is_new {
            new_errors.push(current.clone());
        }
    }

    Ok(new_errors)
}

fn format_error(error: &model::FullNodeInfo) -> String {
    let mut reasons = Vec::new();
    if error.references == 0 {
        reasons.push("dead code");
    }
    if error.parent_name_prefix {
        reasons.push("useless prefix");
    }
    let reason_str = reasons.join(", ");

    format!(
        "{}:{}:{} :: {} ({})",
        error.file_path.display(),
        error.start_position.row,
        error.start_position.column,
        error.name,
        reason_str
    )
}
