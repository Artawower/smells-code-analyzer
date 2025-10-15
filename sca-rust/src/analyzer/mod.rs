pub mod files;
pub mod report;

mod lsp;
mod tree;

use crate::config::AppConfig;
use crate::model::{FullNodeInfo, NodeInfo};
use crate::sanitize::sanitize_source;
use anyhow::{anyhow, Context, Result};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use tree::TreeAnalyzer;
use url::Url;

pub struct Analyzer {
    config: AppConfig,
    tree_analyzer: TreeAnalyzer,
    lsp_client: lsp::LspClient,
    version: i32,
}

impl Analyzer {
    pub async fn new(config: &AppConfig) -> Result<Self> {
        let tree_analyzer = TreeAnalyzer::new(config.grammar, config.reference_nodes.clone())?;
        let lsp_client = lsp::LspClient::new(config).await?;
        Ok(Self {
            config: config.clone(),
            tree_analyzer,
            lsp_client,
            version: 1,
        })
    }

    pub async fn analyze_file(&mut self, path: &Path) -> Result<Vec<FullNodeInfo>> {
        let source = self.config.read_source(path)?;
        let sanitized = sanitize_source(&source);
        let nodes = self
            .tree_analyzer
            .find_positions(&sanitized)
            .with_context(|| format!("Tree-sitter failed for {}", path.display()))?;

        let uri = Url::from_file_path(path)
            .map_err(|_| anyhow!("Unable to convert path {} to URL", path.display()))?;

        self.lsp_client
            .did_open(&uri, &self.config.lsp_name, sanitized.clone(), self.version)
            .await?;
        self.version += 1;

        let full_nodes = self
            .enrich_nodes(&uri, path.to_path_buf(), nodes, None)
            .await?;

        self.lsp_client.did_close(&uri).await?;
        Ok(full_nodes)
    }

    fn enrich_nodes<'a>(
        &'a mut self,
        uri: &'a Url,
        path: PathBuf,
        nodes: Vec<NodeInfo>,
        parent_name: Option<String>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<FullNodeInfo>>> + Send + 'a>> {
        Box::pin(async move {
            let mut enriched = Vec::with_capacity(nodes.len());

            for node in nodes {
                let references = self
                    .lsp_client
                    .references(uri, node.start_position)
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to fetch references for {} at {}:{}",
                            path.display(),
                            node.start_position.row,
                            node.start_position.column
                        )
                    })?;

                let children = self
                    .enrich_nodes(uri, path.clone(), node.children, Some(node.name.clone()))
                    .await?;

                let parent_prefix = parent_name
                    .as_ref()
                    .map(|parent| node.name.to_lowercase().starts_with(&parent.to_lowercase()))
                    .unwrap_or(false);

                let references_count = references.saturating_sub(1);

                enriched.push(FullNodeInfo {
                    node_type: node.node_type,
                    name: node.name,
                    start_position: node.start_position,
                    file_path: path.clone(),
                    references: references_count,
                    parent_name_prefix: parent_prefix,
                    children,
                });
            }

            Ok(enriched)
        })
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        self.lsp_client.shutdown().await
    }
}

pub fn count_dead_entities(nodes: &[FullNodeInfo]) -> usize {
    nodes
        .iter()
        .map(|node| {
            let current = if node.references == 0 { 1 } else { 0 };
            current + count_dead_entities(&node.children)
        })
        .sum()
}
