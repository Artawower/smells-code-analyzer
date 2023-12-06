import { readFileSync } from 'fs';
import { Grammars } from './ts-analyzer.js';

export interface SmellsCodeAnalyzerConfig {
  projectRootPath: string;
  showProgress?: boolean;
  analyzeDirectory: string;
  lspExecutable: string;
  lspArgs: string[];
  fileMatchingRegexp?: string;
  fileExcludeRegexps?: string[];
  contentMatchingRegexp?: string;
  lspCapabilities?: any;
  lspVersion: string;
  lspName: string;
  grammar: keyof typeof Grammars;
  encoding: BufferEncoding;
}

export function readConfig(configPath?: string): SmellsCodeAnalyzerConfig {
  const config = JSON.parse(
    readFileSync(configPath).toString()
  ) as SmellsCodeAnalyzerConfig;

  return {
    // projectRootPath: '/Users/darkawower/projects/ui',
    // analyzeDirectory: '/Users/darkawower/projects/ui/src',
    // lspExecutable: 'ngserver',
    // lspExecutable: 'node',
    // lspExecutable: 'typescript-language-server',
    // lspArgs: [
    //   '/Users/darkawower/projects/ui/node_modules/@angular/language-server',
    //   '--stdio',
    //   '--ngProbeLocations',
    //   '/Users/darkawower/projects/ui/node_modules/@angular/language-server/bin',
    //   '--tsProbeLocations',
    //   '/Users/darkawower/projects/ui/node_modules/typescript/lib',
    // ],
    // grammar: 'typescript',
    // fileMatchingRegexp: '**/*.ts',
    // contentMatchingRegexp: 'interface|export const',
    // contentMatchingRegexp: 'interface',
    encoding: 'ascii',
    lspVersion: '0.0.2',
    // lspName: 'angular-ls',
    // lspName: 'typescript',
    lspCapabilities: {
      textDocument: {
        codeAction: { dynamicRegistration: true },
        codeLens: { dynamicRegistration: true },
        colorProvider: { dynamicRegistration: true },
        completion: {
          completionItem: {
            commitCharactersSupport: true,
            documentationFormat: ['markdown', 'plaintext'],
            snippetSupport: true,
          },
          completionItemKind: {
            valueSet: [
              1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
              20, 21, 22, 23, 24, 25,
            ],
          },
          contextSupport: true,
          dynamicRegistration: true,
        },
        definition: { dynamicRegistration: true },
        documentHighlight: { dynamicRegistration: true },
        documentLink: { dynamicRegistration: true },
        documentSymbol: {
          dynamicRegistration: true,
          symbolKind: {
            valueSet: [
              1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
              20, 21, 22, 23, 24, 25, 26,
            ],
          },
        },
        formatting: { dynamicRegistration: true },
        hover: {
          contentFormat: ['markdown', 'plaintext'],
          dynamicRegistration: true,
        },
        implementation: { dynamicRegistration: true },
        onTypeFormatting: { dynamicRegistration: true },
        publishDiagnostics: { relatedInformation: true },
        rangeFormatting: { dynamicRegistration: true },
        references: { dynamicRegistration: true },
        rename: { dynamicRegistration: true },
        signatureHelp: {
          dynamicRegistration: true,
          signatureInformation: {
            documentationFormat: ['markdown', 'plaintext'],
          },
        },
        synchronization: {
          didSave: true,
          dynamicRegistration: true,
          willSave: true,
          willSaveWaitUntil: true,
        },
        typeDefinition: { dynamicRegistration: true },
      },
      workspace: {
        applyEdit: true,
        configuration: true,
        didChangeConfiguration: { dynamicRegistration: true },
        didChangeWatchedFiles: { dynamicRegistration: true },
        executeCommand: { dynamicRegistration: true },
        symbol: {
          dynamicRegistration: true,
          symbolKind: {
            valueSet: [
              1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
              20, 21, 22, 23, 24, 25, 26,
            ],
          },
        },
        workspaceEdit: { documentChanges: true },
        workspaceFolders: true,
      },
    },
    ...config,
  };
}
