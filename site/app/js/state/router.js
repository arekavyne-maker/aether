// Hash router for the console. Token fragments are scrubbed on boot, so
// every remaining hash is a route: #/atlas/graph, #/commands/graph, etc.

const VIEWS = new Set(['pulse', 'atlas', 'assurance', 'build', 'govern', 'commands', 'terminal', 'session']);

/** Parse a location hash into a route object. */
export function parse(hash) {
  const raw = String(hash || '').replace(/^#\/?/, '');
  const parts = raw.split('/').filter(Boolean);
  const view = VIEWS.has(parts[0]) ? parts[0] : 'pulse';
  return { view, tab: parts[1] || '', workspace: view === 'commands' ? parts[1] || '' : '' };
}

/** Serialize a route object into a location hash. */
export function href(route) {
  const parts = [route.view];
  if (route.view === 'commands') {
    if (route.workspace) parts.push(route.workspace);
  } else if (route.tab) {
    parts.push(route.tab);
  }
  return `#/${parts.join('/')}`;
}

/** Navigate, updating location.hash without a page reload. */
export function go(route) {
  const next = href(route);
  if (location.hash !== next) location.hash = next;
}

/** Observe route changes. */
export function onChange(callback) {
  const handler = () => callback(parse(location.hash));
  window.addEventListener('hashchange', handler);
  return () => window.removeEventListener('hashchange', handler);
}
