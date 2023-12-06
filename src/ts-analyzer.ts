import { SmellsCodeAnalyzerConfig } from './config.js';
import Parser, { Tree } from 'tree-sitter';
import { SyntaxNode } from 'tree-sitter';
// const typescript = require('tree-sitter-typescript').typescript;
import parser from 'tree-sitter-typescript';
import { NodeInfo } from './node-info';

export const Grammars = {
  typescript: parser.typescript,
} as const;

export function findElemPositions(
  config: SmellsCodeAnalyzerConfig,
  sourceCode: string
): NodeInfo[] {
  const tree = parse(config, sourceCode);
  const foundNodes = findNodes(tree);
  return foundNodes;
}

function parse(config: SmellsCodeAnalyzerConfig, sourceCode: string): Tree {
  const parser = new Parser();
  const grammar = Grammars[config.grammar];
  if (!grammar) {
    console.error(`Grammar ${config.grammar} not found`);
  }
  parser.setLanguage(grammar);
  const tree = parser.parse(sourceCode);
  return tree;
}

function findNodes(tree: Tree): NodeInfo[] {
  // Find all interfaces from tree
  const foundNodeInfos: NodeInfo[] = [];

  for (const node of traverseTree(tree)) {
    // TODO: master replace hardcode for data in the config
    if (node.type === 'interface_declaration') {
      const foundNode: NodeInfo = {
        name: node.child(1).text,
        type: node.type,
        startPos: node.child(1).startPosition,
        children: [],
      };

      for (const childrenNode of traverseTree(node.tree, node)) {
        if (childrenNode.type === 'property_identifier') {
          foundNode.children.push({
            name: childrenNode.text,
            type: childrenNode.type,
            startPos: childrenNode.startPosition,
          });
        }
      }
      foundNodeInfos.push(foundNode);
    }
  }
  return foundNodeInfos;
}

function* traverseTree(
  tree: Tree,
  parentNode?: SyntaxNode
): IterableIterator<SyntaxNode> {
  const cursor = parentNode?.walk() ?? tree.walk();

  let reachedRoot = false;

  while (!reachedRoot) {
    yield cursor.currentNode;

    if (cursor.gotoFirstChild()) {
      continue;
    }

    if (cursor.gotoNextSibling()) {
      continue;
    }

    let retracing = true;

    while (retracing) {
      if (!cursor.gotoParent() || cursor.currentNode === parentNode) {
        retracing = false;
        reachedRoot = true;
      }

      if (cursor.gotoNextSibling()) {
        retracing = false;
      }
    }
  }
}
