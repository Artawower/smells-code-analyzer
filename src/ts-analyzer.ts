import { SmellsCodeAnalyzerConfig } from './config.js';
import Parser, { Tree, SyntaxNode } from 'tree-sitter';
import parser from 'tree-sitter-typescript';
import { NodeInfo } from './node-info';
import { NodeTarget } from './config.js';

export const Grammars = {
  typescript: parser.typescript,
} as const;

export function findElemPositions(
  config: SmellsCodeAnalyzerConfig,
  sourceCode: string
): NodeInfo[] {
  const tree = parse(config, sourceCode);
  const foundNodes = findNodes(config.referenceNodes, tree);
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

function findNodes(
  referenceNodes: NodeTarget[],
  tree: Tree,
  parent?: SyntaxNode
): NodeInfo[] {
  const foundNodeInfos: NodeInfo[] = [];

  for (const node of traverseTree(tree, parent)) {
    foundNodeInfos.push(...handleNode(referenceNodes, node));
  }
  return foundNodeInfos;
}

function handleNode(
  referenceNodes: NodeTarget[],
  node: SyntaxNode
): NodeInfo[] {
  const foundNodeInfos: NodeInfo[] = [];

  referenceNodes.forEach((referenceNode) => {
    if (node.type === referenceNode.type) {
      const target = referenceNode.refType
        ? findTargetNode(node, referenceNode.refType)
        : node;
      if (!target) {
        return;
      }
      const foundNode: NodeInfo = {
        name: target.text,
        type: node.type,
        startPos: target.startPosition,
        children: [],
      };

      if (referenceNode.children) {
        foundNode.children.push(
          ...findNodes(referenceNode.children, node.tree, node)
        );
      }

      foundNodeInfos.push(foundNode);
    }
  });

  return foundNodeInfos;
}

function findTargetNode(
  parentNode: SyntaxNode,
  targetRef: string
): SyntaxNode | undefined {
  for (const node of traverseTree(parentNode.tree, parentNode)) {
    if (node.type === targetRef) {
      return node;
    }
  }
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
