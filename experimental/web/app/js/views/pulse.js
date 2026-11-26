// Project-first home. Editorial chrome, dense data where it matters.
import { esc } from '../format.js';
import { roman } from '../surface.js';
import { icon, WORKSPACE_ICONS } from '../icons.js';
import { nodeCounts, coverageSummary } from '../data/atlas.js';

export function renderPulse(host, context = {}) {
  const atlas = context.atlas || {};
  const surface = context.surface || { workspaces: [], commands: [] };
  const graph = atlas.graph || {};
  const status = atlas.status || {};
  const chain = atlas.chain || {};
  const coverage = coverageSummary(atlas.coverage);
  const counts = nodeCounts(graph.nodes);
  const attention = [];
  if (!atlas.project?.initialized) attention.push({ tone: 'warn', title: 'No project at this root', text: 'Create or open a project to populate the graph. Everything else still works.', action: 'commands' });
  if (chain.valid === false) attention.push({ tone: 'bad', title: 'Event chain is invalid', text: 'The provenance chain did not verify. Treat the graph as untrusted.', action: 'atlas' });
  if (coverage && coverage.gaps) attention.push({ tone: 'warn', title: `${coverage.gaps} verification gap${coverage.gaps === 1 ? '' : 's'}`, text: 'Requirements without current verification evidence.', action: 'assurance' });
  if (!attention.length) attention.push({ tone: 'good', title: 'No attention items', text: 'The CLI reports no gaps or chain failures in this project.', action: 'atlas' });

  const tiles = surface.workspaces.map((workspace, index) => `
    <button class="workspace-tile" data-workspace="${esc(workspace.id)}">
      <div class="tile-top"><span class="roman">${roman(index)}</span><span class="tile-icon">${icon(WORKSPACE_ICONS[workspace.id] || 'overview', 18)}</span></div>
      <h3>${esc(workspace.title)}</h3><p>${esc(workspace.description)}</p>
      <div class="tile-foot"><span>${workspace.command_count} commands</span>${icon('arrow', 14)}</div>
    </button>`).join('');

  host.innerHTML = `
    <div class="overview">
      <section class="hero">
        <div class="eyebrow">The engineering control surface · ${esc(status.version || surface.version || '')}</div>
        <h1>Engineering</h1>
        <p class="hero-serif">Meets <em>Evidence.</em></p>
        <div class="thread-rule"><span></span><i>${icon('overview', 14)}</i><span></span></div>
        <p class="lead">${esc(atlas.project?.root || '')} · ${atlas.project?.initialized ? 'Project ready' : 'No project initialised'}. Every view below reads the canonical CLI; nothing is inferred.</p>
        <div class="hero-actions">
          <button class="btn primary" data-nav="atlas">${icon('overview', 15)} Open Atlas</button>
          <button class="btn" data-nav="terminal">${icon('terminal', 15)} CLI terminal</button>
          <button class="btn ghost" data-nav="commands">Browse ${surface.commands.length} commands</button>
        </div>
      </section>
      <section class="stat-band">
        <div><b>${esc(status.contracts ?? atlas.schema?.contractCount ?? '—')}</b><span>contracts</span></div>
        <div><b>${esc(graph.eventCount ?? 0)}</b><span>events</span></div>
        <div><b>${esc(graph.nodeCount ?? 0)}</b><span>graph nodes</span></div>
        <div><b>${esc(graph.edgeCount ?? 0)}</b><span>graph edges</span></div>
      </section>
      <h2 class="section-title"><span class="roman">I.</span> Requires attention</h2>
      <div class="attention-list">${attention.map((item) => `
        <button class="attention-row ${item.tone}" data-nav="${item.action}">
          <span class="pill ${item.tone}">${esc(item.tone)}</span>
          <span><b>${esc(item.title)}</b><small>${esc(item.text)}</small></span>${icon('arrow', 14)}
        </button>`).join('')}</div>
      <h2 class="section-title"><span class="roman">II.</span> Graph census</h2>
      <div class="census">${counts.length ? counts.slice(0, 8).map(([type, count]) => `<div><b>${count}</b><span>${esc(type)}</span></div>`).join('') : '<div class="empty small">No nodes in this project yet.</div>'}</div>
      <h2 class="section-title"><span class="roman">III.</span> Workspaces</h2><div class="workspace-map">${tiles}</div>
      <h2 class="section-title"><span class="roman">IV.</span> Recent thread</h2>${context.ledgerHtml || ''}
      <div class="notice">Certification claim: false. This console reports what the platform can prove, nothing more.</div>
    </div>`;

  host.querySelectorAll('[data-nav]').forEach((button) => button.addEventListener('click', () => context.onNavigate?.(button.dataset.nav)));
  host.querySelectorAll('[data-workspace]').forEach((button) => button.addEventListener('click', () => context.onWorkspace?.(button.dataset.workspace)));
}
