import test from 'node:test';
import assert from 'node:assert/strict';
import { JobRunner } from './jobs.js';

const TOKEN = 'b'.repeat(64);

function setupFetch() {
  let calls = 0;
  globalThis.localStorage = {
    getItem: () => TOKEN,
    setItem() {}, removeItem() {},
  };
  globalThis.fetch = async (url, options = {}) => {
    calls += 1;
    let data;
    if (options.method === 'POST' && url === '/api/jobs') {
      data = { id: 'j1', argv: ['status'], state: 'running', started_ms: 1000 };
    } else if (url.includes('/events')) {
      data = {
        id: 'j1', after: 0, event_count: 2, finished: true, state: 'ok',
        events: [
          { type: 'started', at: 1000 },
          { type: 'output', stream: 'stdout', data: '{"ok":true}' },
        ],
      };
    } else if (url.endsWith('/jobs/j1')) {
      data = {
        id: 'j1', argv: ['status'], state: 'ok', started_ms: 1000,
        finished_ms: 1200, duration_ms: 200, exit_code: 0, event_count: 2,
        result: { command: 'status', state: 'ok', exit_code: 0, value: { ok: true }, duration_ms: 200 },
      };
    } else {
      data = [];
    }
    return { ok: true, status: 200, json: async () => data };
  };
  return () => calls;
}

test('a job follows real events and resolves when the CLI exits', async () => {
  const calls = setupFetch();
  const finished = [];
  const runner = new JobRunner({ onFinish: (job) => finished.push(job) });
  const job = await runner.start(['status']);
  await new Promise((resolve) => setTimeout(resolve, 20));
  assert.equal(job.id, 'j1');
  assert.equal(finished.length, 1);
  assert.equal(finished[0].state, 'ok');
  assert.equal(finished[0].lines.length, 1);
  assert.equal(finished[0].lines[0].text, '{"ok":true}');
  assert.ok(calls() >= 3);
});

test('a rejected start is reported without a hanging watcher', async () => {
  setupFetch();
  globalThis.fetch = async () => ({
    ok: false, status: 400,
    json: async () => ({ message: 'invalid command token' }),
  });
  const runner = new JobRunner();
  await assert.rejects(() => runner.start(['bad|token']), /invalid command token/);
  assert.equal(runner.list().length, 0);
});
