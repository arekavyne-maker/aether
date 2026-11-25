// Rendering helpers: escaping, syntax highlighting, JSON trees and the
// domain-aware summaries that make command output readable at a glance.

export function esc(value) {
  return String(value ?? '').replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

export function jsonHtml(value) {
  const text = JSON.stringify(value, null, 2) ?? 'null';
  return esc(text).replace(
    /("(?:\\.|[^"\\])*")(\s*:)?|\b(true|false)\b|\b(null)\b|(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)/g,
    (m, str, colon, bool, nul, num) => {
      if (str) return `<span class="${colon ? 'j-key' : 'j-str'}">${str}</span>${colon || ''}`;
      if (bool) return `<span class="j-bool">${bool}</span>`;
      if (nul) return `<span class="j-null">${nul}</span>`;
      return `<span class="j-num">${num}</span>`;
    },
  );
}

export function stateClass(value) {
  if (value === 'ok') return 'ok';
  if (value === 'blocked') return 'blocked';
  if (value === 'usage_error') return 'warn';
  return 'bad';
}

export function statePill(result) {
  const state = result?.state ?? 'error';
  return `<span class="pill ${stateClass(state)}">${esc(state)}</span>`;
}

export function fmtDuration(ms) {
  if (ms >= 1000) return `${(ms / 1000).toFixed(2)} s`;
  return `${ms ?? 0} ms`;
}

export function fmtTime(ts) {
  try {
    return new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  } catch {
    return String(ts ?? '');
  }
}

function primitive(value) {
  if (value === null) return '<span class="j-null">null</span>';
  if (typeof value === 'boolean') return `<span class="j-bool">${value}</span>`;
  if (typeof value === 'number') return `<span class="j-num">${esc(value)}</span>`;
  return `<span class="j-str">${esc(JSON.stringify(value))}</span>`;
}

function node(key, value, depth) {
  const label = key === null ? '' : `<span class="tree-key">${esc(key)}</span>`;
  if (value === null || typeof value !== 'object') {
    return `<div class="tree-leaf">${label}${label ? '<span class="tree-colon">: </span>' : ''}${primitive(value)}</div>`;
  }
  const entries = Array.isArray(value)
    ? value.map((item, index) => [String(index), item])
    : Object.entries(value);
  const kind = Array.isArray(value) ? `[${entries.length}]` : `{${entries.length}}`;
  const open = depth < 2 ? ' open' : '';
  const children = depth >= 6
    ? '<div class="tree-leaf dim">…</div>'
    : entries.map(([k, v]) => node(k, v, depth + 1)).join('');
  return `<details class="tree"${open}><summary>${label}<span class="tree-kind">${kind}</span></summary><div class="tree-body">${children}</div></details>`;
}

export function jsonTree(value) {
  return `<div class="json-tree">${node(null, value, 0)}</div>`;
}

function rows(items) {
  return `<div class="kv">${items.map(([k, v]) => `<div class="kv-row"><span>${esc(k)}</span><b>${esc(v)}</b></div>`).join('')}</div>`;
}

function table(headers, body) {
  return `<div class="table-wrap"><table><thead><tr>${headers.map((h) => `<th>${esc(h)}</th>`).join('')}</tr></thead><tbody>${body}</tbody></table></div>`;
}

/** Domain-aware summary. Returns '' when no renderer applies. */
export function smartSummary(name, value) {
  if (!value || typeof value !== 'object') return '';
  const n = String(name || '');

  if (n === 'status') {
    const graph = value.graph || {};
    const caps = Object.entries(value)
      .filter(([k, v]) => typeof v === 'string' && k !== 'phase')
      .map(([k, v]) => [k.replaceAll('_', ' '), v]);
    return `<div class="summary-lead"><span class="big-version">${esc(value.version || '—')}</span><span class="phase">${esc(value.phase || '')}</span></div>
      ${value.certification_claim ? '<div class="notice bad">Certification claim present — verify before relying on this output.</div>' : '<div class="notice">Certification claim: false. This interface reports what the platform can prove, nothing more.</div>'}
      ${rows([
        ['contracts', value.contracts ?? '—'],
        ['graph nodes / edges / events', `${graph.nodes ?? 0} / ${graph.edges ?? 0} / ${graph.events ?? 0}`],
        ['chain head', graph.chain_head ?? 'none'],
      ])}
      <h4>Capabilities</h4>${rows(caps)}`;
  }

  if (n === 'graph chain') {
    const errors = Array.isArray(value.errors) ? value.errors : [];
    return `<div class="notice ${value.valid ? 'good' : 'bad'}">${value.valid ? 'Chain valid' : 'Chain invalid'}</div>
      ${rows([['events', value.event_count ?? 0], ['nodes', value.node_count ?? 0], ['edges', value.edge_count ?? 0], ['head', value.head ?? 'none']])}
      ${errors.length ? `<h4>Errors</h4><ul class="bullets">${errors.map((e) => `<li>${esc(e)}</li>`).join('')}</ul>` : ''}`;
  }

  if (n === 'vv coverage' || n === 'validation report' || n === 'requirement report') {
    const total = Number(value.requirements_total ?? 0);
    const verified = Number(value.requirements_verified ?? 0);
    const pct = total ? Math.round((verified / total) * 100) : 0;
    const bucket = Math.min(100, Math.max(0, Math.floor(pct / 10) * 10));
    const gaps = Array.isArray(value.gaps) ? value.gaps : [];
    const gapRows = gaps.map((g) => `<tr><td>${esc(g.requirement_id ?? '—')}</td><td><code>${esc(g.reason_code ?? '—')}</code></td><td>${esc(g.criticality ?? '—')}</td><td>${esc(g.reason ?? g.requirement_text ?? '')}</td></tr>`).join('');
    return `<div class="coverage"><div class="coverage-num">${pct}<small>%</small></div><div><div class="coverage-label">${verified} of ${total} requirements verified</div><div class="bar"><span class="fill p${bucket}${pct === 100 ? ' full' : ''}"></span></div></div></div>
      ${value.claim ? `<div class="notice">${esc(value.claim)}</div>` : ''}
      ${gaps.length ? `<h4>Reason-coded gaps</h4>${table(['Requirement', 'Reason code', 'Criticality', 'Detail'], gapRows)}` : '<div class="notice good">No gaps reported.</div>'}`;
  }

  if (n === 'vv decisions') {
    const decisions = Array.isArray(value.decisions) ? value.decisions : [];
    if (!decisions.length) return '';
    return `<h4>Review decisions</h4>${table(['Decision', 'Role', 'Verdict'], decisions.map((d) => `<tr><td>${esc(d.decision_id ?? d.id ?? '—')}</td><td>${esc(d.role ?? '—')}</td><td>${esc(d.verdict ?? d.decision ?? '—')}</td></tr>`).join(''))}`;
  }

  if (n === 'schema list') {
    const contracts = Array.isArray(value.contracts) ? value.contracts : [];
    return `${rows([['contract packages', value.contract_count ?? contracts.length], ['graph node types', value.node_type_count ?? '—'], ['graph edge types', value.edge_type_count ?? '—']])}
      <h4>Contract packages</h4><div class="chips">${contracts.slice(0, 24).map((c) => `<span class="chip">${esc(c.id ?? c.contract_id ?? c)}</span>`).join('')}</div>`;
  }

  if (n === 'doctor') {
    const checks = Array.isArray(value.checks) ? value.checks : [];
    if (!checks.length) return '';
    return `<div class="notice">${esc(value.aether_version || '')} · ${checks.filter((c) => c.state === 'ok').length}/${checks.length} checks ok</div>
      ${table(['Check', 'State', 'Detail'], checks.map((c) => `<tr><td>${esc(c.name ?? '')}</td><td><span class="pill ${c.state === 'ok' ? 'ok' : (c.optional ? 'blocked' : 'bad')}">${esc(c.state ?? '')}${c.optional ? ' · optional' : ''}</span></td><td>${esc(c.detail ?? '')}</td></tr>`).join(''))}`;
  }

  if (n === 'metrics') {
    const metrics = value.metrics || {};
    return `<div class="notice">${esc(value.claim || '')}</div>${table(['Metric', 'Kind', 'Value'], Object.values(metrics).map((m) => `<tr><td>${esc(m.name ?? '')}</td><td>${esc(m.kind ?? '')}</td><td>${esc(m.value ?? '')}</td></tr>`).join(''))}`;
  }

  if (n === 'benchmark') {
    const benchmarks = Array.isArray(value.benchmarks) ? value.benchmarks : [];
    if (!benchmarks.length) return '';
    return `<div class="notice ${value.all_passed ? 'good' : 'bad'}">${value.all_passed ? 'All benchmarks passed' : 'One or more benchmarks failed'}</div>
      ${table(['Benchmark', 'State', 'Runtime'], benchmarks.map((b) => `<tr><td>${esc(b.id ?? '')} · ${esc(b.title ?? '')}</td><td><span class="pill ${b.skipped ? 'blocked' : (b.passed ? 'ok' : 'bad')}">${b.skipped ? 'skipped' : (b.passed ? 'passed' : 'failed')}</span></td><td>${esc(fmtDuration(b.runtime_ms ?? 0))}</td></tr>`).join(''))}`;
  }

  if (n === 'policy explain') {
    return `<div class="notice ${value.allow ? 'good' : 'bad'}">${esc(value.decision ?? (value.allow ? 'allow' : 'deny'))} — ${esc(value.reason ?? '')}</div>
      ${rows([['action', value.action ?? '—'], ['engine', value.engine ?? '—'], ['subject', typeof value.subject === 'object' ? JSON.stringify(value.subject) : value.subject ?? '—']])}`;
  }

  if (n === 'discover commands') {
    const commands = Array.isArray(value.commands) ? value.commands : [];
    return `<div class="notice">${commands.length} discovered commands · ${esc(value.command_count ?? '')} advertised</div><div class="chips">${commands.slice(0, 60).map((c) => `<span class="chip">${esc(c.name)}</span>`).join('')}</div>`;
  }

  if (typeof value.gates !== 'undefined' || typeof value.passed !== 'undefined') {
    return rows(Object.entries(value).filter(([, v]) => typeof v !== 'object').map(([k, v]) => [k.replaceAll('_', ' '), v]));
  }

  return '';
}

/** Full result panel used by the inspector and terminal entries. */
export function resultHtml(result, options = {}) {
  if (!result) return '';
  const name = options.commandName || result.command || '';
  const managed = result.command ? `<div class="result-argv"><span class="prompt">aether</span> ${esc(result.command)}</div>` : '';
  const summary = smartSummary(name, result.value);
  const hasSummary = Boolean(summary);
  return `<div class="result ${stateClass(result.state)}">
    <div class="result-head">
      ${statePill(result)}
      <span class="pill">exit ${esc(result.exit_code)}</span>
      <span class="pill">${esc(fmtDuration(result.duration_ms))}</span>
      ${options.ledgerId ? `<button class="icon-btn" data-copy-argv="${esc(options.ledgerId)}" title="Copy argv">${iconCopy()}</button>` : ''}
    </div>
    ${managed}
    <details class="result-section"${hasSummary ? ' open' : ''}>
      <summary>${hasSummary ? 'Summary' : 'Output'}</summary>
      <div class="result-body">${hasSummary ? summary : (result.text ? `<pre class="json">${esc(result.text)}</pre>` : jsonTree(result.value))}</div>
    </details>
    <details class="result-section"><summary>JSON</summary><div class="result-body"><pre class="json">${jsonHtml(result.value)}</pre></div></details>
    ${result.stderr ? `<details class="result-section"><summary>Logs (stderr)</summary><div class="result-body"><pre class="stderr">${esc(result.stderr)}</pre></div></details>` : ''}
  </div>`;
}

function iconCopy() {
  return '<svg class="icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"/></svg>';
}

export function downloadJson(data, filename) {
  const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
}
