import { Point } from 'tree-sitter';

export interface NodeInfo {
  type: string;
  name: string;
  children?: NodeInfo[];
  startPos: Point;
}

export interface FullNodeInfo extends Omit<NodeInfo, 'children'> {
  references: number;
  filePath: string;
  children?: FullNodeInfo[];
  parentNamePrefix?: boolean;
}
