import { JSONRPCEndpoint, LspClient } from 'ts-lsp-client';
import { SmellsCodeAnalyzerConfig } from './config.js';

import { pathToFileURL } from 'url';
import { spawn } from 'child_process';

export async function initLSP(
  config: SmellsCodeAnalyzerConfig
): Promise<LspClient> {
  const process = spawn(config.lspExecutable, config.lspArgs, {
    shell: true,
    stdio: 'pipe',
  });

  // process.stdout.on('data', function (data) {
  //   //Here is where the output goes
  //   console.log('stdout: ' + data.toString());
  // });

  process.stderr.on('data', function (data) {
    console.log('✎: [line 28][lsp.ts] data: ', data.toString());
  });
  // // process.stderr.on('data', function (data) {
  // //   //Here is where the error output goes

  // //   console.log('stderr: ' + data.toString());
  // // });
  // process.on('close', function (code) {
  //   //Here you can get the exit code of the script

  //   console.log('closing code: ' + code);
  // });

  // create an RPC endpoint for the process
  const endpoint = new JSONRPCEndpoint(process.stdin, process.stdout);

  // create the LSP client
  const client = new LspClient(endpoint);

  await client.initialize({
    processId: process.pid,
    capabilities: config.lspCapabilities,
    clientInfo: {
      name: config.lspName,
      version: config.lspVersion,
    },
    workspaceFolders: [
      {
        name: 'workspace',
        uri: pathToFileURL(config.projectRootPath).href,
      },
    ],
    rootUri: null,
    initializationOptions: {
      tsserver: {
        logDirectory: '.log',
        logVerbosity: 'verbose',
        trace: 'verbose',
      },
    },
  });
  return client;
}
