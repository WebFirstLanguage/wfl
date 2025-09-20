import * as assert from 'assert';
import * as fs from 'fs';
import * as path from 'path';

// Tests for VS Code extension structure and configuration
describe('WFL Extension Structure Tests', () => {
  const extensionRoot = path.resolve(__dirname, '..', '..');

  it('Should have valid package.json', () => {
    const packageJsonPath = path.join(extensionRoot, 'package.json');
    assert.ok(fs.existsSync(packageJsonPath), 'package.json should exist');
    
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
    
    // Verify essential fields
    assert.strictEqual(packageJson.name, 'vscode-wfl', 'Extension name should be vscode-wfl');
    assert.strictEqual(packageJson.displayName, 'WebFirst Language', 'Display name should be correct');
    assert.ok(packageJson.version, 'Version should be specified');
    assert.ok(packageJson.engines.vscode, 'VS Code engine version should be specified');
    
    // Verify activation events
    assert.ok(packageJson.activationEvents, 'Activation events should be defined');
    assert.ok(
      packageJson.activationEvents.includes('onLanguage:wfl'),
      'Should activate on WFL language'
    );
    
    // Verify main entry point
    assert.strictEqual(packageJson.main, './out/extension.js', 'Main entry point should be correct');
  });

  it('Should have language configuration', () => {
    const langConfigPath = path.join(extensionRoot, 'language-configuration.json');
    assert.ok(fs.existsSync(langConfigPath), 'language-configuration.json should exist');
    
    const langConfig = JSON.parse(fs.readFileSync(langConfigPath, 'utf8'));
    
    // Verify basic language configuration
    assert.ok(langConfig.comments, 'Comment configuration should exist');
    assert.ok(langConfig.brackets, 'Bracket configuration should exist');
    assert.ok(langConfig.autoClosingPairs, 'Auto-closing pairs should be configured');
  });

  it('Should have syntax highlighting grammar', () => {
    const grammarPath = path.join(extensionRoot, 'syntaxes', 'wfl.tmLanguage.json');
    assert.ok(fs.existsSync(grammarPath), 'WFL TextMate grammar should exist');
    
    const grammar = JSON.parse(fs.readFileSync(grammarPath, 'utf8'));
    
    // Verify grammar structure
    assert.strictEqual(grammar.scopeName, 'source.wfl', 'Scope name should be source.wfl');
    assert.ok(grammar.patterns, 'Grammar patterns should be defined');
    assert.ok(grammar.repository, 'Grammar repository should be defined');
  });

  it('Should have compiled extension code', () => {
    const extensionJsPath = path.join(extensionRoot, 'out', 'extension.js');
    assert.ok(fs.existsSync(extensionJsPath), 'Compiled extension.js should exist');
    
    // Verify the compiled code has basic structure
    const extensionCode = fs.readFileSync(extensionJsPath, 'utf8');
    assert.ok(extensionCode.includes('activate'), 'Extension should have activate function');
    assert.ok(extensionCode.includes('deactivate'), 'Extension should have deactivate function');
  });

  it('Should have proper configuration schema', () => {
    const packageJsonPath = path.join(extensionRoot, 'package.json');
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
    
    // Verify configuration contributions
    assert.ok(packageJson.contributes, 'Should have contributions');
    assert.ok(packageJson.contributes.configuration, 'Should have configuration');
    
    const config = packageJson.contributes.configuration;
    assert.ok(config.properties, 'Configuration should have properties');
    
    // Verify essential configuration properties
    const props = config.properties;
    assert.ok(props['wfl.serverPath'], 'Should have serverPath configuration');
    assert.ok(props['wfl.serverArgs'], 'Should have serverArgs configuration');
    assert.ok(props['wfl.versionMode'], 'Should have versionMode configuration');
    assert.ok(props['wfl.format'], 'Should have format configuration');
  });

  it('Should have proper command contributions', () => {
    const packageJsonPath = path.join(extensionRoot, 'package.json');
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
    
    // Verify command contributions
    assert.ok(packageJson.contributes.commands, 'Should have command contributions');
    
    const commands = packageJson.contributes.commands;
    const commandIds = commands.map((cmd: any) => cmd.command);
    
    // Verify essential commands
    assert.ok(
      commandIds.includes('wfl.restartLanguageServer'),
      'Should have restart language server command'
    );
    assert.ok(
      commandIds.includes('wfl.selectLspExecutable'),
      'Should have select LSP executable command'
    );
    assert.ok(
      commandIds.includes('wfl.format'),
      'Should have format command'
    );
  });

  it('Should have proper language contributions', () => {
    const packageJsonPath = path.join(extensionRoot, 'package.json');
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
    
    // Verify language contributions
    assert.ok(packageJson.contributes.languages, 'Should have language contributions');
    
    const languages = packageJson.contributes.languages;
    const wflLang = languages.find((lang: any) => lang.id === 'wfl');
    
    assert.ok(wflLang, 'Should have WFL language definition');
    assert.ok(wflLang.extensions.includes('.wfl'), 'Should associate with .wfl files');
    assert.ok(wflLang.aliases.includes('WFL'), 'Should have WFL alias');
  });

  it('Should have proper grammar contributions', () => {
    const packageJsonPath = path.join(extensionRoot, 'package.json');
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
    
    // Verify grammar contributions
    assert.ok(packageJson.contributes.grammars, 'Should have grammar contributions');
    
    const grammars = packageJson.contributes.grammars;
    const wflGrammar = grammars.find((gram: any) => gram.language === 'wfl');
    
    assert.ok(wflGrammar, 'Should have WFL grammar definition');
    assert.strictEqual(wflGrammar.scopeName, 'source.wfl', 'Should have correct scope name');
    assert.ok(wflGrammar.path.includes('wfl.tmLanguage.json'), 'Should point to grammar file');
  });

  it('Should have test files compiled', () => {
    const testOutDir = path.join(extensionRoot, 'out', 'test');
    assert.ok(fs.existsSync(testOutDir), 'Test output directory should exist');
    
    // Check for compiled test files
    const extensionTestPath = path.join(testOutDir, 'extension.test.js');
    const lspTestPath = path.join(testOutDir, 'lsp-integration.test.js');
    const structureTestPath = path.join(testOutDir, 'extension-structure.test.js');
    
    assert.ok(fs.existsSync(extensionTestPath), 'Extension test should be compiled');
    assert.ok(fs.existsSync(lspTestPath), 'LSP integration test should be compiled');
    assert.ok(fs.existsSync(structureTestPath), 'Structure test should be compiled');
  });

  it('Should have proper dependencies', () => {
    const packageJsonPath = path.join(extensionRoot, 'package.json');
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
    
    // Verify essential dependencies
    assert.ok(packageJson.dependencies, 'Should have dependencies');
    assert.ok(
      packageJson.dependencies['vscode-languageclient'],
      'Should have vscode-languageclient dependency'
    );
    
    // Verify dev dependencies
    assert.ok(packageJson.devDependencies, 'Should have dev dependencies');
    assert.ok(
      packageJson.devDependencies['@types/vscode'],
      'Should have VS Code types'
    );
    assert.ok(
      packageJson.devDependencies['typescript'],
      'Should have TypeScript'
    );
  });
});
