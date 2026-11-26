// Live command jobs. Every run is one CLI invocation; the browser follows
// its real stdout/stderr events and stops when the CLI process exits.
import { token } from './api.js';

const MAX_LINES = 250;

function headers() {
  return { Authorization: `Bearer ${token() || ''}`, 'Content-Type': 'application/json' };
}

async function json(path, options = {}) {
  const response = await fetch(`/api${path}`, { ...options, headers: { ...headers(), ...(options.headers || {}) } });
  const data = await response.json().catch(() => ({}));
  if (!response.ok) throw new Error(data.message || `request failed (${response.status})`);
  return data;
}

/** One long-poll watcher per job, so two views never race each other. */
export class JobRunner {
  constructor({ onEvent = () => {}, onState = () => {}, onFinish = () => {}, onError = () => {} } = {}) {
    this.handlers = { onEvent, onState, onFinish, onError };
    this.jobs = new Map();
    this.watching = new Set();
  }

  remember(job) {
    const current = this.jobs.get(job.id) || { id: job.id, argv: job.argv || [], events: [], lines: [], state: 'running' };
    const merged = { ...current, ...job };
    this.jobs.set(job.id, merged);
    return merged;
  }

  get(id) { return this.jobs.get(id) || null; }
  list() { return [...this.jobs.values()]; }

  async start(argv) {
    const job = this.remember(await json('/jobs', {
      method: 'POST',
      body: JSON.stringify({ argv }),
    }));
    this.watch(job.id);
    return job;
  }

  watch(id) {
    if (this.watching.has(id)) return;
    this.watching.add(id);
    void this.poll(id).finally(() => this.watching.delete(id));
  }

  async poll(id) {
    let after = 0;
    while (true) {
      try {
        const feed = await json(`/jobs/${encodeURIComponent(id)}/events?after=${after}`);
        after = feed.event_count;
        const job = this.get(id);
        if (!job) break;
        for (const event of feed.events) {
          job.events.push(event);
          if (event.type === 'output') {
            job.lines.push({ stream: event.stream, text: event.data });
            if (job.lines.length > MAX_LINES) job.lines.shift();
          }
          this.handlers.onEvent(job, event);
        }
        const snapshot = await json(`/jobs/${encodeURIComponent(id)}`);
        Object.assign(job, snapshot, { lines: job.lines, events: job.events });
        this.handlers.onState(job);
        if (feed.finished) {
          this.handlers.onFinish(job);
          break;
        }
      } catch (error) {
        this.handlers.onError(id, error);
        break;
      }
    }
  }

  async cancel(id) {
    const snapshot = await json(`/jobs/${encodeURIComponent(id)}/cancel`, { method: 'POST' });
    const job = this.get(id);
    if (job) Object.assign(job, snapshot);
    this.handlers.onState(job);
    return snapshot;
  }

  async snapshot(id) {
    const snapshot = await json(`/jobs/${encodeURIComponent(id)}`);
    return this.remember(snapshot);
  }

  async saved() {
    const list = await json('/jobs');
    for (const job of list) this.remember(job);
    return this.list();
  }
}

/** Render the live run dock from the jobs the runner remembers. */
export function renderDock(host, runner, { onCancel, onSelect } = {}) {
  const jobs = runner.list().slice(-4).reverse();
  if (!jobs.length) {
    host.innerHTML = '<div class="live-empty">No runs yet this session. Commands and terminal entries appear here live.</div>';
    return;
  }
  host.innerHTML = jobs.map((job) => {
    const active = job.state === 'running';
    const lines = job.lines.slice(-14).map((line) =>
      `<div class="live-line ${line.stream}">${escapeHtml(line.text)}</div>`).join('');
    return `<div class="live-job ${job.state}" data-job="${job.id}">
      <div class="live-head">
        <span class="pill ${job.state}">${job.state}</span>
        <code>aether ${escapeHtml((job.argv || []).join(' '))}</code>
        ${active ? `<button class="btn ghost mini" data-cancel="${job.id}">Cancel</button>` : ''}
      </div>
      <div class="live-meter"><span class="live-elapsed" data-elapsed="${job.started_ms || 0}" data-duration="${job.duration_ms ?? ''}">${elapsed(job)}</span>
        <span class="dim">${job.exit_code === undefined || job.exit_code === null ? '' : `exit ${job.exit_code}`}</span></div>
      ${lines ? `<pre class="live-log">${lines}</pre>` : ''}
      ${active ? '<div class="live-pulse" aria-label="command running"></div>' : ''}
      ${job.result || job.text ? `<button class="btn ghost mini" data-select="${job.id}">View result</button>` : ''}
    </div>`;
  }).join('');
  host.querySelectorAll('[data-cancel]').forEach((button) => button.addEventListener('click', () => onCancel?.(button.dataset.cancel)));
  host.querySelectorAll('[data-select]').forEach((button) => button.addEventListener('click', () => onSelect?.(button.dataset.select)));
}

function elapsed(job) {
  if (job.duration_ms !== undefined && job.duration_ms !== null) return `${(job.duration_ms / 1000).toFixed(1)}s`;
  const start = job.started_ms || Date.now();
  return `${((Date.now() - start) / 1000).toFixed(1)}s`;
}

export function tickDock(host) {
  host.querySelectorAll('[data-elapsed]').forEach((node) => {
    const duration = node.dataset.duration;
    const job = {
      started_ms: Number(node.dataset.elapsed),
      duration_ms: duration === '' ? undefined : Number(duration),
    };
    node.textContent = elapsed(job);
  });
}

function escapeHtml(value) {
  return String(value).replace(/[&<>"']/g, (char) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  })[char]);
}
