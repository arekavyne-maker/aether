import test from 'node:test';
import assert from 'node:assert/strict';
import { tokenize, formatArgv, isBuiltin } from './argv.js';

test('splits simple commands and strips the aether prefix', () => {
  assert.deepEqual(tokenize('schema list').tokens, ['schema', 'list']);
  assert.deepEqual(tokenize('aether status --json').tokens, ['status', '--json']);
});

test('respects quotes and escapes without a shell', () => {
  const result = tokenize('vv coverage "/tmp/my requirements.yaml"');
  assert.equal(result.error, null);
  assert.deepEqual(result.tokens, ['vv', 'coverage', '/tmp/my requirements.yaml']);
  assert.deepEqual(tokenize("policy explain 'graph.mutate'").tokens, ['policy', 'explain', 'graph.mutate']);
});

test('rejects shell operators and unterminated quotes', () => {
  assert.match(tokenize('status && rm -rf /').error, /shell operator/);
  assert.match(tokenize('status > out.txt').error, /shell operator/);
  assert.match(tokenize('status | head').error, /shell operator/);
  assert.match(tokenize('status "oops').error, /unterminated/);
});

test('empty input and builtins', () => {
  assert.deepEqual(tokenize('   ').tokens, []);
  assert.equal(isBuiltin(tokenize('clear').tokens), true);
  assert.equal(isBuiltin(tokenize('status').tokens), false);
});

test('display formatter quotes ambiguous tokens', () => {
  assert.equal(formatArgv(['vv', 'coverage', '/tmp/my file.yaml']), 'vv coverage "/tmp/my file.yaml"');
});
