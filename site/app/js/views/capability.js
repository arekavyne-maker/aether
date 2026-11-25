// Build and Govern views: honest capability summaries from doctor + graph.
import { esc } from '../format.js';
import { nodeCounts } from '../data/atlas.js';

const FILTERS = {
  build: /Geometry|Mesh|Part|Assembly|Material|Process|Tooling|Simulation|Load|Environment|Test/,
  govern: /Decision|Policy|Identity|Agent|Release|Evidence|Configuration|Baseline/,
};

export function renderCapability(host, context = {}) {
  const kind = context.kind === 'govern' ? 'govern' : 'build';
  const atlas = context.atlas || {};
  const health = context.health || {};
  const checks = Array.isArray(health.checks) ? health.checks : [];
  const counts = nodeCounts(atlas.graph?.nodes).filter(([type]) => FILTERS[kind].test(type));
  const surface = context.surface || { commands: [] };
  const family = kind === 'build' ? /geometry|mesh|simulate|design|manufacture|inspect|test|loads|material|units/ : /policy|agent|identity|release|evidence|classify|token|config|validation|fidelity/;
  const commands = surface.commands.filter((command) => family.test(command.name));

  host.innerHTML = `
    <div class="page-head"><div class="eyebrow">${kind === 'build' ? 'Build · design and manufacturing' : 'Govern · assurance and policy'}</div>
      <h1>${kind === 'build' ? 'From design to as-built' : 'Authority and evidence'}</h1>
      <p class="lead">Health checks come from <code>aether doctor</code>. Only checks that actually ran are shown as passing.</p></div>
    <div class="health-grid">${checks.map((check) => `
      <div class="health-row"><span class="pill ${check.state === 'ok' ? 'ok' : check.optional ? 'warn' : 'bad'}">${esc(check.state)}</span>
        <b>${esc(check.name)}</b><small>${esc(check.detail || '')}</small>${check.optional ? '<span class="dim">optional</span>' : ''}</div>`).join('') || '<div class="empty small">Run <code>aether doctor</code> to populate health checks.</div>'}</div>
    <h2 class="section-title">Graph census</h2>
    <div class="census">${counts.length ? counts.slice(0, 10).map(([type, count]) => `<div><b>${count}</b><span>${esc(type)}</span></div>`).join('') : '<div class="empty small">No nodes of this domain in the current snapshot.</div>'}</div>
    <h2 class="section-title">Related commands</h2>
    <div class="cap-grid">${commands.slice(0, 24).map((command) => `<button class="cap link-cap" data-cmd="${esc(command.name)}"><span>${esc(command.family)}</span><b>aether ${esc(command.name)}</b><small>${esc(command.about)}</small></button>`).join('')}</div>`;

  host.querySelectorAll('[data-cmd]').forEach((button) => button.addEventListener('click', () => context.onCommand?.(button.dataset.cmd)));
}
