use crate::model::NodeTarget;
use anyhow::{bail, Context, Result};
use encoding_rs::{Encoding, UTF_8};
use globset::{Glob, GlobSet, GlobSetBuilder};
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grammar {
    TypeScript,
}

impl Grammar {
    fn from_str(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "typescript" => Ok(Self::TypeScript),
            other => bail!("Unsupported grammar '{other}'"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub show_passed: bool,
    pub project_root_path: PathBuf,
    pub threshold: Option<usize>,
    pub show_progress: bool,
    pub analyze_directory: PathBuf,
    pub lsp_executable: String,
    pub lsp_args: Vec<String>,
    pub file_matching_glob: GlobSet,
    pub file_exclude_glob: GlobSet,
    pub content_pattern: Option<Regex>,
    pub lsp_capabilities: Option<Value>,
    pub initialization_options: Value,
    pub reference_nodes: Vec<NodeTarget>,
    pub lsp_version: String,
    pub lsp_name: String,
    pub grammar: Grammar,
    pub encoding: &'static Encoding,
    pub encoding_label: String,
}

impl AppConfig {
    pub fn read_source(&self, path: &Path) -> Result<String> {
        let bytes = fs::read(path)
            .with_context(|| format!("Failed to read source file {}", path.display()))?;

        let (content, _, had_errors) = self.encoding.decode(&bytes);
        if had_errors {
            tracing::warn!(
                "Decoding errors while reading {} with encoding {}",
                path.display(),
                self.encoding_label
            );
        }
        Ok(content.into_owned())
    }

    pub fn summary(&self) -> String {
        format!(
            "root={}, analyze={}, grammar={:?}, lsp={} {:#?}",
            self.project_root_path.display(),
            self.analyze_directory.display(),
            self.grammar,
            self.lsp_executable,
            self.lsp_args
        )
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawNodeTarget {
    #[serde(rename = "type")]
    node_type: Option<String>,
    #[serde(default)]
    ref_type: Option<String>,
    #[serde(default)]
    children: Vec<RawNodeTarget>,
}

impl RawNodeTarget {
    fn into_target(self) -> Option<NodeTarget> {
        let node_type = self.node_type?;
        Some(NodeTarget {
            node_type,
            ref_type: self.ref_type,
            children: self
                .children
                .into_iter()
                .filter_map(|child| child.into_target())
                .collect(),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawConfig {
    #[serde(default)]
    show_passed: bool,
    project_root_path: PathBuf,
    #[serde(default)]
    threshold: Option<usize>,
    #[serde(default)]
    show_progress: bool,
    analyze_directory: PathBuf,
    lsp_executable: String,
    #[serde(default)]
    lsp_args: Vec<String>,
    #[serde(default = "default_file_glob")]
    file_matching_regexp: String,
    #[serde(default)]
    file_exclude_regexps: Vec<String>,
    #[serde(default)]
    content_matching_regexp: Option<String>,
    #[serde(default)]
    lsp_capabilities: Option<Value>,
    #[serde(default = "default_initialization_options")]
    initialization_options: Value,
    reference_nodes: Vec<RawNodeTarget>,
    #[serde(default = "default_lsp_version")]
    lsp_version: String,
    lsp_name: String,
    grammar: String,
    #[serde(default = "default_encoding")]
    encoding: String,
}

fn default_file_glob() -> String {
    "**/*.*".to_string()
}

fn default_encoding() -> String {
    "utf-8".to_string()
}

fn default_initialization_options() -> Value {
    serde_json::json!({
        "tsserver": {
            "logDirectory": ".log",
            "logVerbosity": "verbose",
            "trace": "verbose"
        }
    })
}

fn default_lsp_version() -> String {
    "0.0.0".to_string()
}

pub fn load_config(path: &Path, threshold_override: Option<usize>) -> Result<AppConfig> {
    let raw_bytes = fs::read(path).with_context(|| format!("Failed to read {:?}", path))?;
    let RawConfig {
        show_passed,
        project_root_path,
        threshold: raw_threshold,
        show_progress,
        analyze_directory,
        lsp_executable,
        lsp_args,
        file_matching_regexp,
        file_exclude_regexps,
        content_matching_regexp,
        lsp_capabilities,
        initialization_options,
        reference_nodes,
        lsp_version,
        lsp_name,
        grammar,
        encoding,
    } = serde_json::from_slice(&raw_bytes)
        .with_context(|| format!("Invalid configuration JSON {:?}", path))?;

    let config_dir = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let project_root_path = absolutize(config_dir.clone(), project_root_path);
    let analyze_directory = absolutize(config_dir, analyze_directory);

    let grammar = Grammar::from_str(&grammar)?;
    let encoding_label = encoding;
    let encoding = resolve_encoding(&encoding_label)?;
    let threshold = threshold_override.or(raw_threshold);

    let include_patterns = vec![file_matching_regexp];
    let file_matching_glob = compile_glob(&include_patterns, Some("**/*"))?;
    let file_exclude_glob = compile_glob(&file_exclude_regexps, None)?;
    let content_pattern = content_matching_regexp
        .as_deref()
        .map(Regex::new)
        .transpose()
        .context("Invalid contentMatchingRegexp")?;

    let reference_nodes = reference_nodes
        .into_iter()
        .filter_map(|node| node.into_target())
        .collect();

    Ok(AppConfig {
        show_passed,
        project_root_path,
        threshold,
        show_progress,
        analyze_directory,
        lsp_executable,
        lsp_args,
        file_matching_glob,
        file_exclude_glob,
        content_pattern,
        lsp_capabilities,
        initialization_options,
        reference_nodes,
        lsp_version,
        lsp_name,
        grammar,
        encoding,
        encoding_label,
    })
}

fn absolutize(base: PathBuf, value: PathBuf) -> PathBuf {
    if value.is_absolute() {
        value
    } else {
        base.join(value)
    }
}

fn resolve_encoding(label: &str) -> Result<&'static Encoding> {
    let normalized = label.to_ascii_lowercase();
    let encoding = match normalized.as_str() {
        "ascii" => Encoding::for_label(b"us-ascii"),
        _ => Encoding::for_label(normalized.as_bytes()),
    }
    .unwrap_or(UTF_8);
    Ok(encoding)
}

fn compile_glob(patterns: &[String], default_pattern: Option<&str>) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    if patterns.is_empty() {
        if let Some(default) = default_pattern {
            builder.add(Glob::new(default)?);
        }
    } else {
        for pattern in patterns {
            builder.add(Glob::new(pattern)?);
        }
    }
    Ok(builder.build()?)
}
