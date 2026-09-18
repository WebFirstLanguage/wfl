import * as vscode from 'vscode';
import * as cp from 'child_process';
import { LanguageClient, TransportKind } from 'vscode-languageclient/node';
import { BaseFormatter } from './formatting/base-formatter';
import { WflFormatter } from './formatting/wfl-formatter';
import { isWflLspVersion } from './lsp-version';

// Configuration type definitions
interface WflCliConfig {
  path: string;
  autoDetect: boolean;
}

// State tracking
let client: LanguageClient | undefined;
let wflAvailable: boolean = false;
let wflPath: string | undefined;
let lspAvailable: boolean = false;

/**
 * Main extension activation
 */
export function activate(context: vscode.ExtensionContext): void {
  const outputChannel = vscode.window.createOutputChannel('WFL');
  outputChannel.appendLine('WFL extension is now active');
  
  // Detect WFL tools availability
  detectWflAvailability().then(({ wflFound, lspFound, wflCliPath }) => {
    wflAvailable = wflFound;
    lspAvailable = lspFound;
    wflPath = wflCliPath;
    
    outputChannel.appendLine(`WFL CLI available: ${wflAvailable}`);
    outputChannel.appendLine(`WFL LSP available: ${lspAvailable}`);
    if (wflAvailable) {
      outputChannel.appendLine(`WFL CLI path: ${wflPath}`);
    }
    
    // Register formatters
    registerFormatters(context);
    
    // Start LSP client if available
    if (lspAvailable) {
      startLspClient(context, outputChannel);
    }
    
    // Register commands
    registerCommands(context, outputChannel);
  });
}

/**
 * Detect WFL CLI and LSP availability
 */
async function detectWflAvailability(): Promise<{ 
  wflFound: boolean; 
  lspFound: boolean; 
  wflCliPath?: string 
}> {
  const config = vscode.workspace.getConfiguration('wfl');
  const cliConfig = config.get<WflCliConfig>('cli', { path: 'wfl', autoDetect: true });
  
  if (!cliConfig.autoDetect) {
    // Use the configured path directly
    return {
      wflFound: await checkWflCli(cliConfig.path),
      lspFound: await checkWflLsp(config.get('serverPath', 'wfl-lsp')),
      wflCliPath: cliConfig.path
    };
  }
  
  // Try to auto-detect WFL CLI
  // First, try the configured path
  if (await checkWflCli(cliConfig.path)) {
    // Also check for LSP
    return {
      wflFound: true,
      lspFound: await checkWflLsp(config.get('serverPath', 'wfl-lsp')),
      wflCliPath: cliConfig.path
    };
  }
  
  // Try to find in PATH
  const isWindows = process.platform === 'win32';
  const wflCmd = isWindows ? 'wfl.exe' : 'wfl';
  
  try {
    // Try to find in PATH using 'which' on Unix or 'where' on Windows
    const whichCmd = isWindows ? 'where' : 'which';
    const { stdout } = await execPromise(whichCmd, [wflCmd]);
    
    if (stdout.trim()) {
      const detectedPath = stdout.trim().split('\n')[0];
      return {
        wflFound: true,
        lspFound: await checkWflLsp(config.get('serverPath', 'wfl-lsp')),
        wflCliPath: detectedPath
      };
    }
  } catch {
    // Command failed, WFL CLI not found in PATH
  }
  
  // Not found
  return {
    wflFound: false,
    lspFound: await checkWflLsp(config.get('serverPath', 'wfl-lsp')),
    wflCliPath: undefined
  };
}

/**
 * Check if WFL CLI is available
 */
async function checkWflCli(path: string): Promise<boolean> {
  try {
    const { stdout } = await execPromise(path, ['--version']);
    // If we got a version output, WFL CLI is available
    return stdout.trim().startsWith('wfl version');
  } catch {
    return false;
  }
}

/**
 * Check if WFL LSP server is available
 */
async function checkWflLsp(path: string): Promise<boolean> {
  try {
    const { stdout } = await execPromise(path, ['--version']);
    // If we got a version output, WFL LSP is available
    return isWflLspVersion(stdout);
  } catch {
    return false;
  }
}

/**
 * Promise-based exec
 */
function execPromise(command: string, args: string[]): Promise<{ stdout: string; stderr: string }> {
  return new Promise((resolve, reject) => {
    let stdout = '';
    let stderr = '';
    
    const process = cp.spawn(command, args);
    
    process.stdout.on('data', (data: Buffer | string) => {
      stdout += data.toString();
    });
    
    process.stderr.on('data', (data: Buffer | string) => {
      stderr += data.toString();
    });
    
    process.on('close', (code: number | null) => {
      if (code === 0) {
        resolve({ stdout, stderr });
      } else {
        reject(new Error(`Command failed with code ${code}: ${stderr}`));
      }
    });
    
    process.on('error', (err: Error) => {
      reject(err);
    });
  });
}

/**
 * Register document formatters
 */
function registerFormatters(context: vscode.ExtensionContext): void {
  const config = vscode.workspace.getConfiguration('wfl.format');
  const formatEnabled = config.get<boolean>('enable', true);
  
  if (!formatEnabled) {
    return;
  }
  
  // Determine which formatter to use
  const provider = config.get<string>('provider', 'auto');
  const useWflFormatter = (provider === 'wfl' || (provider === 'auto' && wflAvailable)) && !!wflPath;
  
  // Register the selected formatter
  if (useWflFormatter && wflPath) {
    // Register WFL CLI-based formatter
    const wflFormatter = new WflFormatter(wflPath);
    
    context.subscriptions.push(
      vscode.languages.registerDocumentFormattingEditProvider('wfl', wflFormatter),
      vscode.languages.registerDocumentRangeFormattingEditProvider('wfl', wflFormatter)
    );
    
    // Register format on save if enabled
    if (config.get<boolean>('formatOnSave', true)) {
      context.subscriptions.push(
        vscode.workspace.onWillSaveTextDocument((event: vscode.TextDocumentWillSaveEvent) => {
          if (event.document.languageId === 'wfl') {
            event.waitUntil(
              vscode.commands.executeCommand('editor.action.formatDocument')
            );
          }
        })
      );
    }
    
    // Register format on type if enabled
    if (config.get<boolean>('formatOnType', false)) {
      context.subscriptions.push(
        vscode.languages.registerOnTypeFormattingEditProvider('wfl', {
          provideOnTypeFormattingEdits: (
            document: vscode.TextDocument, 
            position: vscode.Position, 
            ch: string, 
            options: vscode.FormattingOptions
          ) => {
            // Only format on specific characters like ':'
            if (ch === ':' || ch === '\n') {
              const range = new vscode.Range(
                new vscode.Position(position.line, 0),
                position
              );
              return wflFormatter.provideDocumentRangeFormattingEdits(document, range, options);
            }
            return [];
          }
        }, ':', '\n')
      );
    }
  } else {
    // Register built-in formatter
    const baseFormatter = new BaseFormatter();
    
    context.subscriptions.push(
      vscode.languages.registerDocumentFormattingEditProvider('wfl', baseFormatter),
      vscode.languages.registerDocumentRangeFormattingEditProvider('wfl', baseFormatter)
    );
    
    // Register format on save if enabled
    if (config.get<boolean>('formatOnSave', true)) {
      context.subscriptions.push(
        vscode.workspace.onWillSaveTextDocument((event: vscode.TextDocumentWillSaveEvent) => {
          if (event.document.languageId === 'wfl') {
            event.waitUntil(
              vscode.commands.executeCommand('editor.action.formatDocument')
            );
          }
        })
      );
    }
  }
}

/**
 * Start the LSP client
 */
function startLspClient(context: vscode.ExtensionContext, outputChannel: vscode.OutputChannel): void {
  const config = vscode.workspace.getConfiguration('wfl');
  const serverPath = config.get<string>('serverPath', 'wfl-lsp');
  const serverArgs = config.get<string[]>('serverArgs', []);
  
  // Configure the server options
  const serverOptions = {
    run: { 
      command: serverPath, 
      args: serverArgs,
      transport: TransportKind.stdio
    },
    debug: {
      command: serverPath,
      args: [...serverArgs, '--debug'],
      transport: TransportKind.stdio
    }
  };
  
  // Configure the client options
  const clientOptions = {
    documentSelector: [{ scheme: 'file', language: 'wfl' }],
    outputChannel: outputChannel,
    synchronize: {
      configurationSection: 'wfl'
    }
  };
  
  // Create and start the client
  client = new LanguageClient(
    'wfl',
    'WFL Language Server',
    serverOptions,
    clientOptions
  );
  
  // Fix: client.start() returns a Promise, but we need a Disposable
  const clientDisposable = client.start();
  context.subscriptions.push({ dispose: () => { clientDisposable.then(() => {}); } });
  
  outputChannel.appendLine('WFL Language Server started');
}

/**
 * Register extension commands
 */
function registerCommands(context: vscode.ExtensionContext, outputChannel: vscode.OutputChannel): void {
  // Restart language server command
  context.subscriptions.push(
    vscode.commands.registerCommand('wfl.restartLanguageServer', async () => {
      if (client) {
        await client.stop();
        startLspClient(context, outputChannel);
        vscode.window.showInformationMessage('WFL Language Server restarted');
      } else if (lspAvailable) {
        startLspClient(context, outputChannel);
        vscode.window.showInformationMessage('WFL Language Server started');
      } else {
        vscode.window.showErrorMessage('WFL Language Server is not available');
      }
    })
  );
  
  // Select LSP executable command
  context.subscriptions.push(
    vscode.commands.registerCommand('wfl.selectLspExecutable', async () => {
      const options: vscode.OpenDialogOptions = {
        canSelectMany: false,
        title: 'Select WFL LSP Executable',
        filters: {
          'Executable Files': ['exe', 'sh', ''],
          'All Files': ['*']
        }
      };
      
      const fileUri = await vscode.window.showOpenDialog(options);
      if (fileUri && fileUri.length > 0) {
        const filePath = fileUri[0].fsPath;
        
        // Update the configuration
        await vscode.workspace.getConfiguration('wfl')
          .update('serverPath', filePath, vscode.ConfigurationTarget.Global);
        
        // Check if it's a valid LSP server
        const isValidLsp = await checkWflLsp(filePath);
        if (isValidLsp) {
          vscode.window.showInformationMessage(
            'WFL LSP executable updated. Restart the language server to apply changes.'
          );
        } else {
          vscode.window.showWarningMessage(
            'The selected file does not appear to be a valid WFL LSP executable.'
          );
        }
      }
    })
  );
  
  // Format document command
  context.subscriptions.push(
    vscode.commands.registerCommand('wfl.format', async () => {
      const editor = vscode.window.activeTextEditor;
      if (editor && editor.document.languageId === 'wfl') {
        await vscode.commands.executeCommand('editor.action.formatDocument');
      } else {
        vscode.window.showErrorMessage('No active WFL document to format');
      }
    })
  );
}

/**
 * Extension deactivation
 */
export function deactivate(): Promise<void> | undefined {
  if (client) {
    return client.stop();
  }
  return undefined;
}
