import * as assert from 'assert';
import { isWflLspVersion } from '../lsp-version';

describe('WFL LSP version detection', () => {
  it('accepts the current binary version output', () => {
    assert.strictEqual(isWflLspVersion('wfl-lsp 0.1.0\n'), true);
  });

  it('preserves support for the legacy version prefix', () => {
    assert.strictEqual(isWflLspVersion('wfl-lsp version 0.1.0\r\n'), true);
  });

  it('accepts valid prerelease and build metadata', () => {
    for (const prefix of ['wfl-lsp ', 'wfl-lsp version ']) {
      assert.strictEqual(isWflLspVersion(`${prefix}1.2.3-beta.1+build.5`), true);
    }
  });

  it('rejects another executable or embedded version banner', () => {
    for (const output of ['v20.0.0', 'wfl 0.1.0', 'other wfl-lsp 0.1.0', '']) {
      assert.strictEqual(isWflLspVersion(output), false, output);
    }
  });

  it('rejects malformed and incomplete LSP version output', () => {
    for (const output of [
      'wfl-lsp version', 'wfl-lsp version garbage', 'wfl-lsp 1.2',
      'wfl-lsp version 01.2.3', 'wfl-lsp 1.2.3 extra', 'wfl-lsp 1.2.3\nother output'
    ]) {
      assert.strictEqual(isWflLspVersion(output), false, output);
    }
  });
});
