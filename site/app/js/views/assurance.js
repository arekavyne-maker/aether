// Assurance view: coverage, gaps, chain integrity and evidence census.
import { esc } from '../format.js';
import { renderCoverage } from '../viz/coverage.js';
import { nodeCounts, coverageSummary } from '../data/atlas.js';

export function renderAssurance(host, context = {}) {
  const atlas = context.atlas || {};
  const chain = atlas.chain || {};
  const coverage = coverageSummary(atlas.coverage);
  const counts = nodeCounts(atlas.graph?.nodes);
  const evidence = counts.filter(([type]) => /Evidence|TestRun|Inspection|Result|Measurement/.test(type));

  host.innerHTML = `
    <div class="page-head"><div class="eyebrow">Assurance · evidence and verification</div><h1>What can be proven</h1>
      <p class="lead">Coverage figures are quoted from the CLI, never recomputed by the browser. Gap reason codes are shown verbatim.</p></div>
    <section class="assurance-strip">
      <div><span>Event chain</span><b class="pill ${chain.valid === false ? 'bad' : 'ok'}">${chain.valid === false ? 'invalid' : 'valid'}</b></div>
      <div><span>Events</span><b>${esc(chain.event_count ?? 0)}</b></div>
      <div><span>Coverage</span><b>${coverage ? `${coverage.percent}%` : '—'}</b></div>
      <div><span>Gaps</span><b>${coverage?.gaps ?? '—'}</b></div>
    </section>
    <h2 class="section-title">Verification coverage</h2>
    <div id="assurance-coverage"></div>
    <h2 class="section-title">Evidence census</h2>
    <div class="census">${evidence.length ? evidence.map(([type, count]) => `<div><b>${count}</b><span>${esc(type)}</span></div>`).join('') : '<div class="empty small">No evidence nodes in the current graph snapshot.</div>'}</div>
    <div class="notice">${esc(coverage?.claim || 'coverage only; this is not certification and does not imply airworthiness')}</div>`;

  renderCoverage(host.querySelector('#assurance-coverage'), atlas.coverage, {
    onRequirement: (id) => context.onRequirement?.(id),
  });
}
