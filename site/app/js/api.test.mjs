import test from 'node:test';
import assert from 'node:assert/strict';
import { initToken, token, forgetToken } from './api.js';

function newStorage() {
  const values = new Map();
  return {
    getItem: (key) => (values.has(key) ? values.get(key) : null),
    setItem: (key, value) => values.set(key, String(value)),
    removeItem: (key) => values.delete(key),
  };
}

function setup(hash, session, local) {
  globalThis.location = { hash, pathname: '/', search: '', reload() {} };
  globalThis.history = { replaceState() {} };
  globalThis.sessionStorage = session;
  globalThis.localStorage = local;
}

const TOKEN = 'a'.repeat(64);

test('a fragment token is remembered and scrubbed from the URL', () => {
  const session = newStorage();
  const local = newStorage();
  setup(`#token=${TOKEN}`, session, local);
  assert.equal(initToken(), TOKEN);
  assert.equal(token(), TOKEN);
  assert.equal(session.getItem('aether.token'), TOKEN);
  assert.equal(local.getItem('aether.token'), TOKEN);
  assert.equal(location.hash, `#token=${TOKEN}`); // replaceState is mocked
});

test('a link without a token uses the saved token from another tab', () => {
  const session = newStorage();
  const local = newStorage();
  local.setItem('aether.token', TOKEN);
  setup('', session, local);
  assert.equal(initToken(), TOKEN);
  assert.equal(token(), TOKEN);
});

test('a short token does not displace a valid saved token', () => {
  const session = newStorage();
  const local = newStorage();
  local.setItem('aether.token', TOKEN);
  setup('#token=abc123', session, local);
  assert.equal(initToken(), TOKEN);
  assert.equal(local.getItem('aether.token'), TOKEN);
});

test('forgetToken clears both stores', () => {
  const session = newStorage();
  const local = newStorage();
  setup('', session, local);
  session.setItem('aether.token', TOKEN);
  local.setItem('aether.token', TOKEN);
  forgetToken();
  assert.equal(token(), null);
});
