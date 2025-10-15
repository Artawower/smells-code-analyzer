use crate::config::AppConfig;
use anyhow::{Context, Result};
use ignore::{DirEntry, WalkBuilder};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tracing::debug;

pub fn collect_files(config: &AppConfig, only: Option<&HashSet<PathBuf>>) -> Result<Vec<PathBuf>> {
    let mut files = match only {
        Some(paths) => collect_from_list(config, paths)?,
        None => collect_from_walk(config)?,
    };

    files.sort();
    files.dedup();
    Ok(files)
}

fn collect_from_walk(config: &AppConfig) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let walker = build_walker(&config.analyze_directory);

    for entry in walker {
        let entry = entry?;
        if !is_regular_file(&entry) {
            continue;
        }
        let path = entry.into_path();
        if !config.file_matching_glob.is_match(&path) {
            continue;
        }
        if config.file_exclude_glob.is_match(&path) {
            continue;
        }
        if let Some(pattern) = &config.content_pattern {
            let content = config
                .read_source(&path)
                .with_context(|| format!("Failed to read {}", path.display()))?;
            if !pattern.is_match(&content) {
                continue;
            }
        }
        files.push(path);
    }

    Ok(files)
}

fn collect_from_list(config: &AppConfig, targets: &HashSet<PathBuf>) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for path in targets {
        if !path.starts_with(&config.analyze_directory) {
            debug!("Skip file outside analyze directory: {}", path.display());
            continue;
        }
        if !config.file_matching_glob.is_match(path) {
            debug!("Skip file not matching glob: {}", path.display());
            continue;
        }
        if config.file_exclude_glob.is_match(path) {
            debug!("Skip file excluded by glob: {}", path.display());
            continue;
        }
        if let Some(pattern) = &config.content_pattern {
            let content = config
                .read_source(path)
                .with_context(|| format!("Failed to read {}", path.display()))?;
            if !pattern.is_match(&content) {
                debug!("Skip file not matching content pattern: {}", path.display());
                continue;
            }
        }
        files.push(path.clone());
    }

    Ok(files)
}

fn build_walker(path: &Path) -> ignore::Walk {
    WalkBuilder::new(path)
        .standard_filters(false)
        .hidden(false)
        .follow_links(false)
        .build()
}

fn is_regular_file(entry: &DirEntry) -> bool {
    entry.file_type().map(|ft| ft.is_file()).unwrap_or(false)
}
