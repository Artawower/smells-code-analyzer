use std::path::PathBuf;
use tree_sitter::Point;

#[derive(Debug, Clone)]
pub struct NodeTarget {
    pub node_type: String,
    pub ref_type: Option<String>,
    pub children: Vec<NodeTarget>,
}

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub node_type: String,
    pub name: String,
    pub start_position: Point,
    pub children: Vec<NodeInfo>,
}

#[derive(Debug, Clone)]
pub struct FullNodeInfo {
    #[allow(dead_code)]
    pub node_type: String,
    pub name: String,
    pub start_position: Point,
    pub file_path: PathBuf,
    pub references: usize,
    pub parent_name_prefix: bool,
    pub children: Vec<FullNodeInfo>,
}
