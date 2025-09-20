import { defineConfig } from '@vscode/test-cli';

export default defineConfig({
  files: 'out/test/**/*.test.js',
  workspaceFolder: './test-workspace',
  mocha: {
    ui: 'bdd',
    timeout: 30000,
    color: true
  },
  extensionDevelopmentPath: '.',
  extensionTestsPath: './out/test/suite/index.js'
});
