use crate::config::Grammar;
use crate::model::{NodeInfo, NodeTarget};
use anyhow::{Context, Result};
use tree_sitter::{Language, Node, Parser};

pub struct TreeAnalyzer {
    parser: Parser,
    reference_nodes: Vec<NodeTarget>,
}

impl TreeAnalyzer {
    pub fn new(grammar: Grammar, reference_nodes: Vec<NodeTarget>) -> Result<Self> {
        let mut parser = Parser::new();
        parser
            .set_language(language_from_grammar(grammar))
            .context("Failed to configure tree-sitter grammar")?;

        Ok(Self {
            parser,
            reference_nodes,
        })
    }

    pub fn find_positions(&mut self, source: &str) -> Result<Vec<NodeInfo>> {
        let tree = self
            .parser
            .parse(source, None)
            .context("tree-sitter failed to parse source")?;

        let mut found = Vec::new();
        traverse_result(tree.root_node(), |node| {
            let mut matches = self.handle_node(source.as_bytes(), node)?;
            found.append(&mut matches);
            Ok(())
        })?;

        Ok(found)
    }

    fn handle_node(&self, source: &[u8], node: Node<'_>) -> Result<Vec<NodeInfo>> {
        let mut matched = Vec::new();
        for target in &self.reference_nodes {
            if node.kind() == target.node_type {
                if let Some(found) = self.build_node(source, node, target)? {
                    matched.push(found);
                }
            }
        }
        Ok(matched)
    }

    fn build_node(
        &self,
        source: &[u8],
        node: Node<'_>,
        target: &NodeTarget,
    ) -> Result<Option<NodeInfo>> {
        let target_node = if let Some(ref_type) = &target.ref_type {
            match find_descendant(node, ref_type.as_str()) {
                Some(found) => found,
                None => return Ok(None),
            }
        } else {
            node
        };

        let name = target_node
            .utf8_text(source)
            .unwrap_or_default()
            .trim()
            .to_string();

        let children = if target.children.is_empty() {
            Vec::new()
        } else {
            self.collect_children(source, node, &target.children)?
        };

        Ok(Some(NodeInfo {
            node_type: node.kind().to_string(),
            name,
            start_position: target_node.start_position(),
            children,
        }))
    }
}

fn language_from_grammar(grammar: Grammar) -> Language {
    match grammar {
        Grammar::TypeScript => tree_sitter_typescript::language_typescript(),
    }
}

fn find_descendant<'a>(node: Node<'a>, target_kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    let mut reached_root = false;

    while !reached_root {
        let candidate = cursor.node();
        if candidate.kind() == target_kind {
            return Some(candidate);
        }

        if cursor.goto_first_child() {
            continue;
        }
        if cursor.goto_next_sibling() {
            continue;
        }

        loop {
            if !cursor.goto_parent() {
                reached_root = true;
                break;
            }
            if cursor.goto_next_sibling() {
                break;
            }
        }
    }

    None
}

impl TreeAnalyzer {
    fn collect_children(
        &self,
        source: &[u8],
        parent: Node<'_>,
        targets: &[NodeTarget],
    ) -> Result<Vec<NodeInfo>> {
        let mut children = Vec::new();
        let mut is_root = true;
        traverse_result(parent, |candidate| {
            if is_root {
                is_root = false;
                return Ok(());
            }
            for target in targets {
                if candidate.kind() == target.node_type {
                    if let Some(child) = self.build_node(source, candidate, target)? {
                        children.push(child);
                    }
                }
            }
            Ok(())
        })?;
        Ok(children)
    }
}

fn traverse_result(node: Node<'_>, mut visit: impl FnMut(Node<'_>) -> Result<()>) -> Result<()> {
    let mut cursor = node.walk();
    let mut reached_root = false;

    while !reached_root {
        visit(cursor.node())?;

        if cursor.goto_first_child() {
            continue;
        }
        if cursor.goto_next_sibling() {
            continue;
        }

        loop {
            if !cursor.goto_parent() {
                reached_root = true;
                break;
            }
            if cursor.goto_next_sibling() {
                break;
            }
        }
    }
    Ok(())
}
