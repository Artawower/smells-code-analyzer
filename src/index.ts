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
    //   '/Users/darkawower/projects/ui/src/app/report/components/report-grid/components/child-counted-row-group-renderer/child-counted-row-group-renderer.component.ts'
    // );
    // console.log(
    //   `✎: [index.ts][${new Date().toString()}] rep`,
    //   buildReport(fileInfo, config.showPassed)
    // );

    for (const filePath of filesForAnalysis) {
        if (config.showProgress) {
            console.log(
                `Analyze [${i + 1}/${filesForAnalysis.length}] ${filePath}`
            );
        }
        const filesInfo = await analyzeFile(config, lsp, filePath);
        analyzedNodes.push(...filesInfo);
        const report = buildReport(filesInfo, config.showPassed);
        if (report?.length) {
            console.log(report);
        }
        i++;
    }

    const deadEntities = findDeadEntities(analyzedNodes);
    console.log(`Found ${deadEntities.length} dead entities`);
    if (config.threshold && deadEntities.length > config.threshold) {
        throw new Error(
            `Found ${deadEntities.length} dead entities, threshold is ${config.threshold}`
        );
    }

    lsp.shutdown();
    lsp.exit();
}

function findDeadEntities(nodes: FullNodeInfo[]): FullNodeInfo[] {
    const deadEntities: FullNodeInfo[] = [];
    for (const node of nodes) {
        if (node.references === 0) {
            deadEntities.push(node);
        }
        deadEntities.push(...findDeadEntities(node.children));
    }
    return deadEntities;
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
    const sourceCodeWithoutComments = sourceCode
        .replace(/\/\/([^\/\n]*)$/gm, '')
        .replace(/\/\*[\s\S]*\*\//gm, '')
        .replace(/console.log\([^)]*\);/gm, '')
        .replace(/[^\x00-\x7F]{1}/g, ' ')
        .replace(
            /[аАбБвВгГдДеЕёЁжЖзЗиИйЙкКлЛмМнНоОпПрРсСтТуУфФхХцЦчЧшШщЩъЪыЫьЬэЭюЮяЯ]{1}/g,
            ''
        );
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
                character: nodeInfo.startPos.column,
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
    program.option(
        '-t, --threshold <value>',
        'Max amount of dead code entities'
    );

    program.parse(process.argv);

    const options = program.opts();
    if (!options.configFile) {
        throw new Error('Config file is not specified');
    }

    const startTime = performance.now();
    console.log(`✎: [index.ts][${new Date().toString()}] start analyze`);
    const config = readConfig(options.configFile, options.threshold);
    await analyzeProject(config);
    const endTime = performance.now();
    console.log(`Analyze took ${(endTime - startTime) / 1000} s`);
})();
