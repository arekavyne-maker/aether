// SVG graph renderer. Pure DOM/SVG, no libraries, no inline styles (CSP).
import { esc } from '../format.js';
import { forceLayout, layeredLayout, radialLayout, fit } from './layout.js';

const DOMAIN = [
  [/Requirement|Need|Constraint|Assumption/, 'req'],
  [/Evidence|TestRun|Inspection|Measurement|Result|Finding/, 'evidence'],
  [/Geometry|Mesh|Part|Assembly|Material|Process|Tooling/, 'build'],
  [/Simulation|Model|Surrogate|Load|Environment|DigitalTwin/, 'sim'],
  [/Configuration|Baseline|Snapshot|Change|Variant/, 'config'],
  [/Decision|Policy|Identity|Agent|Release/, 'govern'],
];

export function domainOf(type) {
  for (const [pattern, domain] of DOMAIN) if (pattern.test(type)) return domain;
  return 'other';
}

const LAYOUTS = [
  ['force', 'Force'],
  ['layered', 'Layered'],
  ['radial', 'Radial'],
];

/** Render an interactive graph into `host`. Returns a controller. */
export function renderGraph(host, model, options = {}) {
  const onSelect = options.onSelect || (() => {});
  const state = {
    layout: 'force',
    query: '',
    type: '',
    selection: null,
    table: false,
    scale: 1,
    tx: 0,
    ty: 0,
    raw: null,
    screen: null,
  };
  host.innerHTML = `
    <div class="graph-toolbar">
      <div class="graph-modes">${LAYOUTS.map(([id, label]) => `<button class="chip ${id === state.layout ? 'active' : ''}" data-layout="${id}">${label}</button>`).join('')}</div>
      <label class="search-field graph-search">${iconSearch()}<input id="graph-query" placeholder="Find a node…" autocomplete="off"></label>
      <select id="graph-type" class="graph-select"><option value="">All node types</option></select>
      <button class="btn ghost" id="graph-fit">Fit</button>
      <button class="btn ghost" id="graph-table">Table</button>
    </div>
    <div class="graph-stage">
      <svg class="graph-svg" role="img" aria-label="Engineering graph">
        <defs><marker id="graph-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" class="graph-arrow"></path></marker></defs>
        <g class="graph-viewport"><g class="graph-edges"></g><g class="graph-nodes"></g></g>
      </svg>
      <div class="graph-empty"></div>
      <div class="graph-zoom"><button data-zoom="in" title="Zoom in">+</button><button data-zoom="out" title="Zoom out">−</button></div>
    </div>
    <div class="graph-table-host"></div>`;

  const svg = host.querySelector('.graph-svg');
  const viewport = host.querySelector('.graph-viewport');
  const edgeLayer = host.querySelector('.graph-edges');
  const nodeLayer = host.querySelector('.graph-nodes');
  const types = [...new Set(model.nodes.map((node) => node.type))].sort();
  const typeSelect = host.querySelector('#graph-type');
  typeSelect.innerHTML += types.map((type) => `<option value="${esc(type)}">${esc(type)}</option>`).join('');

  function compute(relayout = true) {
    if (!model.nodes.length) {
      host.querySelector('.graph-empty').innerHTML = '<div class="empty">No graph events yet. Run <code>aether run</code> in this project, or open a project that has graph events.</div>';
      return;
    }
    if (relayout || !state.raw) {
      state.raw = state.layout === 'layered'
        ? layeredLayout(model)
        : state.layout === 'radial'
          ? radialLayout(model, state.selection)
          : forceLayout(model);
    }
    const width = Math.max(480, host.clientWidth || 900);
    const height = Math.max(360, (host.querySelector('.graph-stage')?.clientHeight || 520) - 8);
    state.screen = fit(state.raw.positions, state.raw.bounds, width, height);
    applyTransform();
    paint();
  }

  function applyTransform() {
    viewport.setAttribute('transform', `translate(${state.tx} ${state.ty}) scale(${state.scale})`);
  }

  function visible(node) {
    if (state.type && node.type !== state.type) return false;
    if (!state.query) return true;
    const haystack = `${node.type} ${node.state} ${JSON.stringify(node.properties)}`.toLowerCase();
    return haystack.includes(state.query.toLowerCase());
  }

  function paint() {
    if (!state.screen) return;
    const positions = state.screen.positions;
    const visibleIds = new Set(model.nodes.filter(visible).map((node) => node.id));
    const neighbors = new Set();
    if (state.selection) {
      neighbors.add(state.selection);
      for (const edge of model.edges) {
        if (edge.source === state.selection) neighbors.add(edge.target);
        if (edge.target === state.selection) neighbors.add(edge.source);
      }
    }
    edgeLayer.innerHTML = model.edges.map((edge) => {
      const a = positions[edge.source];
      const b = positions[edge.target];
      if (!a || !b) return '';
      const hidden = !visibleIds.has(edge.source) || !visibleIds.has(edge.target);
      const related = !state.selection || edge.source === state.selection || edge.target === state.selection;
      const length = Math.hypot(b.x - a.x, b.y - a.y) || 1;
      const ux = ((b.x - a.x) / length) * 9;
      const uy = ((b.y - a.y) / length) * 9;
      const classes = ['g-edge', `edge-${esc(edge.type)}`, hidden ? 'hidden' : '', related ? '' : 'faded'].join(' ');
      return `<path class="${classes}" data-edge="${esc(edge.id)}" d="M ${(a.x + ux).toFixed(1)} ${(a.y + uy).toFixed(1)} L ${(b.x - ux).toFixed(1)} ${(b.y - uy).toFixed(1)}" marker-end="url(#graph-arrow)"><title>${esc(edge.type)}</title></path>`;
    }).join('');
    nodeLayer.innerHTML = model.nodes.map((node) => {
      const point = positions[node.id];
      if (!point) return '';
      const radius = 5 + Math.min(9, (model.degree[model.byId.get(node.id)] || 0) * 1.3);
      const classes = [
        'g-node', `domain-${domainOf(node.type)}`,
        visibleIds.has(node.id) ? '' : 'hidden',
        state.selection === node.id ? 'selected' : '',
        state.selection && !neighbors.has(node.id) ? 'faded' : '',
      ].join(' ');
      return `<g class="${classes}" data-id="${esc(node.id)}" transform="translate(${point.x.toFixed(1)} ${point.y.toFixed(1)})" tabindex="0" role="button" aria-label="${esc(node.type)}">
        <circle class="g-node-shape" r="${radius.toFixed(1)}"></circle>
        <text class="g-node-label" x="${(radius + 5).toFixed(1)}" y="4">${esc(node.type)}</text>
        <title>${esc(node.type)} · ${esc(node.state || 'no state')}</title>
      </g>`;
    }).join('');
    host.querySelector('.graph-empty').innerHTML = '';
  }

  function paintTable() {
    const hostTable = host.querySelector('.graph-table-host');
    if (!state.table) { hostTable.innerHTML = ''; return; }
    const rows = model.nodes.map((node) => `<tr>
      <td class="mono">${esc(node.type)}</td><td>${esc(node.state || '—')}</td>
      <td>${model.degree[model.byId.get(node.id)] || 0}</td>
      <td class="mono dim">${esc(node.id)}</td></tr>`).join('');
    hostTable.innerHTML = `<table class="data-table"><thead><tr><th>Type</th><th>State</th><th>Degree</th><th>Node id</th></tr></thead><tbody>${rows}</tbody></table>`;
  }

  host.addEventListener('click', (event) => {
    const layout = event.target.closest('[data-layout]');
    if (layout) {
      state.layout = layout.dataset.layout;
      host.querySelectorAll('[data-layout]').forEach((b) => b.classList.toggle('active', b === layout));
      state.raw = null;
      compute();
      return;
    }
    if (event.target.closest('#graph-fit')) { state.tx = 0; state.ty = 0; state.scale = 1; applyTransform(); return; }
    if (event.target.closest('#graph-table')) {
      state.table = !state.table;
      host.querySelector('.graph-stage').classList.toggle('hidden', state.table);
      paintTable();
      return;
    }
    const zoom = event.target.closest('[data-zoom]');
    if (zoom) {
      state.scale = Math.max(0.4, Math.min(4, state.scale * (zoom.dataset.zoom === 'in' ? 1.2 : 1 / 1.2)));
      applyTransform();
      return;
    }
    const node = event.target.closest('[data-id]');
    if (node) {
      state.selection = state.selection === node.dataset.id ? null : node.dataset.id;
      paint();
      onSelect(state.selection, model.nodes[model.byId.get(state.selection)] || null);
    }
  });
  host.addEventListener('input', (event) => {
    if (event.target.id === 'graph-query') { state.query = event.target.value.trim(); paint(); }
  });
  host.addEventListener('change', (event) => {
    if (event.target.id === 'graph-type') { state.type = event.target.value; paint(); }
  });
  host.addEventListener('keydown', (event) => {
    const node = event.target.closest('[data-id]');
    if (node && (event.key === 'Enter' || event.key === ' ')) {
      event.preventDefault();
      node.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    }
  });
  let dragging = null;
  svg.addEventListener('pointerdown', (event) => {
    if (event.target.closest('[data-id]')) return;
    dragging = { x: event.clientX, y: event.clientY, tx: state.tx, ty: state.ty };
    svg.setPointerCapture(event.pointerId);
  });
  svg.addEventListener('pointermove', (event) => {
    if (!dragging) return;
    state.tx = dragging.tx + event.clientX - dragging.x;
    state.ty = dragging.ty + event.clientY - dragging.y;
    applyTransform();
  });
  svg.addEventListener('pointerup', () => { dragging = null; });
  svg.addEventListener('wheel', (event) => {
    event.preventDefault();
    state.scale = Math.max(0.4, Math.min(4, state.scale * (event.deltaY < 0 ? 1.1 : 1 / 1.1)));
    applyTransform();
  }, { passive: false });

  compute();
  requestAnimationFrame(() => compute(false));
  return {
    compute,
    select(id) { state.selection = id; if (state.layout === 'radial') { state.raw = null; compute(); } else paint(); },
    destroy() {},
  };
}

function iconSearch() {
  return '<svg class="icon" width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="7" fill="none" stroke="currentColor" stroke-width="1.6"/><path d="M16 16l5 5" stroke="currentColor" stroke-width="1.6"/></svg>';
}
