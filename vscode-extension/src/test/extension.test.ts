import * as assert from 'assert';
import * as vscode from 'vscode';

// Basic test suite for the extension
describe('WFL Extension Tests', () => {
  
  it('Extension should be activated', async () => {
    // Verify the extension is activated
    const extension = vscode.extensions.getExtension('wfl.vscode-wfl');
    assert.notStrictEqual(extension, undefined);
    
    if (extension) {
      // Wait for extension to activate if not already
      if (!extension.isActive) {
        await extension.activate();
      }
      assert.strictEqual(extension.isActive, true);
    }
  });

  it('Should recognize an untitled WFL document', async () => {
    // Create a simple WFL document
    const content = '// This is a WFL test file\nstore test as "value"';
    const doc = await vscode.workspace.openTextDocument({ language: 'wfl', content });
    
    // Check the document is identified as WFL
    assert.strictEqual(doc.languageId, 'wfl');
    assert.strictEqual(doc.getText(), content);
  });

});
