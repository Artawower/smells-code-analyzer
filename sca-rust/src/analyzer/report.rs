use crate::model::FullNodeInfo;

pub fn build_report(nodes: &[FullNodeInfo], show_all: bool) -> String {
    if nodes.is_empty() {
        return String::new();
    }

    let mut sections = Vec::new();
    for node in nodes {
        if let Some(line) = render_node(node, show_all, 0) {
            sections.push(line);
        }
    }

    if sections.is_empty() {
        return String::new();
    }

    let header = nodes
        .first()
        .map(|n| n.file_path.display().to_string())
        .unwrap_or_default();

    let mut report = String::new();
    report.push_str(&header);
    report.push('\n');
    report.push_str(&sections.join("\n"));
    report.push('\n');
    report.push_str(&"-".repeat(80));
    report.push('\n');
    report
}

fn render_node(node: &FullNodeInfo, show_all: bool, depth: usize) -> Option<String> {
    if !show_all && !has_errors(node) {
        return None;
    }

    let reasons = reasons(node);
    let status = if reasons.is_empty() { "✅" } else { "💩" };
    let padding = "\t".repeat(depth);
    let reason_str = reasons.join(", ");

    let mut lines = vec![format!(
        "{padding}[{status}] {}:{}:{} :: ({reason_str})",
        node.name, node.start_position.row, node.start_position.column
    )];

    for child in &node.children {
        if let Some(child_line) = render_node(child, show_all, depth + 1) {
            lines.push(child_line);
        }
    }

    Some(lines.join("\n"))
}

fn reasons(node: &FullNodeInfo) -> Vec<String> {
    let mut reasons = Vec::new();
    if node.references == 0 {
        reasons.push("dead code".to_string());
    }
    if node.parent_name_prefix {
        reasons.push("useless prefix".to_string());
    }
    reasons
}

fn has_errors(node: &FullNodeInfo) -> bool {
    node.references == 0 || node.parent_name_prefix || node.children.iter().any(has_errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FullNodeInfo;
    use tree_sitter::Point;

    #[test]
    fn renders_dead_code() {
        let node = FullNodeInfo {
            node_type: "interface".to_string(),
            name: "Foo".to_string(),
            start_position: Point { row: 10, column: 2 },
            file_path: "test".into(),
            references: 0,
            parent_name_prefix: false,
            children: vec![],
        };

        let report = render_node(&node, false, 0).unwrap();
        assert!(report.contains("💩"));
        assert!(report.contains("dead code"));
    }
}
