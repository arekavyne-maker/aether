// Atlas: one linked surface for graph, schema, coverage and command map.
import { esc } from '../format.js';
import { postJSON } from '../api.js';
import { loadGraph } from '../data/atlas.js';
import { graphModel, normalize } from '../data/atlas.js';
import { diffGraph } from '../viz/layout.js';
import { renderGraph } from '../viz/graph.js';
import { renderCoverage } from '../viz/coverage.js';
import { treemap } from '../viz/treemap.js';

const TABS = [['graph', 'Graph'], ['schema', 'Schema'], ['coverage', 'Coverage'], ['commands', 'Commands']];

export function renderAtlas(host, context = {}) {
  const atlas = context.atlas || normalize(null);
  const tab = TABS.some(([id]) => id === context.tab) ? context.tab : 'graph';
  const surface = context.surface || { commands: [] };
  let current = atlas;
  let controller = null;
  let compare = null;

  host.innerHTML = `
    <div class="page-head atlas-head">
      <div class="eyebrow">Atlas · ${esc(atlas.project?.root || 'local')}</div>
      <h1>The engineering graph</h1>
      <p class="lead">Every node and edge below is read from <code>aether graph as-of</code>. Nothing is inferred or invented.</p>
    </div>
    <div class="atlas-tabs">${TABS.map(([id, label]) => `<button class="atlas-tab ${id === tab ? 'active' : ''}" data-tab="${id}">${label}</button>`).join('')}</div>
    <section id="atlas-panel"></section>`;

  const panel = host.querySelector('#atlas-panel');

  function lineage() {
    const sources = current.sources || [];
    return `<details class="lineage"><summary>${sources.length} CLI sources · ${current.timing?.durationMs || 0} ms</summary><div class="lineage-list">${sources.map((source) => `
      <div><code>${esc(source.command)}</code><span class="pill ${source.ok === false ? 'bad' : source.cached ? 'read' : 'ok'}">${source.ok === false ? `exit ${source.exit_code}` : source.cached ? 'cached' : 'live'}</span><small>${Number(source.duration_ms) || 0} ms</small></div>`).join('')}</div></details>`;
  }

  function renderNodeDetail(node) {
    const detail = host.querySelector('#atlas-detail');
    if (!detail) return;
    if (!node) {
      detail.innerHTML = '<div class="empty small">Select a node to inspect its properties and relationships.</div>';
      return;
    }
    const model = graphModel(current.graph);
    const related = model.edges.filter((edge) => edge.source === node.id || edge.target === node.id);
    const properties = Object.entries(node.properties || {}).map(([key, value]) => `
      <tr><td class="mono">${esc(key)}</td><td>${esc(typeof value === 'object' ? JSON.stringify(value) : String(value))}</td></tr>`).join('');
    detail.innerHTML = `
      <div class="eyebrow">Node</div><h3>${esc(node.type)}</h3>
      <div class="detail-meta"><span class="pill ${node.state === 'released' || node.state === 'succeeded' ? 'ok' : ''}">${esc(node.state || 'no state')}</span>${node.immutable ? '<span class="pill">immutable</span>' : ''}</div>
      <table class="data-table"><tbody>${properties || '<tr><td colspan="2" class="dim">No properties</td></tr>'}</tbody></table>
      <h4>Relationships</h4>
      ${related.length ? related.map((edge) => `<div class="edge-line"><code>${esc(edge.type)}</code><span>${esc(edge.source === node.id ? '→ ' : '← ')}${esc(edge.source === node.id ? edge.target : edge.source)}</span></div>`).join('') : '<div class="empty small">No edges.</div>'}`;
  }

  async function renderGraphTab() {
    panel.innerHTML = `
      <div class="timeline-bar">
        <button class="btn ghost" id="compare-offset">Compare</button>
        <label>Event <b id="offset-label">${current.graph.offset}</b> / ${current.graph.eventCount}</label>
        <input type="range" id="offset-range" min="0" max="${Math.max(0, current.graph.eventCount)}" value="${current.graph.offset}">
        <span class="pill ${current.chain?.valid === false ? 'bad' : 'ok'}">${current.chain?.valid === false ? 'chain invalid' : 'chain valid'}</span>
      </div>
      <div id="compare-note"></div>
      <div class="atlas-split">
        <section class="atlas-canvas" id="atlas-graph"></section>
        <aside class="atlas-detail" id="atlas-detail"><div class="empty small">Select a node to inspect its properties and relationships.</div></aside>
      </div>
      ${lineage()}`;
    const model = graphModel(current.graph);
    controller = renderGraph(host.querySelector('#atlas-graph'), model, {
      onSelect: (_id, node) => renderNodeDetail(node),
    });
    host.querySelector('#offset-range').addEventListener('change', async (event) => {
      const offset = Number(event.target.value);
      const response = await loadGraph(offset);
      if (!response.ok) return;
      const loaded = normalize({ graph: response.data.graph });
      current = { ...current, graph: loaded.graph };
      host.querySelector('#offset-label').textContent = String(offset);
      controller = renderGraph(host.querySelector('#atlas-graph'), graphModel(current.graph), {
        onSelect: (_id, node) => renderNodeDetail(node),
      });
      host.querySelector('#atlas-detail').innerHTML = '<div class="empty small">Select a node to inspect its properties and relationships.</div>';
    });
    host.querySelector('#compare-offset').addEventListener('click', async () => {
      const beforeOffset = Math.max(0, current.graph.offset - 1);
      const response = await loadGraph(beforeOffset);
      if (!response.ok) return;
      const diff = diffGraph(response.data.graph, current.graph);
      const note = host.querySelector('#compare-note');
      const parts = [
        `${diff.addedNodes.length} node${diff.addedNodes.length === 1 ? '' : 's'} added`,
        `${diff.removedNodes.length} removed`,
        `${diff.addedEdges.length} edge${diff.addedEdges.length === 1 ? '' : 's'} added`,
        `${diff.removedEdges.length} removed`,
      ];
      note.innerHTML = `<div class="notice">Compared offset ${beforeOffset} → ${current.graph.offset}: ${parts.join(' · ')}.</div>`;
      compare = diff;
    });
  }

  function renderSchemaTab() {
    const nodeTypes = current.schema.nodeTypes;
    const edgeTypes = current.schema.edgeTypes;
    panel.innerHTML = `
      <div class="schema-toolbar"><label class="search-field"><input id="schema-query" placeholder="Filter type names…" autocomplete="off"></label><span class="pill">${nodeTypes.length} node types · ${edgeTypes.length} edge types · ${current.schema.contractCount} contracts</span></div>
      <div class="schema-grid">
        <section><h3>Node types</h3><div id="schema-nodes" class="schema-list">${nodeTypes.map((type) => `<button class="schema-row" data-type="${esc(type)}">${esc(type)}</button>`).join('')}</div></section>
        <section><h3>Edge types</h3><div id="schema-edges" class="schema-list">${edgeTypes.map((type) => `<button class="schema-row" data-type="${esc(type)}">${esc(type)}</button>`).join('')}</div></section>
        <aside id="schema-detail"><div class="empty small">Select a type to read its contract.</div></aside>
      </div>${lineage()}`;
    panel.querySelector('#schema-query').addEventListener('input', (event) => {
      const query = event.target.value.trim().toLowerCase();
      panel.querySelectorAll('.schema-row').forEach((row) => {
        row.classList.toggle('hidden', !row.dataset.type.toLowerCase().includes(query));
      });
    });
    panel.querySelectorAll('[data-type]').forEach((row) => row.addEventListener('click', async () => {
      const response = await postJSON('/exec', { argv: ['schema', 'show', row.dataset.type] });
      const detail = panel.querySelector('#schema-detail');
      if (!response.ok || response.data.state !== 'ok') {
        detail.innerHTML = `<div class="notice bad">${esc(response.data?.message || 'Definition unavailable')}</div>`;
        return;
      }
      detail.innerHTML = `<div class="eyebrow">${esc(row.dataset.type)}</div><pre class="json">${esc(JSON.stringify(response.data.value, null, 2))}</pre>`;
    }));
  }

  function renderCoverageTab() {
    panel.innerHTML = '<div class="coverage-host"></div>' + lineage();
    renderCoverage(panel.querySelector('.coverage-host'), current.coverage, {
      onRequirement: (id) => {
        const model = graphModel(current.graph);
        const node = model.nodes.find((candidate) => candidate.id === id);
        if (node) {
          context.onTab?.('graph');
        }
      },
    });
  }

  function renderCommandsTab() {
    const families = new Map();
    for (const command of surface.commands || []) {
      families.set(command.family, (families.get(command.family) || 0) + 1);
    }
    const rects = treemap([...families.entries()].map(([family, value]) => ({ family, value })), 960, 520);
    const blocks = rects.map((rect) => {
      const size = Math.min(rect.w, rect.h);
      const label = size > 46 ? esc(rect.family) : '';
      return `<g class="map-tile" data-family="${esc(rect.family)}" tabindex="0"><rect x="${rect.x.toFixed(1)}" y="${rect.y.toFixed(1)}" width="${rect.w.toFixed(1)}" height="${rect.h.toFixed(1)}"></rect>${label ? `<text x="${(rect.x + rect.w / 2).toFixed(1)}" y="${(rect.y + rect.h / 2).toFixed(1)}">${label}</text>` : ''}<title>${esc(rect.family)} · ${rect.value} commands</title></g>`;
    }).join('');
    panel.innerHTML = `
      <div class="command-map-head"><div class="eyebrow">Command surface</div><h2>${surface.commands.length} commands across ${families.size} families</h2><p>Click a tile to open that command family in the workspace.</p></div>
      <svg class="command-map" viewBox="0 0 960 520" role="img" aria-label="Command family treemap">${blocks}</svg>${lineage()}`;
    panel.querySelectorAll('[data-family]').forEach((tile) => tile.addEventListener('click', () => context.onFamily?.(tile.dataset.family)));
  }

  host.querySelectorAll('[data-tab]').forEach((button) => button.addEventListener('click', () => {
    context.onTab?.(button.dataset.tab);
  }));

  if (tab === 'schema') renderSchemaTab();
  else if (tab === 'coverage') renderCoverageTab();
  else if (tab === 'commands') renderCommandsTab();
  else renderGraphTab();

  return { compare: () => compare };
}
