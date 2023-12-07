import { FullNodeInfo } from './node-info.js';

export function buildReport(
  fullNodesInfo: FullNodeInfo[],
  showAll = false,
  includeFileName = true,
  tabs = 0,
  blockSeparator = '\n' + '-'.repeat(80)
): string {
  if (!fullNodesInfo?.length) {
    return '';
  }
  const reports = fullNodesInfo
    .filter((n) => showAll || hasErrors(n))
    .map((n) => {
      const reasons = [];

      if (n.references === 0) {
        reasons.push('dead code');
      }

      if (n.parentNamePrefix) {
        reasons.push(`useless prefix`);
      }

      const baseInfo = `${'\t'.repeat(tabs)}[${
        reasons.length === 0 ? '✅' : '💩'
      }] ${n.name}:${n.startPos.row}:${n.startPos.column} :: (${reasons.join(
        ', '
      )})`;

      const childrenInfo = buildReport(
        n.children,
        showAll,
        false,
        tabs + 1,
        ''
      );
      if (!childrenInfo?.length) {
        return baseInfo;
      }
      return baseInfo + '\n' + childrenInfo;
    });

  if (reports.length) {
    const prefix = includeFileName ? fullNodesInfo[0].filePath + '\n' : '';
    return prefix + reports.join('\n') + blockSeparator + '\n';
  }

  return '';
}

function hasErrors(n: FullNodeInfo): boolean {
  return (
    n.references === 0 ||
    n.parentNamePrefix ||
    n.children?.some((nc) => hasErrors(nc))
  );
}
