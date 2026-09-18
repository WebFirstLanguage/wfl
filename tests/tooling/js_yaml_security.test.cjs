const assert = require('node:assert/strict');
const { createRequire } = require('node:module');
const path = require('node:path');
const { test } = require('node:test');

const extensionRequire = createRequire(
  path.resolve(__dirname, '../../vscode-extension/package.json')
);
const yaml = extensionRequire('js-yaml');

// GHSA-2883-xcg3-v3hh: use tiny inputs and explicit budgets to reproduce the
// empty-source bypass without a timing assertion or CPU-exhaustion payload.
const emptyMerges = 'sources: &sources [{}, {}]\ntargets:\n  - <<: *sources\n  - <<: *sources\n';

test('ordinary YAML merges preserve values and explicit overrides', () => {
  const result = yaml.load('defaults: &defaults {enabled: true, name: default}\nconfig: {<<: *defaults, name: wfl}\n', {
    maxTotalMergeKeys: 3
  });
  assert.deepEqual(result.config, { enabled: true, name: 'wfl' });
});

test('empty merge sources are accepted at the exact work budget', () => {
  assert.deepEqual(yaml.load(emptyMerges, { maxTotalMergeKeys: 4 }), {
    sources: [{}, {}],
    targets: [{}, {}]
  });
});

test('repeated empty merge sources exhaust the document work budget', () => {
  assert.throws(() => yaml.load(emptyMerges, { maxTotalMergeKeys: 3 }), {
    name: 'YAMLException',
    message: /maxTotalMergeKeys/
  });
});

test('even a single empty merge source consumes a work unit', () => {
  assert.throws(() => yaml.load('target: {<<: {}}\n', { maxTotalMergeKeys: 0 }), {
    name: 'YAMLException',
    message: /maxTotalMergeKeys/
  });
});
