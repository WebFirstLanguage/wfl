import * as assert from 'assert';
import * as vscode from 'vscode';

function waitForDiagnostics(
  uri: vscode.Uri,
  matches: (diagnostics: readonly vscode.Diagnostic[]) => boolean
): Promise<readonly vscode.Diagnostic[]> {
  return new Promise((resolve, reject) => {
    const subscription = vscode.languages.onDidChangeDiagnostics(event => {
      if (!event.uris.some(changed => changed.toString() === uri.toString())) {
        return;
      }
      const diagnostics = vscode.languages.getDiagnostics(uri);
      if (matches(diagnostics)) {
        clearTimeout(timer);
        subscription.dispose();
        resolve(diagnostics);
      }
    });
    const timer = setTimeout(() => {
      subscription.dispose();
      reject(new Error(`Timed out waiting for WFL diagnostics for ${uri.fsPath}`));
    }, 10000);
  });
}

describe('WFL LSP Integration Tests', () => {
  let workspace: vscode.Uri;
  let fixtureFolder: vscode.Uri;
  let document: vscode.TextDocument;
  let sequence = 0;

  before(async () => {
    const folder = vscode.workspace.workspaceFolders?.[0];
    assert.ok(folder, 'The test runner must supply an isolated workspace');
    workspace = folder.uri;
    const extension = vscode.extensions.getExtension('wfl.vscode-wfl');
    assert.ok(extension, 'WFL extension should be available');
    await extension.activate();
    // Activation detects installed tools asynchronously before registering commands.
    const deadline = Date.now() + 10000;
    while (!(await vscode.commands.getCommands()).includes('wfl.format')) {
      assert.ok(Date.now() < deadline, 'Extension commands should register within 10 seconds');
      await new Promise(resolve => setTimeout(resolve, 25));
    }
  });

  beforeEach(async () => {
    fixtureFolder = vscode.Uri.joinPath(workspace, `test-${++sequence}`);
    await vscode.workspace.fs.createDirectory(fixtureFolder);
    const uri = vscode.Uri.joinPath(fixtureFolder, 'test.wfl');
    await vscode.workspace.fs.writeFile(uri, Buffer.from('store x as 5\ndisplay x'));
    document = await vscode.workspace.openTextDocument(uri);
  });

  afterEach(async () => {
    for (const dirty of vscode.workspace.textDocuments.filter(candidate =>
      candidate.isDirty && candidate.uri.toString().startsWith(fixtureFolder.toString() + '/')
    )) {
      await vscode.window.showTextDocument(dirty);
      await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
    }
    await vscode.commands.executeCommand('workbench.action.closeAllEditors');
    await vscode.workspace.fs.delete(fixtureFolder, { recursive: true });
  });

  it('Should activate extension for .wfl files', () => {
    assert.strictEqual(document.languageId, 'wfl');
    assert.strictEqual(document.getText().replace(/\r\n/g, '\n'), 'store x as 5\ndisplay x');
    assert.ok(vscode.extensions.getExtension('wfl.vscode-wfl')?.isActive);
  });

  it('Should associate an editor with the WFL language', async () => {
    const editor = await vscode.window.showTextDocument(document);
    assert.strictEqual(editor.document.languageId, 'wfl');
    assert.ok(editor.document.fileName.endsWith('.wfl'));
  });

  it('Should register WFL language configuration', async () => {
    assert.ok((await vscode.languages.getLanguages()).includes('wfl'));
  });

  it('Should provide document formatting edits', async () => {
    const setup = new vscode.WorkspaceEdit();
    setup.replace(document.uri, new vscode.Range(0, 0, document.lineCount, 0), 'store total as 1+2');
    assert.ok(await vscode.workspace.applyEdit(setup));
    assert.ok(await document.save());
    const edits = await vscode.commands.executeCommand<vscode.TextEdit[]>(
      'vscode.executeFormatDocumentProvider', document.uri, { tabSize: 4, insertSpaces: true }
    );
    assert.ok(edits?.length, 'A registered formatter must return edits');
    const edit = new vscode.WorkspaceEdit();
    edit.set(document.uri, edits);
    assert.ok(await vscode.workspace.applyEdit(edit));
    assert.ok(await document.save());
    assert.strictEqual(document.getText(), 'store total as 1 + 2');
  });

  it('Should register all WFL commands', async () => {
    const commands = await vscode.commands.getCommands();
    for (const command of ['wfl.restartLanguageServer', 'wfl.selectLspExecutable', 'wfl.format']) {
      assert.ok(commands.includes(command), `${command} should be registered`);
    }
  });

  it('Should apply and save document edits', async () => {
    const edit = new vscode.WorkspaceEdit();
    edit.insert(document.uri, new vscode.Position(document.lineCount, 0), '\n// test comment');
    assert.ok(await vscode.workspace.applyEdit(edit));
    assert.ok(await document.save());
    assert.ok(document.getText().endsWith('// test comment'));
  });

  it('Should receive real LSP diagnostics and clear them after a repair', async () => {
    const uri = vscode.Uri.joinPath(fixtureFolder, 'error-test.wfl');
    await vscode.workspace.fs.writeFile(uri, Buffer.from('store x as\n'));
    const received = waitForDiagnostics(uri, diagnostics => diagnostics.some(
      diagnostic => diagnostic.severity === vscode.DiagnosticSeverity.Error
    ));
    const errorDocument = await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(errorDocument);
    const diagnostics = await received;
    assert.ok(diagnostics.some(diagnostic => diagnostic.message.length > 0));

    const cleared = waitForDiagnostics(uri, diagnostics => diagnostics.length === 0);
    const edit = new vscode.WorkspaceEdit();
    edit.replace(uri, new vscode.Range(0, 0, errorDocument.lineCount, 0), 'store x as 5\ndisplay x');
    assert.ok(await vscode.workspace.applyEdit(edit));
    assert.ok(await errorDocument.save());
    assert.deepStrictEqual(await cleared, []);
  });

  it('Should expose defaults alongside the configured test server', () => {
    const config = vscode.workspace.getConfiguration('wfl');
    assert.strictEqual(config.inspect<string>('serverPath')?.defaultValue, 'wfl-lsp');
    assert.strictEqual(config.get('serverPath'), process.env.WFL_LSP_PATH);
    assert.deepStrictEqual(config.get('serverArgs'), []);
    assert.strictEqual(config.get('versionMode'), 'warn');
    assert.ok(config.get('format'));
  });

  it('Should create, read, and delete WFL files', async () => {
    const uri = vscode.Uri.joinPath(fixtureFolder, 'temp-test.wfl');
    await vscode.workspace.fs.writeFile(uri, Buffer.from('store temp as "temporary value"'));
    const temporary = await vscode.workspace.openTextDocument(uri);
    assert.strictEqual(temporary.languageId, 'wfl');
    assert.strictEqual(temporary.getText(), 'store temp as "temporary value"');
    await vscode.workspace.fs.delete(uri);
    await assert.rejects(async () => vscode.workspace.fs.stat(uri));
  });
});
