// Client-side session ledger: every invocation from either surface lands here.
import { esc, fmtTime, fmtDuration, stateClass, downloadJson } from './format.js';

const KEY = 'aether.ledger';
const CAP = 200;

export function load() {
  try {
    const parsed = JSON.parse(localStorage.getItem(KEY) || '[]');
    return Array.isArray(parsed) ? parsed.slice(0, CAP) : [];
  } catch {
    return [];
  }
}

function save(entries) {
  localStorage.setItem(KEY, JSON.stringify(entries.slice(0, CAP)));
}

export function append(entry) {
  const entries = load();
  const record = {
    id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    ts: Date.now(),
    ...entry,
  };
  save([record, ...entries]);
  return record;
}

export function clear() {
  save([]);
}

export function recent(count = 5) {
  return load().slice(0, count);
}

export function find(id) {
  return load().find((entry) => entry.id === id);
}

export function exportLedger() {
  downloadJson(load(), `aether-session-${new Date().toISOString().slice(0, 10)}.json`);
}

export function ledgerHtml(entries) {
  if (!entries.length) {
    return '<div class="empty"><p>No commands yet.</p><p>Run a command from a workspace, the palette, or the terminal — every invocation is recorded here.</p></div>';
  }
  return `<div class="table-wrap"><table class="ledger-table"><thead><tr><th>Time</th><th>Command</th><th>State</th><th>Exit</th><th>Duration</th><th></th></tr></thead><tbody>
    ${entries.map((entry) => `<tr>
      <td class="mono dim">${esc(fmtTime(entry.ts))}</td>
      <td><button class="link-btn" data-ledger-inspect="${esc(entry.id)}">${esc(entry.argv || entry.command || '')}</button></td>
      <td><span class="pill ${stateClass(entry.state)}">${esc(entry.state)}</span></td>
      <td class="mono">${esc(entry.exit_code)}</td>
      <td class="mono dim">${esc(fmtDuration(entry.duration_ms))}</td>
      <td class="row-actions"><button class="icon-btn" data-ledger-rerun="${esc(entry.id)}" title="Run again">↻</button><button class="icon-btn" data-copy-argv="${esc(entry.id)}" title="Copy argv">⧉</button></td>
    </tr>`).join('')}
  </tbody></table></div>`;
}
