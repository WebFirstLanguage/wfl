import * as assert from 'assert';
import * as cp from 'child_process';
import * as path from 'path';
import * as fs from 'fs';
import { promisify } from 'util';
import { once } from 'events';
import {
  createProtocolConnection, InitializeRequest, InitializedNotification,
  ShutdownRequest, ExitNotification
} from 'vscode-languageclient/node';

describe('WFL LSP Server Integration Tests', () => {
  const extensionRoot = path.resolve(__dirname, '..', '..');
  const lspServerPath = process.env.WFL_LSP_PATH || path.resolve(
    extensionRoot, '..', 'target', 'debug', process.platform === 'win32' ? 'wfl-lsp.exe' : 'wfl-lsp'
  );

  it('Should find the built LSP server executable', () => {
    assert.ok(fs.existsSync(lspServerPath), `LSP executable must exist: ${lspServerPath}`);
  });

  it('Should run the LSP executable and return its version', async () => {
    const { stdout } = await promisify(cp.execFile)(lspServerPath, ['--version'], { timeout: 5000 });
    assert.match(stdout.trim(), /^wfl-lsp \d+\.\d+\.\d+/);
  });

  it('Should initialize and shut down over the real stdio protocol', async () => {
    const server = cp.spawn(lspServerPath, [], { stdio: ['pipe', 'pipe', 'pipe'] });
    let stderr = '';
    server.stderr.on('data', data => { stderr += data.toString(); });
    const exited = new Promise<number | null>((resolve, reject) => {
      server.once('error', reject);
      server.once('close', resolve);
    });
    // Attach an error handler immediately, including spawn failures before shutdown.
    void exited.catch(() => undefined);
    const connection = createProtocolConnection(server.stdout, server.stdin);
    connection.listen();
    let timer: NodeJS.Timeout | undefined;
    const timeout = new Promise<never>((_, reject) => {
      timer = setTimeout(() => reject(new Error(`LSP protocol timed out: ${stderr}`)), 10000);
    });
    try {
      await Promise.race([timeout, (async () => {
        await once(server, 'spawn');
        const result = await connection.sendRequest(InitializeRequest.type, {
          processId: process.pid, rootUri: null, capabilities: {}, workspaceFolders: null
        });
        assert.ok(result.capabilities.hoverProvider, 'Server must advertise hover support');
        assert.ok(result.capabilities.completionProvider, 'Server must advertise completion support');
        await connection.sendNotification(InitializedNotification.type, {});
        assert.strictEqual(await connection.sendRequest(ShutdownRequest.type), null);
        await connection.sendNotification(ExitNotification.type);
        server.stdin.end();
        assert.strictEqual(await exited, 0, `Server should exit cleanly: ${stderr}`);
      })()]);
    } finally {
      clearTimeout(timer);
      connection.dispose();
      if (server.exitCode === null && server.signalCode === null) {
        server.kill('SIGKILL');
      }
      let cleanupTimer: NodeJS.Timeout | undefined;
      try {
        await Promise.race([exited, new Promise<never>((_, reject) => {
          cleanupTimer = setTimeout(() => reject(new Error('LSP process did not exit after cleanup')), 2000);
        })]);
      } finally {
        clearTimeout(cleanupTimer);
      }
    }
  });

  it('Should declare LSP configuration in the extension manifest', () => {
    const manifest = JSON.parse(fs.readFileSync(path.join(extensionRoot, 'package.json'), 'utf8'));
    const config = manifest.contributes.configuration.properties;
    assert.strictEqual(config['wfl.serverPath'].default, 'wfl-lsp');
    assert.deepStrictEqual(config['wfl.serverArgs'].default, []);
  });

  it('Should compile the extension LSP client', () => {
    const extensionCode = fs.readFileSync(path.join(extensionRoot, 'out', 'extension.js'), 'utf8');
    assert.ok(extensionCode.includes('LanguageClient'));
    assert.ok(extensionCode.includes('serverPath'));
  });
});
