// Aether local console — zero dependencies, all data from the local API.
const $ = (sel, root = document) => root.querySelector(sel);
const $$ = (sel, root = document) => [...root.querySelectorAll(sel)];

const state = {
  token: null,
  catalog: null,
  commands: [],
  status: null,
  view: 'overview',
  results: {},
  paletteOpen: false,
  paletteQuery: '',
  paletteIndex: 0,
  rawCommand: '',
  rawFlags: '--json',
};

// The token is passed in the URL fragment so it is never sent to a server
// in a request or stored in a cookie. Move it into sessionStorage at once.
(function initToken() {
  const match = location.hash.match(/token=([0-9a-f]{64})/);
  if (match) {
    sessionStorage.setItem('aether.token', match[1]);
    history.replaceState(null, '', location.pathname);
  }
  state.token = sessionStorage.getItem('aether.token');
})();

function esc(value) {
  return String(value).replace(/[&<>"']/g, c => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

function jsonHtml(value) {
  const text = JSON.stringify(value, null, 2) ?? 'null';
  return esc(text).replace(
    /("(?:\\.|[^"\\])*")(\s*:)?|\b(true|false)\b|\b(null)\b|(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)/g,
    (m, str, colon, bool, nul, num) => {
      if (str) return `<span class="${colon ? 'j-key' : 'j-str'}">${str}</span>${colon || ''}`;
      if (bool) return `<span class="j-bool">${bool}</span>`;
      if (nul) return `<span class="j-null">${nul}</span>`;
      return `<span class="j-num">${num}</span>`;
    }
  );
}

async function api(path, options = {}) {
  const headers = { ...(options.headers || {}), Authorization: `Bearer ${state.token}` };
  if (options.body) headers['Content-Type'] = 'application/json';
  const response = await fetch(`/api${path}`, { ...options, headers });
  const text = await response.text();
  let data;
  try { data = JSON.parse(text); } catch { data = { error: 'non_json', message: text.slice(0, 500) }; }
  return { ok: response.ok, status: response.status, data };
}

function stateClass(value) {
  if (value === 'ok') return 'ok';
  if (value === 'blocked') return 'blocked';
  return 'error';
}

function resultHtml(result) {
  if (!result) return '';
  const pillClass = stateClass(result.state);
  return `
    <div class="result">
      <span class="pill ${pillClass}">${esc(result.state)}</span>
      <span class="pill">exit ${result.exit_code}</span>
      <span class="pill">${result.duration_ms} ms</span>
      <pre class="json">${jsonHtml(result.value)}</pre>
      ${result.text ? `<pre class="json">${esc(result.text)}</pre>` : ''}
      ${result.stderr ? `<div class="stderr">${esc(result.stderr)}</div>` : ''}
    </div>`;
}

async function boot() {
  if (!state.token) {
    document.getElementById('app').innerHTML =
      '<div class="fatal">Missing token. Launch <code>aether-web</code> and use the printed URL.</div>';
    return;
  }
  const [catalog, commands, status] = await Promise.all([
    api('/catalog'), api('/commands'), api('/status'),
  ]);
  if (!catalog.ok || !commands.ok) {
    document.getElementById('app').textContent = 'API unavailable: ' + (catalog.data.message || commands.data.message);
    return;
  }
  state.catalog = catalog.data;
  state.commands = commands.data;
  state.status = status.ok ? status.data : { error: status.data.message };
  renderShell();
}

function renderShell() {
  const nav = state.catalog.workspaces.map(ws => `
    <button class="nav-item ${state.view === ws.id ? 'active' : ''}" data-view="${ws.id}">
      <span>${esc(ws.title)}</span>
      <span class="count">${state.catalog.commands.filter(c => c.workspace === ws.id).length}</span>
    </button>`).join('');
  document.getElementById('app').innerHTML = `
    <div class="shell">
      <aside class="sidebar">
        <div class="brand"><b>Aether</b><span>local console</span></div>
        <nav class="nav">
          <button class="nav-item ${state.view === 'overview' ? 'active' : ''}" data-view="overview">Overview</button>
          <div class="nav-label">Workspaces</div>
          ${nav}
          <div class="nav-label spaced">Tools</div>
          <button class="nav-item ${state.view === 'raw' ? 'active' : ''}" data-view="raw">Raw command</button>
        </nav>
        <div class="nav-foot">
          <kbd>⌘K</kbd> command palette<br>
          <code>127.0.0.1</code> only · no egress
        </div>
      </aside>
      <main class="main" id="main"></main>
    </div>
    <div class="overlay" id="overlay">
      <div class="palette">
        <input id="palette-input" placeholder="Run a command…" autocomplete="off">
        <div class="palette-list" id="palette-list"></div>
      </div>
    </div>`;

  $$('[data-view]').forEach(button => button.addEventListener('click', () => {
    state.view = button.dataset.view;
    renderShell();
  }));
  // Keydown is registered once at module init; renderShell must not add
  // another listener each time it rebuilds the DOM.
  renderMain();
}

function renderMain() {
  const main = document.getElementById('main');
  if (state.view === 'overview') main.innerHTML = overviewHtml();
  else if (state.view === 'raw') main.innerHTML = rawHtml();
  else main.innerHTML = workspaceHtml(state.view);
  bindMain();
}

function overviewHtml() {
  const s = state.status || {};
  const ws = state.catalog.workspaces.map(w => `
    <div class="card">
      <div class="meta">${esc(w.id)}</div>
      <h2>${esc(w.title)}</h2>
      <p>${esc(w.description)}</p>
    </div>`).join('');
  const capabilities = Object.entries(s)
    .filter(([k, v]) => typeof v === 'string' && !k.startsWith('graph'))
    .map(([k, v]) => `<div class="card"><div class="meta">${esc(k)}</div><p>${esc(v)}</p></div>`).join('');
  return `
    <div class="eyebrow">Local control surface</div>
    <div class="hero">
      <div class="hero-card">
        <h1>Aether</h1>
        <span class="version">${esc(s.version || 'unknown')}</span>
        <p class="lead phase">${esc(s.phase || '')}</p>
        <div class="disclaimer">Certification claim: ${esc(String(s.certification_claim))} — this interface reports what the platform can prove, nothing more.</div>
      </div>
      <div class="stats">
        <div class="stat"><div class="n">${esc(s.contracts ?? '—')}</div><div class="l">contracts</div></div>
        <div class="stat"><div class="n">${esc(s.graph_node_types ?? '—')}</div><div class="l">node types</div></div>
        <div class="stat"><div class="n">${esc(s.graph_edge_types ?? '—')}</div><div class="l">edge types</div></div>
        <div class="stat"><div class="n">${state.commands.length}</div><div class="l">commands</div></div>
      </div>
    </div>
    <h3>Graph</h3>
    <div class="grid"><div class="card"><p>${esc(JSON.stringify(s.graph || {}))}</p></div></div>
    <div class="rule"></div>
    <h3>Capability surface</h3>
    <div class="grid">${capabilities}</div>
    <div class="rule"></div>
    <h3>Workspaces</h3>
    <div class="grid">${ws}</div>`;
}

function workspaceHtml(id) {
  const workspace = state.catalog.workspaces.find(w => w.id === id);
  if (!workspace) return '<p>Unknown workspace.</p>';
  const commands = state.catalog.commands.filter(c => c.workspace === id);
  const cards = commands.map((cmd, i) => commandCard(cmd, i)).join('');
  return `
    <div class="eyebrow">Workspace · ${esc(id)}</div>
    <h1>${esc(workspace.title)}</h1>
    <p class="lead">${esc(workspace.description)}</p>
    <div class="hint">CLI is the single source of truth. Every action below spawns the canonical binary with machine-readable output.</div>
    <div class="rule"></div>
    ${cards}`;
}

function commandCard(cmd, index) {
  const fields = cmd.args.map(arg => `
    <div class="field">
      <label>${esc(arg.label)}${arg.required ? ' *' : ''}</label>
      <input data-cmd="${index}" data-arg="${esc(arg.name)}" data-kind="${arg.kind}"
             placeholder="${arg.kind === 'path' ? '/path/to/file' : esc(arg.name)}">
    </div>`).join('');
  const approval = cmd.kind === 'approval'
    ? '<div class="disclaimer">Requires an explicit human authority. Supply the approver identity before execution.</div>'
    : '';
  const result = state.results[cmd.name] ? resultHtml(state.results[cmd.name]) : '';
  return `
    <div class="card cmd" id="cmd-${index}">
      <div class="cmd-head" data-toggle="${index}">
        <div>
          <div class="name">aether ${esc(cmd.name)}</div>
          <div class="about">${esc(cmd.about)} · <span class="pill">${esc(cmd.kind)}</span></div>
        </div>
        <span class="pill">${cmd.args.length} arg${cmd.args.length === 1 ? '' : 's'}</span>
      </div>
      <div class="cmd-body">
        ${approval}
        <div class="form">${fields}</div>
        <div class="actions">
          <button class="btn primary" data-run="${index}">Run</button>
          <button class="btn" data-copy="${index}">Copy command</button>
          <span class="hint">--json added automatically</span>
        </div>
        <div id="result-${index}">${result}</div>
      </div>
    </div>`;
}

function rawHtml() {
  return `
    <div class="eyebrow">Advanced</div>
    <h1>Raw command</h1>
    <p class="lead">Run any command advertised by <code>aether discover commands</code>. Arguments are passed as discrete argv elements — no shell is involved.</p>
    <div class="card">
      <div class="form">
        <div class="field"><label>Command tokens</label><input id="raw-command" value="${esc(state.rawCommand)}" placeholder="schema list"></div>
        <div class="field"><label>Flags</label><input id="raw-flags" value="${esc(state.rawFlags)}" placeholder="--json"></div>
      </div>
      <div class="actions"><button class="btn primary" id="raw-run">Run</button></div>
      <div id="raw-result">${state.results['__raw'] ? resultHtml(state.results['__raw']) : ''}</div>
    </div>`;
}

function bindMain() {
  $$('[data-toggle]').forEach(el => el.addEventListener('click', () => {
    document.getElementById(`cmd-${el.dataset.toggle}`).classList.toggle('open');
  }));
  $$('[data-run]').forEach(button => button.addEventListener('click', async () => {
    const index = button.dataset.run;
    const cmd = state.catalog.commands.filter(c => c.workspace === state.view)[Number(index)];
    const values = {};
    $$( `[data-cmd="${index}"]`).forEach(input => {
      values[input.dataset.arg] = input.value;
    });
    button.disabled = true;
    button.textContent = 'Running…';
    const response = await api('/invoke', {
      method: 'POST',
      body: JSON.stringify({ name: cmd.name, values }),
    });
    state.results[cmd.name] = response.ok ? response.data : {
      state: 'error', exit_code: response.status, value: response.data, stderr: '', duration_ms: 0,
    };
    document.getElementById(`result-${index}`).innerHTML = resultHtml(state.results[cmd.name]);
    button.disabled = false;
    button.textContent = 'Run';
  }));
  $$('[data-copy]').forEach(button => button.addEventListener('click', () => {
    const index = button.dataset.copy;
    const cmd = state.catalog.commands.filter(c => c.workspace === state.view)[Number(index)];
    const tokens = [`aether ${cmd.name}`];
    $$(`[data-cmd="${index}"]`).forEach(input => { if (input.value) tokens.push(input.value); });
    navigator.clipboard.writeText(tokens.join(' '));
    button.textContent = 'Copied';
    setTimeout(() => { button.textContent = 'Copy command'; }, 1200);
  }));
  const rawRun = document.getElementById('raw-run');
  if (rawRun) rawRun.addEventListener('click', async () => {
    state.rawCommand = document.getElementById('raw-command').value.trim();
    state.rawFlags = document.getElementById('raw-flags').value.trim();
    const response = await api('/raw', {
      method: 'POST',
      body: JSON.stringify({ command: state.rawCommand.split(/\s+/).filter(Boolean), flags: state.rawFlags.split(/\s+/).filter(Boolean) }),
    });
    state.results['__raw'] = response.ok ? response.data : {
      state: 'error', exit_code: response.status, value: response.data, stderr: '', duration_ms: 0,
    };
    document.getElementById('raw-result').innerHTML = resultHtml(state.results['__raw']);
  });
}

function paletteMatches() {
  const query = state.paletteQuery.toLowerCase().trim();
  const all = [...state.catalog.commands.map(c => ({ name: c.name, about: c.about })), ...state.commands];
  const seen = new Set();
  return all.filter(c => {
    if (seen.has(c.name)) return false;
    seen.add(c.name);
    return !query || c.name.toLowerCase().includes(query) || (c.about || '').toLowerCase().includes(query);
  }).slice(0, 40);
}

function renderPalette() {
  const list = paletteMatches();
  state.paletteIndex = Math.min(state.paletteIndex, Math.max(0, list.length - 1));
  document.getElementById('palette-list').innerHTML = list.map((c, i) => `
    <button class="palette-item ${i === state.paletteIndex ? 'active' : ''}" data-pick="${esc(c.name)}">
      <div class="n">aether ${esc(c.name)}</div>
      <div class="a">${esc(c.about || '')}</div>
    </button>`).join('');
  $$('[data-pick]').forEach(item => item.addEventListener('click', () => pickPalette(item.dataset.pick)));
}

function openPalette() {
  state.paletteOpen = true;
  document.getElementById('overlay').classList.add('open');
  const input = document.getElementById('palette-input');
  input.value = '';
  state.paletteQuery = '';
  state.paletteIndex = 0;
  renderPalette();
  input.focus();
  input.oninput = () => { state.paletteQuery = input.value; state.paletteIndex = 0; renderPalette(); };
}

function closePalette() {
  state.paletteOpen = false;
  document.getElementById('overlay').classList.remove('open');
}

function pickPalette(name) {
  closePalette();
  state.rawCommand = name;
  state.rawFlags = '--json';
  state.view = 'raw';
  renderShell();
}

function onKey(event) {
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
    event.preventDefault();
    state.paletteOpen ? closePalette() : openPalette();
    return;
  }
  if (event.key === 'Escape' && state.paletteOpen) { closePalette(); return; }
  if (state.paletteOpen && (event.key === 'ArrowDown' || event.key === 'ArrowUp')) {
    event.preventDefault();
    const count = paletteMatches().length;
    state.paletteIndex = (state.paletteIndex + (event.key === 'ArrowDown' ? 1 : -1) + count) % Math.max(count, 1);
    renderPalette();
    return;
  }
  if (state.paletteOpen && event.key === 'Enter') {
    const list = paletteMatches();
    if (list[state.paletteIndex]) pickPalette(list[state.paletteIndex].name);
  }
}

boot();
document.addEventListener('keydown', onKey);
