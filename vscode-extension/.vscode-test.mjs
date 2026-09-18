import { defineConfig } from '@vscode/test-cli';
import { downloadAndUnzipVSCode } from '@vscode/test-electron';
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const extensionRoot = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.dirname(extensionRoot);
const artifactsRoot = path.join(repositoryRoot, 'target', 'test-artifacts', 'vscode-extension');
const lspPath = path.resolve(process.env.WFL_LSP_PATH || path.join(
  repositoryRoot, 'target', 'debug', process.platform === 'win32' ? 'wfl-lsp.exe' : 'wfl-lsp'
));
if (!existsSync(lspPath)) {
  throw new Error(`Build the LSP server with cargo build --locked -p wfl-lsp before npm test: ${lspPath}`);
}
mkdirSync(artifactsRoot, { recursive: true });
const runRoot = mkdtempSync(path.join(artifactsRoot, 'run-'));
const workspaceFolder = path.join(runRoot, 'workspace');
mkdirSync(path.join(workspaceFolder, '.vscode'), { recursive: true });
writeFileSync(path.join(workspaceFolder, '.vscode', 'settings.json'), JSON.stringify({
  'workbench.localHistory.enabled': false,
  'wfl.serverPath': lspPath,
  'wfl.cli': { path: path.join(runRoot, 'unavailable-wfl'), autoDetect: false },
  'wfl.format': { enable: true, provider: 'builtin', formatOnSave: false }
}));
process.on('exit', () => rmSync(runRoot, { recursive: true, force: true, maxRetries: 3 }));

const vscodeExecutablePath = await downloadAndUnzipVSCode({
  version: process.env.VSCODE_TEST_VERSION || 'stable',
  cachePath: path.join(artifactsRoot, 'vscode')
});

export default defineConfig({
  files: 'out/test/**/*.test.js',
  workspaceFolder,
  useInstallation: { fromPath: vscodeExecutablePath },
  env: { WFL_LSP_PATH: lspPath },
  launchArgs: [
    '--disable-extensions', '--disable-workspace-trust', '--skip-welcome',
    `--user-data-dir=${path.join(runRoot, 'user-data')}`,
    `--extensions-dir=${path.join(runRoot, 'extensions')}`
  ],
  mocha: {
    ui: 'bdd',
    timeout: 30000,
    color: true,
    forbidPending: true,
    forbidOnly: true
  },
  extensionDevelopmentPath: extensionRoot
});
