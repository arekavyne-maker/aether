// Aether local console — shell, workspaces, composer, inspector and bindings.
// Zero build step, zero dependencies, zero external requests.
import { initToken, api, postJSON, forgetToken } from './api.js';
import { icon, WORKSPACE_ICONS } from './icons.js';
import { esc, resultHtml } from './format.js';
import { formatArgv } from './argv.js';
import { groupCommands, roman } from './surface.js';
import { Terminal, terminalHelp } from './terminal.js';
import * as ledger from './ledger.js';
import * as palette from './palette.js';
import { loadAtlas, loadHealth, normalize as normalizeAtlas } from './data/atlas.js';
import * as router from './state/router.js';
import { renderPulse } from './views/pulse.js';
import { renderAtlas } from './views/atlas.js';
import { renderAssurance } from './views/assurance.js';
import { renderCapability } from './views/capability.js';

const state = {
  surface: null,
  status: null,
  view: 'overview',
  workspace: null,
  selected: null,
  helps: new Map(),
  filter: '',
  kind: 'all',
  result: null,
  resultName: '',
  lastResult: null,
  favorites: new Set(),
  theme: 'night',
  fileTarget: null,
  atlas: null,
  health: null,
  atlasTab: 'graph',
};

const MANAGED_FLAGS = new Set(['--json', '--offline', '-h', '--help']);
const PATH_HINT = /(path|file|yaml|json|document|manifest|snapshot|bundle|root|spec|workflow|candidate|envelope|proposal|card|directory)/i;

const terminal = new Terminal({
  surface: () => state.surface,
  getHelp: (name) => state.helps.get(name),
  onInvoke: runArgv,
  onBuiltin: runBuiltin,
});

function $(selector, root = document) { return root.querySelector(selector); }
function $$(selector, root = document) { return [...root.querySelectorAll(selector)]; }

function fatal(message, detail = '') {
  document.getElementById('app').innerHTML = `<div class="fatal">
    <div class="eyebrow">Cannot start</div><h1>Local console unavailable</h1>
    <p>${esc(message)}</p>
    <p class="dim">${esc(detail)}</p>
    <button class="btn" id="fatal-refresh">Retry this link</button>
    <button class="btn ghost" id="fatal-forget">Clear saved token</button>
  </div>`;
  $('#fatal-refresh')?.addEventListener('click', () => location.reload());
  $('#fatal-forget')?.addEventListener('click', () => {
    forgetToken();
    location.reload();
  });
}

async function boot() {
  state.theme = localStorage.getItem('aether.theme') || 'night';
  document.documentElement.dataset.theme = state.theme;
  try {
    state.favorites = new Set(JSON.parse(localStorage.getItem('aether.favorites') || '[]'));
  } catch { state.favorites = new Set(); }

  if (!initToken()) {
    fatal(
      'No console token is saved for this browser.',
      'Open the URL printed by aether-web once. It contains a 64-character token in the fragment; after that, plain links work in every tab.',
    );
    return;
  }
  const [surface, status, atlas, health] = await Promise.all([
    api('/surface'), api('/status'), loadAtlas(), loadHealth(),
  ]);
  if (!surface.ok) {
    const rejected = surface.status === 401;
    fatal(
      rejected ? 'The saved token was rejected by the server.' : (surface.data?.message || 'The local API did not respond.'),
      rejected
        ? 'The server may have restarted with a new token. Open the URL it printed, or clear the saved token and use a fresh link.'
        : 'Check that aether-web is running on this address.',
    );
    return;
  }
  state.surface = surface.data;
  state.status = status.ok ? status.data : {};
  state.atlas = atlas.ok ? normalizeAtlas(atlas.data) : normalizeAtlas(null);
  state.health = health.ok ? (health.data.health || {}) : {};
  renderShell();
  const route = router.parse(location.hash);
  if (route.view !== 'pulse') {
    state.view = route.view;
    state.workspace = route.workspace || null;
    state.atlasTab = route.tab || 'graph';
  }
  renderView();
  router.onChange((next) => {
    if (next.view === state.view && next.workspace === (state.workspace || '') && next.tab === state.atlasTab) return;
    state.view = next.view;
    state.workspace = next.workspace || null;
    if (next.tab) state.atlasTab = next.tab;
    closeInspector();
    syncChrome(state.view);
    renderView();
  });
}

function renderShell() {
  const destinations = [
    ['pulse', 'Pulse', 'overview'],
    ['atlas', 'Atlas', 'graph'],
    ['assurance', 'Assurance', 'verification'],
    ['build', 'Build', 'design'],
    ['govern', 'Govern', 'intelligence'],
    ['commands', 'Commands', 'project'],
  ];
  const nav = destinations.map(([id, label, glyph]) => `
    <button class="nav-item ${state.view === id ? 'active' : ''}" data-view="${id}">
      <span class="nav-icon">${icon(WORKSPACE_ICONS[glyph] || 'overview')}</span>
      <span class="nav-label">${label}</span>
    </button>`).join('');
  const themeIcon = state.theme === 'night' ? 'sun' : 'moon';
  document.getElementById('app').innerHTML = `
    <div class="shell">
      <aside class="rail" id="rail">
        <div class="brand"><span class="brand-mark">A</span><span class="brand-text"><b>Aether</b><small>local console</small></span></div>
        <div class="nav-label">Workspace</div>
        <nav>${nav}</nav>
        <div class="nav-label spaced">Tools</div>
        <button class="nav-item ${state.view === 'workspace' ? 'active' : ''}" data-view="workspace">
          <span class="nav-icon">${icon('project')}</span><span class="nav-label">Command index</span>
        </button>
        <button class="nav-item ${state.view === 'terminal' ? 'active' : ''}" data-view="terminal">
          <span class="nav-icon">${icon('terminal')}</span><span class="nav-label">CLI terminal</span>
        </button>
        <button class="nav-item ${state.view === 'session' ? 'active' : ''}" data-view="session">
          <span class="nav-icon">${icon('ledger')}</span><span class="nav-label">Session ledger</span>
        </button>
        <div class="rail-foot"><span class="dot"></span>127.0.0.1 · air-gapped</div>
      </aside>
      <div class="area">
        <header class="topbar">
          <button class="icon-btn menu-btn" id="menu-btn" title="Toggle navigation">${icon('project')}</button>
          <div class="crumb" id="crumb">Overview</div>
          <div class="top-actions">
            <button class="btn search-btn" id="open-palette">${icon('search')}<span>Search commands</span><kbd>⌘K</kbd></button>
            <span class="pill ok">offline</span>
            <button class="icon-btn" id="theme-btn" title="Toggle theme">${icon(themeIcon)}</button>
            <button class="icon-btn" id="help-btn" title="Shortcuts">${icon('help')}</button>
          </div>
        </header>
        <main class="stage" id="stage"></main>
      </div>
      <aside class="inspector" id="inspector"></aside>
      <div class="dock"><button class="dock-prompt" id="dock-open"><span>aether &rsaquo;</span><em>Run a command in the CLI</em></button><span class="dock-hint">⌘J terminal · ⌘K palette</span></div>
    </div>
    <div class="overlay" id="palette-overlay"><div class="palette">
      <input id="palette-input" placeholder="Search commands, workspaces, actions…" autocomplete="off">
      <div id="palette-list" class="palette-list"></div>
      <div class="palette-foot"><kbd>↵</kbd> open · <kbd>esc</kbd> close · <kbd>↑↓</kbd> navigate</div>
    </div></div>
    <div class="overlay" id="file-overlay"><div class="file-picker">
      <div class="file-head"><div><div class="eyebrow">Project files</div><h2>Choose a path</h2></div><button class="icon-btn" data-close-files>${icon('close')}</button></div>
      <input id="file-search" placeholder="Filter by name…" autocomplete="off">
      <div id="file-list" class="file-list"></div>
    </div></div>
    <div class="overlay" id="help-overlay"><div class="help-sheet">
      <div class="file-head"><div><div class="eyebrow">Reference</div><h2>Keyboard & model</h2></div><button class="icon-btn" data-close-help>${icon('close')}</button></div>
      <div class="help-grid">
        <div><h4>Keyboard</h4><ul class="bullets">
          <li><kbd>⌘K</kbd> command palette</li><li><kbd>⌘J</kbd> CLI terminal</li>
          <li><kbd>⌘↵</kbd> run the open command</li><li><kbd>Tab</kbd> complete a command or flag</li>
          <li><kbd>↑↓</kbd> terminal history</li><li><kbd>?</kbd> this overlay</li>
          <li><kbd>esc</kbd> close an overlay or inspector</li>
        </ul></div>
        <div><h4>The model</h4><p>The CLI is canonical. Every action — clicked or typed — becomes discrete argv elements for one <code>aether</code> process. No shell, no network, no parallel implementation.</p><p>Exit 30 is always shown as <span class="pill blocked">blocked</span>, never as success.</p></div>
      </div>
    </div></div>
    <div class="toasts" id="toasts"></div>`;

  $$('[data-view]').forEach((button) => button.addEventListener('click', () => setView(button.dataset.view)));
  $$('[data-workspace]').forEach((button) => button.addEventListener('click', () => {
    state.workspace = button.dataset.workspace;
    state.selected = null;
    state.filter = '';
    setView('workspace');
  }));
  $('#open-palette').addEventListener('click', () => palette.open());
  $('#dock-open').addEventListener('click', () => { setView('terminal'); terminal.focus(); });
  $('#menu-btn').addEventListener('click', () => $('#rail').classList.toggle('open'));
  $('#theme-btn').addEventListener('click', toggleTheme);
  $('#help-btn').addEventListener('click', () => $('#help-overlay').classList.add('open'));
  $('[data-close-help]').addEventListener('click', () => $('#help-overlay').classList.remove('open'));
  $('[data-close-files]').addEventListener('click', () => $('#file-overlay').classList.remove('open'));
  $('#file-search').addEventListener('input', () => loadFiles($('#file-search').value));
  $('#inspector').addEventListener('click', (event) => {
    if (event.target.closest('[data-close-inspector]')) closeInspector();
  });
  $('#toasts').addEventListener('click', (event) => {
    const button = event.target.closest('[data-copy-argv]');
    if (button) {
      const entry = ledger.find(button.dataset.copyArgv);
      if (entry) copyText(`aether ${entry.argv || entry.command}`);
    }
  });

  palette.configure({
    getSurface: () => state.surface,
    getRecent: () => ledger.recent(8).map((e) => e.argv || e.command),
    onWorkspace: (id) => { state.workspace = id; state.selected = null; setView('workspace'); },
    onCommand: (command) => {
      if (command.group) {
        state.workspace = command.workspace;
        state.selected = null;
        state.filter = command.name;
        setView('workspace');
      } else {
        state.workspace = command.workspace;
        setView('workspace').then(() => selectCommand(command.name));
      }
    },
    onAction: runAction,
  });
}

const VIEW_TITLES = {
  pulse: 'Pulse', atlas: 'Atlas', assurance: 'Assurance', build: 'Build',
  govern: 'Govern', commands: 'Commands', workspace: 'Workspace',
  terminal: 'CLI terminal', session: 'Session ledger',
};

function syncChrome(view) {
  $$('.nav-item').forEach((item) => item.classList.remove('active'));
  const target = view === 'workspace' ? $(`[data-view="workspace"]`) : $(`[data-view="${view}"]`);
  target?.classList.add('active');
  $('#rail').classList.remove('open');
  $('#crumb').textContent = view === 'workspace'
    ? (state.surface.workspaces.find((w) => w.id === state.workspace)?.title || 'Workspace')
    : (VIEW_TITLES[view] || view);
}

function setView(view, options = {}) {
  state.view = view;
  if (options.workspace) state.workspace = options.workspace;
  closeInspector();
  syncChrome(view);
  if (view === 'atlas') router.go({ view, tab: state.atlasTab });
  else if (view === 'commands' || view === 'workspace') router.go({ view, workspace: state.workspace || '' });
  else router.go({ view });
  return Promise.resolve(renderView());
}

function renderView() {
  const stage = $('#stage');
  stage.innerHTML = '';
  if (state.view === 'pulse' || state.view === 'overview') {
    renderPulse(stage, {
      atlas: state.atlas,
      surface: state.surface,
      ledgerHtml: ledger.ledgerHtml(ledger.recent(5)),
      onNavigate: (view) => setView(view === 'commands' ? 'commands' : view),
      onWorkspace: (id) => setView('workspace', { workspace: id }),
    });
  } else if (state.view === 'workspace' || state.view === 'commands') {
    if (!state.surface.workspaces.some((w) => w.id === state.workspace)) {
      state.workspace = state.surface.workspaces[0]?.id || 'overview';
    }
    stage.innerHTML = workspaceHtml();
  } else if (state.view === 'atlas') {
    renderAtlas(stage, {
      atlas: state.atlas,
      surface: state.surface,
      tab: state.atlasTab,
      onTab: (tab) => { state.atlasTab = tab; setView('atlas'); },
      onFamily: (family) => {
        const command = state.surface.commands.find((c) => c.family === family);
        state.filter = family;
        setView('workspace', { workspace: command?.workspace || state.workspace || 'overview' });
      },
    });
  } else if (state.view === 'assurance') {
    renderAssurance(stage, {
      atlas: state.atlas,
      onRequirement: () => { state.atlasTab = 'graph'; setView('atlas'); },
    });
  } else if (state.view === 'build' || state.view === 'govern') {
    renderCapability(stage, {
      kind: state.view,
      atlas: state.atlas,
      health: state.health,
      surface: state.surface,
      onCommand: (name) => {
        const command = state.surface.commands.find((c) => c.name === name);
        setView('workspace', { workspace: command?.workspace || 'overview' }).then(() => selectCommand(name));
      },
    });
  } else if (state.view === 'session') {
    stage.innerHTML = sessionHtml();
  } else if (state.view === 'terminal') {
    stage.appendChild(terminalHost());
  }
  if (state.view === 'workspace' || state.view === 'commands') bindWorkspace();
  if (state.view === 'session') bindSession();
}

let terminalNode = null;
function terminalHost() {
  if (!terminalNode) {
    terminalNode = document.createElement('div');
    terminal.mount(terminalNode);
  }
  return terminalNode;
}

function overviewHtml() {
  const s = state.status || {};
  const graph = s.graph || {};
  const cards = state.surface.workspaces.map((w, i) => `
    <button class="workspace-tile" data-workspace="${w.id}">
      <div class="tile-top"><span class="roman">${roman(i)}</span><span class="tile-icon">${icon(WORKSPACE_ICONS[w.id] || 'overview', 18)}</span></div>
      <h3>${esc(w.title)}</h3><p>${esc(w.description)}</p>
      <div class="tile-foot"><span>${w.command_count} commands</span>${icon('arrow', 14)}</div>
    </button>`).join('');
  const caps = Object.entries(s)
    .filter(([k, v]) => typeof v === 'string' && k !== 'phase')
    .slice(0, 4)
    .map(([k, v]) => `<div class="cap"><span>${esc(k.replaceAll('_', ' '))}</span><b>${esc(v)}</b></div>`).join('');
  return `
    <div class="overview">
      <section class="hero">
        <div class="eyebrow">Local control surface · ${esc(s.version || state.surface.version)}</div>
        <h1>Aether</h1>
        <p class="hero-serif">One graph.<br><em>Every discipline.</em></p>
        <div class="thread-rule"><span></span><i>${icon('overview', 14)}</i><span></span></div>
        <p class="lead">A modern local surface for the canonical CLI. Click a command, or open the terminal and type it. Both surfaces spawn the same binary and record the same session.</p>
        <div class="hero-actions"><button class="btn primary" data-view="terminal">${icon('terminal', 15)} Open CLI</button><button class="btn" data-view="session">${icon('ledger', 15)} Session ledger</button></div>
      </section>
      <section class="stat-band">
        <div><b>${esc(s.contracts ?? '—')}</b><span>contracts</span></div>
        <div><b>${esc(state.surface.commands.length)}</b><span>commands</span></div>
        <div><b>${esc(graph.nodes ?? 0)}</b><span>graph nodes</span></div>
        <div><b>${esc(graph.events ?? 0)}</b><span>events</span></div>
      </section>
      <div class="notice">${s.certification_claim ? 'Certification claim present — verify before relying on it.' : 'Certification claim: false. This console reports what the platform can prove, nothing more.'}</div>
      <h2 class="section-title">Capability surface</h2><div class="cap-grid">${caps}</div>
      <h2 class="section-title">Workspaces</h2><div class="workspace-map">${cards}</div>
      <h2 class="section-title">Recent thread</h2>${ledger.ledgerHtml(ledger.recent(5))}
    </div>`;
}

function workspaceHtml() {
  const workspace = state.surface.workspaces.find((w) => w.id === state.workspace);
  if (!workspace) return '<div class="empty">Unknown workspace.</div>';
  const index = state.surface.workspaces.indexOf(workspace);
  return `
    <div class="page-head">
      <div class="eyebrow">Workspace ${roman(index)} · ${esc(workspace.id)}</div>
      <h1>${esc(workspace.title)}</h1>
      <p class="lead">${esc(workspace.description)}</p>
    </div>
    <div class="workspace-grid">
      <section class="command-index">
        <div class="index-toolbar">
          <label class="search-field">${icon('search', 15)}<input id="cmd-filter" value="${esc(state.filter)}" placeholder="Filter commands…" autocomplete="off"></label>
          <div class="chips" id="kind-chips">${['all', 'read', 'approval', 'long', 'group'].map((k) => `<button class="chip ${state.kind === k ? 'active' : ''}" data-kind="${k}">${k}</button>`).join('')}</div>
        </div>
        <div id="command-list"></div>
      </section>
      <section class="composer" id="composer"><div class="empty"><p>Select a command to compose it.</p></div></section>
    </div>`;
}

function renderCommandList() {
  const host = $('#command-list');
  if (!host) return;
  const { pinned, groups, total } = groupCommands(state.surface, state.workspace, state.filter, state.kind, state.favorites);
  const row = (command) => `<button class="cmd-row ${state.selected === command.name ? 'active' : ''}" data-cmd="${esc(command.name)}">
    <span class="cmd-name">aether ${esc(command.name)}</span>
    <span class="cmd-about">${esc(command.about)}</span>
    <span class="cmd-tags"><span class="pill ${command.kind}">${esc(command.kind)}</span>${state.favorites.has(command.name) ? `<span class="fav">${icon('star', 12)}</span>` : ''}</span>
  </button>`;
  const family = (group) => `<details class="family" open><summary><span>${esc(group.family)}</span><span class="dim">${group.commands.length}</span></summary>${group.commands.map(row).join('')}</details>`;
  host.innerHTML = (pinned.length || groups.length)
    ? `<div class="index-count">${total} command${total === 1 ? '' : 's'}</div>
        ${pinned.length ? `<details class="family" open><summary><span>Pinned</span><span class="dim">${pinned.length}</span></summary>${pinned.map(row).join('')}</details>` : ''}
        ${groups.map(family).join('')}`
    : '<div class="empty small">No commands match this filter.</div>';
  host.querySelectorAll('[data-cmd]').forEach((button) => button.addEventListener('click', () => selectCommand(button.dataset.cmd)));
}

function bindWorkspace() {
  if (!$('#cmd-filter')) return;
  $('#cmd-filter').addEventListener('input', (event) => { state.filter = event.target.value; renderCommandList(); });
  $$('[data-kind]').forEach((chip) => chip.addEventListener('click', () => {
    state.kind = chip.dataset.kind;
    $$('[data-kind]').forEach((c) => c.classList.toggle('active', c === chip));
    renderCommandList();
  }));
  renderCommandList();
  if (state.selected) selectCommand(state.selected);
}

async function selectCommand(name) {
  state.selected = name;
  const command = state.surface.commands.find((c) => c.name === name);
  if (!command) return;
  $$('.cmd-row').forEach((row) => row.classList.toggle('active', row.dataset.cmd === name));
  const composer = $('#composer');
  if (!composer) return;
  composer.innerHTML = `<div class="eyebrow">Command</div><h2>aether ${esc(name)}</h2><p class="lead">${esc(command.about)}</p><div class="empty small">Reading command contract…</div>`;
  let help = state.helps.get(name);
  if (!help) {
    const response = await api(`/describe?name=${encodeURIComponent(name)}`);
    if (!response.ok) {
      composer.innerHTML = `<div class="notice bad">${esc(response.data?.message || 'Help unavailable')}</div>`;
      return;
    }
    help = response.data;
    state.helps.set(name, help);
  }
  if (state.selected !== name) return;
  renderComposer(command, help);
}

function field(label, input, hint = '') {
  return `<div class="field"><label>${label}</label>${input}${hint ? `<small class="dim">${esc(hint)}</small>` : ''}</div>`;
}

function browseButton(key) {
  return `<button class="icon-btn browse" data-browse="${esc(key)}" title="Choose a project file">${icon('folder', 15)}</button>`;
}

function renderComposer(command, help) {
  const composer = $('#composer');
  if (!composer) return;
  const groups = help.subcommands || [];
  if (groups.length) {
    const children = state.surface.commands.filter((c) => c.name.startsWith(`${command.name} `));
    composer.innerHTML = `<div class="eyebrow">Command group</div><h2>aether ${esc(command.name)}</h2>
      <p class="lead">${esc(command.about)}</p><div class="notice">This command has child commands. Choose one below.</div>
      <div class="child-list">${children.map((c) => `<button class="child-row" data-cmd="${esc(c.name)}"><span>aether ${esc(c.name)}</span>${icon('chevron', 14)}</button>`).join('')}</div>`;
    composer.querySelectorAll('[data-cmd]').forEach((button) => button.addEventListener('click', () => selectCommand(button.dataset.cmd)));
    return;
  }
  const positionals = (help.positionals || []).map((p, i) => {
    const key = `pos:${p.name}`;
    const input = `<div class="input-with-action"><input data-pos="${esc(p.name)}" id="field-${i}" placeholder="${esc(p.help || p.name)}" autocomplete="off">${PATH_HINT.test(`${p.name} ${p.help}`) ? browseButton(key) : ''}</div>`;
    return field(`${esc(p.name)}${p.required ? ' <b class="req">*</b>' : ''}`, input, p.help);
  }).join('');
  const valuedOptions = (help.options || []).filter((o) => o.value_name && !MANAGED_FLAGS.has(o.flag));
  const booleanOptions = (help.options || []).filter((o) => !o.value_name && !MANAGED_FLAGS.has(o.flag));
  const options = valuedOptions.map((o) => field(`${esc(o.flag)} <span class="dim">${esc(o.value_name)}</span>`,
    `<div class="input-with-action"><input data-opt="${esc(o.flag)}" placeholder="${esc(o.help || o.value_name)}" autocomplete="off">${PATH_HINT.test(`${o.value_name} ${o.help}`) ? browseButton(`opt:${o.flag}`) : ''}</div>`, o.default ? `default: ${o.default}` : '')).join('');
  const toggles = booleanOptions.map((o) => `<label class="toggle"><input type="checkbox" data-bool="${esc(o.flag)}"><span>${esc(o.flag)}</span><small>${esc(o.help)}</small></label>`).join('');
  const isApproval = command.kind === 'approval';
  composer.innerHTML = `
    <div class="eyebrow">Command · ${esc(command.kind)}</div>
    <h2>aether ${esc(command.name)}</h2>
    <p class="lead">${esc(command.about)}</p>
    <div class="usage mono">${esc(help.usage || `aether ${command.name}`)}</div>
    ${isApproval ? '<div class="notice warn">Requires explicit human authority. Supply the approver identity where the command asks for it.</div>' : ''}
    <form id="cmd-form">
      <div class="fields">${positionals}${options}</div>
      ${toggles ? `<details class="advanced"><summary>Flags</summary><div class="toggle-grid">${toggles}</div></details>` : ''}
      <label class="toggle auth-toggle" id="auth-toggle"><input type="checkbox" id="authorize"><span>I confirm this command may change project state</span></label>
      <div class="argv-preview"><span class="prompt">aether</span> <span id="argv-text"></span><span class="managed">--offline</span></div>
      <div class="form-actions">
        <button class="btn primary" id="cmd-run" type="submit">${icon('play', 14)} Run</button>
        <button class="btn ghost" id="cmd-copy" type="button">${icon('copy', 14)} Copy argv</button>
        <button class="btn ghost" id="cmd-pin" type="button">${icon('star', 14)} ${state.favorites.has(command.name) ? 'Unpin' : 'Pin'}</button>
      </div>
      <div id="cmd-message" class="dim small"></div>
    </form>`;

  const form = $('#cmd-form');
  form.addEventListener('input', updateArgvPreview);
  form.addEventListener('change', updateArgvPreview);
  form.addEventListener('submit', async (event) => { event.preventDefault(); await runComposer(command); });
  $('#cmd-copy').addEventListener('click', () => copyText(`aether ${formatArgv([...buildArgv(command), '--offline'])}`));
  $('#cmd-pin').addEventListener('click', () => {
    if (state.favorites.has(command.name)) state.favorites.delete(command.name);
    else state.favorites.add(command.name);
    localStorage.setItem('aether.favorites', JSON.stringify([...state.favorites]));
    renderCommandList();
    renderComposer(command, help);
  });
  form.querySelectorAll('[data-browse]').forEach((button) => button.addEventListener('click', () => openFiles(button.dataset.browse)));
  updateArgvPreview();
}

function buildArgv(command) {
  const argv = command.name.split(/\s+/);
  $$('[data-pos]').forEach((input) => { if (input.value.trim()) argv.push(input.value.trim()); });
  $$('[data-opt]').forEach((input) => {
    if (input.value.trim()) { argv.push(input.dataset.opt, input.value.trim()); }
  });
  $$('[data-bool]:checked').forEach((input) => argv.push(input.dataset.bool));
  return argv;
}

function updateArgvPreview() {
  const host = $('#argv-text');
  if (!host || !state.selected) return;
  const command = state.surface.commands.find((c) => c.name === state.selected);
  const argv = buildArgv(command);
  host.innerHTML = `${esc(formatArgv(argv))} ${argv.includes('--format') ? '' : '<span class="managed">--json</span>'}`;
  const authToggle = $('#auth-toggle');
  if (authToggle) authToggle.classList.toggle('hidden', !argv.includes('--yes') && command.kind !== 'approval');
}

async function runComposer(command) {
  const argv = buildArgv(command);
  if ((argv.includes('--yes') || command.kind === 'approval') && !$('#authorize')?.checked) {
    $('#cmd-message').textContent = 'Confirm authorization before running this command.';
    return;
  }
  const button = $('#cmd-run');
  button.disabled = true;
  button.textContent = 'Running…';
  $('#cmd-message').textContent = '';
  const result = await invokeAndRecord(argv, command.name);
  button.disabled = false;
  button.innerHTML = `${icon('play', 14)} Run`;
  if (!result.ok) {
    $('#cmd-message').textContent = result.data?.message || 'The command could not run.';
    return;
  }
  $('#cmd-message').textContent = '';
  showResult(result.data, command.name);
}

async function runArgv(argv, structured) {
  const result = await api('/exec', {
    method: 'POST',
    body: JSON.stringify({ argv, structured }),
  });
  if (!result.ok) {
    return { state: 'error', exit_code: result.status, duration_ms: 0, value: result.data, stderr: result.data?.message || '', command: argv.join(' ') };
  }
  ledger.append({
    argv: `aether ${formatArgv(argv)}`,
    command: argv.join(' '),
    state: result.data.state,
    exit_code: result.data.exit_code,
    duration_ms: result.data.duration_ms,
    workspace: 'terminal',
  });
  return result.data;
}

async function invokeAndRecord(argv, commandName) {
  const result = await postJSON('/exec', { argv });
  if (result.ok) {
    ledger.append({
      argv: `aether ${formatArgv(argv)}`,
      command: commandName,
      state: result.data.state,
      exit_code: result.data.exit_code,
      duration_ms: result.data.duration_ms,
      workspace: state.workspace || 'overview',
    });
  }
  return result;
}

async function runBuiltin(argv, term) {
  const name = argv[0];
  if (name === 'help') return terminalHelp();
  if (name === 'clear') { term.clear(); return ''; }
  if (name === 'history') return term.history.map((line, i) => `${String(i + 1).padStart(3)} ${line}`).join('\n') || 'No history.';
  if (name === 'theme') {
    setTheme(argv[1] === 'light' ? 'light' : 'dark');
    return `Theme: ${state.theme}`;
  }
  if (name === 'open') {
    const id = argv[1];
    if (state.surface.workspaces.some((w) => w.id === id)) {
      state.workspace = id;
      state.selected = null;
      setView('workspace');
      return `Opened workspace ${id}.`;
    }
    return `Unknown workspace: ${id || '(none)'}`;
  }
  return `Unknown built-in: ${name}`;
}

function showResult(result, name) {
  state.result = result;
  state.resultName = name;
  state.lastResult = { result, name };
  const inspector = $('#inspector');
  inspector.innerHTML = `<div class="inspector-head"><div><div class="eyebrow">Inspector</div><h2>aether ${esc(name)}</h2></div><button class="icon-btn" data-close-inspector>${icon('close')}</button></div>
    ${resultHtml(result, { commandName: name })}`;
  inspector.classList.add('open');
  if (result.state === 'blocked') toast('Blocked by the CLI — see the reason in the inspector.', 'bad');
}

function closeInspector() {
  $('#inspector')?.classList.remove('open');
  state.result = null;
}

function sessionHtml() {
  return `<div class="page-head"><div class="eyebrow">Tools · Local</div><h1>Session ledger</h1>
    <p class="lead">Every invocation from the workspace composer and the CLI terminal. Stored only in this browser.</p></div>
    <div class="session-toolbar"><button class="btn" id="session-export">${icon('download', 14)} Export JSON</button><button class="btn ghost" id="session-clear">${icon('refresh', 14)} Clear</button></div>
    <div id="session-list">${ledger.ledgerHtml(ledger.load())}</div>`;
}

function bindSession() {
  $('#session-export').addEventListener('click', () => ledger.exportLedger());
  $('#session-clear').addEventListener('click', () => { ledger.clear(); $('#session-list').innerHTML = ledger.ledgerHtml([]); });
  $('#session-list').addEventListener('click', async (event) => {
    const inspect = event.target.closest('[data-ledger-inspect]');
    const rerun = event.target.closest('[data-ledger-rerun]');
    const copy = event.target.closest('[data-copy-argv]');
    if (copy) {
      const entry = ledger.find(copy.dataset.copyArgv);
      if (entry) copyText(entry.argv || entry.command);
      return;
    }
    const entry = ledger.find((inspect || rerun)?.dataset.ledgerInspect
      || rerun?.dataset.ledgerRerun);
    if (!entry) return;
    if (inspect) showResult(entry, entry.command || 'command');
    if (rerun) {
      const parsed = tokenizeLine(entry.argv);
      const result = await invokeAndRecord(parsed, entry.command || parsed.join(' '));
      if (result.ok) showResult(result.data, entry.command || parsed.join(' '));
    }
  });
}

function tokenizeLine(text) {
  const raw = String(text || '').replace(/^aether\s+/, '');
  const parts = raw.match(/"([^"]*)"|'([^']*)'|(\S+)/g) || [];
  return parts.map((part) => part.replace(/^["']|["']$/g, ''));
}

function openFiles(key) {
  state.fileTarget = key;
  $('#file-overlay').classList.add('open');
  $('#file-search').value = '';
  loadFiles('');
  $('#file-search').focus();
}

async function loadFiles(query) {
  const host = $('#file-list');
  host.innerHTML = '<div class="empty small">Loading project files…</div>';
  const response = await api(`/files?q=${encodeURIComponent(query)}&limit=80`);
  if (!response.ok) {
    host.innerHTML = `<div class="empty small">${esc(response.data?.message || 'File discovery unavailable')}</div>`;
    return;
  }
  host.innerHTML = response.data.length ? response.data.map((entry) => `
    <button class="file-row" data-file="${esc(entry.path)}">${icon(entry.kind === 'dir' ? 'folder' : 'file', 15)}<span>${esc(entry.display)}</span>${icon('arrow', 13)}</button>
  `).join('') : '<div class="empty small">No matching files.</div>';
  host.querySelectorAll('[data-file]').forEach((button) => button.addEventListener('click', () => {
    const key = state.fileTarget;
    const input = key?.startsWith('pos:')
      ? $(`[data-pos="${CSS.escape(key.slice(4))}"]`)
      : $(`[data-opt="${CSS.escape(key.slice(4))}"]`);
    if (input) {
      input.value = button.dataset.file;
      input.dispatchEvent(new Event('input', { bubbles: true }));
    }
    $('#file-overlay').classList.remove('open');
  }));
}

function runAction(action) {
  if (action === 'terminal') { setView('terminal'); terminal.focus(); }
  else if (action === 'session') setView('session');
  else if (action === 'theme') toggleTheme();
  else if (action === 'help') $('#help-overlay').classList.add('open');
  else if (action === 'clear-session') { ledger.clear(); if (state.view === 'session') renderView(); }
  else if (action === 'copy-last') copyText(state.lastResult ? `aether ${state.lastResult.result.command || state.lastResult.name}` : '');
}

function toggleTheme() { setTheme(state.theme === 'night' ? 'light' : 'night'); }

function setTheme(theme) {
  state.theme = theme;
  document.documentElement.dataset.theme = theme;
  localStorage.setItem('aether.theme', theme);
  const button = $('#theme-btn');
  if (button) button.innerHTML = icon(theme === 'night' ? 'sun' : 'moon');
}

function copyText(text) {
  if (!text) return;
  navigator.clipboard?.writeText(text).then(() => toast('Copied to clipboard.'), () => toast('Copy failed.', 'bad'));
}

function toast(message, tone = '') {
  const host = $('#toasts');
  const item = document.createElement('div');
  item.className = `toast ${tone}`;
  item.textContent = message;
  host.appendChild(item);
  setTimeout(() => item.remove(), 3200);
}

document.addEventListener('keydown', (event) => {
  const key = event.key.toLowerCase();
  if ((event.metaKey || event.ctrlKey) && key === 'k') { event.preventDefault(); palette.toggle(); return; }
  if ((event.metaKey || event.ctrlKey) && key === 'j') { event.preventDefault(); setView('terminal'); terminal.focus(); return; }
  if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
    const run = $('#cmd-run');
    if (run) { event.preventDefault(); run.click(); }
    return;
  }
  if (event.key === 'Escape') {
    palette.close();
    $('#file-overlay')?.classList.remove('open');
    $('#help-overlay')?.classList.remove('open');
    closeInspector();
    return;
  }
  const typing = ['INPUT', 'TEXTAREA'].includes(document.activeElement?.tagName);
  if (key === '?' && !typing) { event.preventDefault(); $('#help-overlay').classList.toggle('open'); }
});

boot();
