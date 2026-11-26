// Coverage and gap rendering from the CLI's validation report.
import { esc } from '../format.js';

const CRITICALITY = ['catastrophic', 'hazardous', 'major', 'minor', 'no_effect'];

/** Render a coverage band and the exact gap rows reported by the CLI. */
export function renderCoverage(host, coverage, options = {}) {
  const onRequirement = options.onRequirement || (() => {});
  if (!coverage) {
    host.innerHTML = '<div class="empty">No project verification report is available. Initialise a project and add requirements, or run <code>vv coverage &lt;file&gt;</code> from the terminal.</div>';
    return;
  }
  const total = Number(coverage.requirements_total) || 0;
  const verified = Number(coverage.requirements_verified) || 0;
  const gaps = Array.isArray(coverage.gaps) ? coverage.gaps : [];
  const uncased = gaps.filter((gap) => /case/i.test(`${gap.reason_code || ''} ${gap.reason || ''}`)).length;
  const percent = total ? Math.round((verified / total) * 100) : 0;
  const bucket = Math.round(percent / 5) * 5;
  const rows = gaps.map((gap) => {
    const rank = CRITICALITY.indexOf(String(gap.criticality || '').toLowerCase());
    return `
      <tr data-requirement="${esc(gap.requirement_id || '')}">
        <td class="mono">${esc(gap.requirement_id || '—')}</td>
        <td>${esc(gap.requirement_text || gap.requirement_id || '—')}</td>
        <td><span class="pill ${rank <= 1 ? 'blocked' : ''}">${esc(gap.criticality || 'unknown')}</span></td>
        <td class="mono">${esc(gap.reason_code || 'NO_EDGE')}</td>
        <td>${esc(gap.reason || gap.status || '—')}</td>
      </tr>`;
  }).join('');

  host.innerHTML = `
    <div class="coverage-band">
      <div class="coverage-ring p${Math.min(100, Math.max(0, bucket))}"><b>${percent}%</b><small>verified</small></div>
      <div class="coverage-figures">
        <div><b>${total}</b><span>requirements</span></div>
        <div><b>${verified}</b><span>verified</span></div>
        <div><b>${uncased}</b><span>uncased</span></div>
        <div><b>${gaps.length}</b><span>gaps</span></div>
      </div>
      <p class="claim">${esc(coverage.claim || 'coverage only; this is not certification')}</p>
    </div>
    ${gaps.length ? `<table class="data-table gaps-table"><thead><tr><th>Requirement</th><th>Text</th><th>Criticality</th><th>Reason code</th><th>Reason</th></tr></thead><tbody>${rows}</tbody></table>` : '<div class="notice good">No gaps reported by the CLI.</div>'}`;

  host.querySelectorAll('[data-requirement]').forEach((row) => {
    row.addEventListener('click', () => onRequirement(row.dataset.requirement));
  });
}
