#!/usr/bin/env node

import { LspClient } from 'ts-lsp-client';
import { findElemPositions } from './ts-analyzer.js';
import { SmellsCodeAnalyzerConfig, readConfig } from './config.js';
import { filesCount, filterFiles } from './filter-files.js';
import { readFileSync } from 'fs';
import { initLSP } from './lsp.js';
import { pathToFileURL } from 'url';
import { FullNodeInfo, NodeInfo } from './node-info.js';
import { buildReport } from './report.js';
import { Command } from 'commander';

async function analyzeProject(config: SmellsCodeAnalyzerConfig): Promise<void> {
  const fc = filesCount(config);
  console.log(`FILES TO ANALYZE: ${fc}`);
  const filesForAnalysis = await filterFiles(config);
  const lsp = await initLSP(config);

  const analyzedNodes: FullNodeInfo[] = [];

  let i = 0;

  // const fileInfo = await analyzeFile(
  //   config,
  //   lsp,
  //   '/Users/darkawower/projects/ui/src/app/shared/directives/disable-control/disable-control.directive.spec.ts'
  // );
  // console.log(
  //   `✎: [index.ts][${new Date().toString()}] rep`,
  //   JSON.stringify(fileInfo, null, 2),
  //   buildReport(fileInfo)
  // );

  for (const filePath of filesForAnalysis) {
    if (config.showProgress) {
      console.log(`Analyze [${i + 1}/${filesForAnalysis.length}] ${filePath}`);
    }
    const filesInfo = await analyzeFile(config, lsp, filePath);
    analyzedNodes.push(...filesInfo);
    const report = buildReport(filesInfo, true);
    if (report?.length) {
      console.log(report);
    }
    i++;
  }

  lsp.shutdown();
  lsp.exit();
}

async function analyzeFile(
  config: SmellsCodeAnalyzerConfig,
  lsp: LspClient,
  filePath: string
): Promise<FullNodeInfo[]> {
  const sourceCode = readFileSync(filePath, {
    encoding: config.encoding,
  }).toString();
  // console.log('✎: [line 46][index.ts] sourceCode: ', sourceCode)
  // const sourceCodeWithoutComments = sourceCode.replace(/\/\/([^\/\n]*)$/gm, '').replace(/\/\*[\s\S]*\*\//gm, '').replace(/console.log\([^)]*\);/gm, '')
  // console.log('✎: [line 48][index.ts] sourceCodeWithoutComments: ', sourceCodeWithoutComments)
  // writeFileSync(filePath, sourceCodeWithoutComments);
  // sourceCode = readFileSync(filePath).toString();
  const sourceCodeWithoutComments = sourceCode;
  const foundNodes = findElemPositions(config, sourceCodeWithoutComments);

  await lsp.didOpen({
    textDocument: {
      uri: pathToFileURL(filePath).href,
      text: sourceCodeWithoutComments,
      version: 1,
      languageId: config.lspName,
    },
  });

  const fullNodesInfo = await analyzeCodeBlock(lsp, filePath, foundNodes);
  return fullNodesInfo;
}

async function analyzeCodeBlock(
  lsp: LspClient,
  filePath: string,
  nodeInfos?: NodeInfo[],
  parent?: NodeInfo
): Promise<FullNodeInfo[]> {
  if (!nodeInfos?.length) {
    return [];
  }
  const fullNodesInfo: FullNodeInfo[] = [];
  for (const nodeInfo of nodeInfos) {
    const references = await lsp.references({
      context: { includeDeclaration: true },
      textDocument: { uri: pathToFileURL(filePath).href },
      position: {
        line: nodeInfo.startPos.row,
        character: nodeInfo.startPos.column + 1,
      },
    });

    const locations: any = references;
    const referencesCount = locations?.length;

    const prefixWithParentName =
      parent &&
      nodeInfo.name.toLowerCase().startsWith(parent.name.toLowerCase());

    fullNodesInfo.push({
      ...nodeInfo,
      filePath,
      children: await analyzeCodeBlock(
        lsp,
        filePath,
        nodeInfo.children,
        nodeInfo
      ),
      references: referencesCount ? referencesCount - 1 : 0,
      parentNamePrefix: prefixWithParentName,
    });
  }

  return fullNodesInfo;
}

(async (): Promise<void> => {
  const program = new Command();
  program.option('-c, --config-file <path>', 'Path for config file');

  program.parse(process.argv);

  const options = program.opts();
  if (!options.configFile) {
    throw new Error('Config file is not specified');
  }

  const startTime = performance.now();
  console.log(`✎: [index.ts][${new Date().toString()}] start analyze`);
  const config = readConfig(options.configFile);
  await analyzeProject(config);
  const endTime = performance.now();
  console.log(`Analyze took ${(endTime - startTime) / 1000} s`);
})();
