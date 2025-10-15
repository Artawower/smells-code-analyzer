import { SmellsCodeAnalyzerConfig } from './config.js';

import { rgPath } from '@vscode/ripgrep';
import {
  CancellationTokenSource,
  ITextQuery,
  TextSearchEngineAdapter,
} from 'ripgrep-wrapper';
import walkSync from 'walk-sync';

/*
 * Collect files matched by configs via ripgrep
 */
export async function filterFiles(
  config: SmellsCodeAnalyzerConfig
): Promise<string[]> {
  const cts = new CancellationTokenSource();

  const query: ITextQuery = {
    contentPattern: {
      pattern: config.contentMatchingRegexp,
      isRegExp: true,
    },
    folderQueries: [
      {
        folder: config.analyzeDirectory,
        includePattern: { [config.fileMatchingRegexp]: true },
        excludePattern: config.fileExcludeRegexps?.reduce((acc, reg) => {
          return { ...acc, [reg]: true };
        }, {}),
      },
    ],
  };

  const searchEngine = new TextSearchEngineAdapter(rgPath, query);
  const result: string[] = [];
  await searchEngine.search(
    cts.token,
    (res) => {
      result.push(...res.map((r) => r.path));
    },
    () => {}
  );

  return result;
}

export function filesCount(config: SmellsCodeAnalyzerConfig): number {
  const paths = walkSync(config.analyzeDirectory, {
    globs: [config.fileMatchingRegexp],
  });
  return paths.length;
}
