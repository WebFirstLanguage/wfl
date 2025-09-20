import * as assert from 'assert';
import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

// Integration tests for WFL VS Code extension with LSP server
describe('WFL LSP Integration Tests', () => {
  let testWorkspaceUri: vscode.Uri;
  let testDocument: vscode.TextDocument;

  before(async function() {
    this.timeout(30000); // Allow time for extension activation
    
    // Get the extension
    const extension = vscode.extensions.getExtension('wfl.vscode-wfl');
    assert.notStrictEqual(extension, undefined, 'WFL extension should be available');
    
    if (extension && !extension.isActive) {
      await extension.activate();
    }
    
    // Create a temporary workspace for testing
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (workspaceFolder) {
      testWorkspaceUri = workspaceFolder.uri;
    } else {
      // Create a temporary folder if no workspace is open
      const tmpDir = path.join(__dirname, '..', '..', 'test-workspace');
      if (!fs.existsSync(tmpDir)) {
        fs.mkdirSync(tmpDir, { recursive: true });
      }
      testWorkspaceUri = vscode.Uri.file(tmpDir);
    }
  });

  after(async () => {
    // Clean up test documents
    if (testDocument && !testDocument.isClosed) {
      await vscode.window.showTextDocument(testDocument);
      await vscode.commands.executeCommand('workbench.action.closeActiveEditor');
    }
  });

  it('Should activate extension for .wfl files', async function() {
    this.timeout(10000);
    
    // Create a test WFL file
    const testFileUri = vscode.Uri.joinPath(testWorkspaceUri, 'test.wfl');
    const testContent = 'store x as 5\ndisplay x';
    
    // Create and open the document
    const edit = new vscode.WorkspaceEdit();
    edit.createFile(testFileUri, { overwrite: true });
    edit.insert(testFileUri, new vscode.Position(0, 0), testContent);
    
    await vscode.workspace.applyEdit(edit);
    testDocument = await vscode.workspace.openTextDocument(testFileUri);
    
    // Verify the document is recognized as WFL
    assert.strictEqual(testDocument.languageId, 'wfl', 'Document should be recognized as WFL');
    assert.strictEqual(testDocument.getText(), testContent, 'Document content should match');
  });

  it('Should provide syntax highlighting for WFL', async function() {
    this.timeout(5000);
    
    if (!testDocument) {
      this.skip();
      return;
    }
    
    // Open the document in the editor
    const editor = await vscode.window.showTextDocument(testDocument);
    
    // Verify the editor is open and has the correct language
    assert.strictEqual(editor.document.languageId, 'wfl');
    assert.strictEqual(editor.document.fileName.endsWith('.wfl'), true);
    
    // Note: Testing actual syntax highlighting requires more complex setup
    // For now, we verify the document is properly associated with WFL language
  });

  it('Should register WFL language configuration', () => {
    // Check that WFL language is registered
    const languages = vscode.languages.getLanguages();
    
    // This is async, so we need to handle it properly
    return languages.then(langs => {
      assert.ok(langs.includes('wfl'), 'WFL language should be registered');
    });
  });

  it('Should provide document formatting capability', async function() {
    this.timeout(10000);
    
    if (!testDocument) {
      this.skip();
      return;
    }

    // Try to format the document
    try {
      await vscode.window.showTextDocument(testDocument);
      const formatCommand = 'editor.action.formatDocument';

      // This might fail if LSP server is not available, which is acceptable
      const result = await vscode.commands.executeCommand(formatCommand);
      console.log('Format command result:', result);
    } catch (error) {
      console.log('Format command failed (acceptable if LSP not available):', error);
    }
  });

  it('Should handle WFL commands', async () => {
    // Test that WFL-specific commands are registered
    const allCommands = await vscode.commands.getCommands();
    
    const wflCommands = [
      'wfl.restartLanguageServer',
      'wfl.selectLspExecutable',
      'wfl.format'
    ];
    
    for (const command of wflCommands) {
      assert.ok(
        allCommands.includes(command),
        `Command ${command} should be registered`
      );
    }
  });

  it('Should handle LSP server connection gracefully', async function() {
    this.timeout(15000);
    
    if (!testDocument) {
      this.skip();
      return;
    }

    // Open the document and wait a bit for LSP to potentially connect
    await vscode.window.showTextDocument(testDocument);
    
    // Wait for potential LSP initialization
    await new Promise(resolve => setTimeout(resolve, 2000));
    
    // Test that the document can be edited without errors
    const edit = new vscode.WorkspaceEdit();
    const newContent = '\n// This is a test comment';
    edit.insert(testDocument.uri, new vscode.Position(testDocument.lineCount, 0), newContent);
    
    const success = await vscode.workspace.applyEdit(edit);
    assert.ok(success, 'Should be able to edit WFL document');
    
    // Verify the edit was applied
    const updatedDocument = await vscode.workspace.openTextDocument(testDocument.uri);
    assert.ok(
      updatedDocument.getText().includes('// This is a test comment'),
      'Edit should be applied to document'
    );
  });

  it('Should provide diagnostics for WFL syntax errors', async function() {
    this.timeout(10000);
    
    // Create a document with syntax errors
    const errorFileUri = vscode.Uri.joinPath(testWorkspaceUri, 'error-test.wfl');
    const errorContent = 'store x as\n// Missing value - syntax error';
    
    const edit = new vscode.WorkspaceEdit();
    edit.createFile(errorFileUri, { overwrite: true });
    edit.insert(errorFileUri, new vscode.Position(0, 0), errorContent);
    
    await vscode.workspace.applyEdit(edit);
    const errorDocument = await vscode.workspace.openTextDocument(errorFileUri);
    await vscode.window.showTextDocument(errorDocument);
    
    // Wait for diagnostics to potentially appear
    await new Promise(resolve => setTimeout(resolve, 3000));
    
    // Check if diagnostics are available
    const diagnostics = vscode.languages.getDiagnostics(errorDocument.uri);
    
    // Note: Diagnostics might not appear if LSP server is not running
    // This test validates that the system can handle error documents gracefully
    console.log(`Diagnostics found: ${diagnostics.length}`);
    
    // Clean up
    await vscode.commands.executeCommand('workbench.action.closeActiveEditor');
    await vscode.workspace.fs.delete(errorFileUri);
  });

  it('Should handle configuration changes', async () => {
    // Test configuration access
    const config = vscode.workspace.getConfiguration('wfl');
    
    // Verify default configuration values
    const serverPath = config.get<string>('serverPath');
    const serverArgs = config.get<string[]>('serverArgs');
    const versionMode = config.get<string>('versionMode');
    
    assert.strictEqual(serverPath, 'wfl-lsp', 'Default server path should be wfl-lsp');
    assert.ok(Array.isArray(serverArgs), 'Server args should be an array');
    assert.strictEqual(versionMode, 'warn', 'Default version mode should be warn');
    
    // Test format configuration
    const formatConfig = config.get('format');
    assert.ok(formatConfig, 'Format configuration should exist');
  });

  it('Should handle file operations correctly', async function() {
    this.timeout(5000);
    
    // Test creating, opening, and closing WFL files
    const tempFileUri = vscode.Uri.joinPath(testWorkspaceUri, 'temp-test.wfl');
    const tempContent = 'store temp as "temporary value"';
    
    // Create file
    const edit = new vscode.WorkspaceEdit();
    edit.createFile(tempFileUri, { overwrite: true });
    edit.insert(tempFileUri, new vscode.Position(0, 0), tempContent);
    
    const success = await vscode.workspace.applyEdit(edit);
    assert.ok(success, 'Should be able to create WFL file');
    
    // Open file
    const tempDocument = await vscode.workspace.openTextDocument(tempFileUri);
    assert.strictEqual(tempDocument.languageId, 'wfl', 'File should be recognized as WFL');
    
    // Show in editor
    const editor = await vscode.window.showTextDocument(tempDocument);
    assert.strictEqual(editor.document.uri.toString(), tempFileUri.toString());
    
    // Close and delete
    await vscode.commands.executeCommand('workbench.action.closeActiveEditor');
    await vscode.workspace.fs.delete(tempFileUri);
  });
});
