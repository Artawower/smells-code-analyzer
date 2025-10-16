use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullNodeInfo {
    #[allow(dead_code)]
    pub node_type: String,
    pub name: String,
    #[serde(with = "point_serde")]
    pub start_position: Point,
    pub file_path: PathBuf,
    pub references: usize,
    pub parent_name_prefix: bool,
    pub children: Vec<FullNodeInfo>,
}

mod point_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use tree_sitter::Point;

    #[derive(Serialize, Deserialize)]
    struct PointData {
        row: usize,
        column: usize,
    }

    pub fn serialize<S>(point: &Point, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let data = PointData {
            row: point.row,
            column: point.column,
        };
        data.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Point, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data = PointData::deserialize(deserializer)?;
        Ok(Point {
            row: data.row,
            column: data.column,
        })
    }
}
