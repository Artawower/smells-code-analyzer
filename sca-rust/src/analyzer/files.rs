use crate::config::AppConfig;
use anyhow::{Context, Result};
use ignore::{DirEntry, WalkBuilder};
use std::path::{Path, PathBuf};

pub fn collect_files(config: &AppConfig) -> Result<Vec<PathBuf>> {
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

    files.sort();
    Ok(files)
}

pub fn count_files(config: &AppConfig) -> Result<usize> {
    let mut count = 0usize;
    let walker = build_walker(&config.analyze_directory);
    for entry in walker {
        let entry = entry?;
        if !is_regular_file(&entry) {
            continue;
        }
        let path = entry.path();
        if !config.file_matching_glob.is_match(path) {
            continue;
        }
        if config.file_exclude_glob.is_match(path) {
            continue;
        }
        count += 1;
    }
    Ok(count)
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
