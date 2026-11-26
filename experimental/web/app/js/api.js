// Aether console — token handling and the local API client. No dependencies.
const TOKEN_KEY = 'aether.token';

function store(tokenValue) {
  try {
    sessionStorage.setItem(TOKEN_KEY, tokenValue);
    localStorage.setItem(TOKEN_KEY, tokenValue);
  } catch {
    // Storage can be disabled; the in-memory copy below still works.
  }
}

function read() {
  try {
    return sessionStorage.getItem(TOKEN_KEY) || localStorage.getItem(TOKEN_KEY);
  } catch {
    return null;
  }
}

/**
 * Accept a token from the URL fragment, then remember it for this origin.
 *
 * A link without a fragment therefore works after the first authorised
 * load — in this tab, in later tabs, and after a browser restart. The
 * fragment is scrubbed so the secret never stays in the address bar or
 * browser history.
 */
export function initToken() {
  const match = String(location.hash || '').match(/token=([0-9a-fA-F]{64})/);
  if (match) {
    const value = match[1].toLowerCase();
    store(value);
    history.replaceState(null, '', location.pathname + location.search);
    return value;
  }
  if (String(location.hash || '').startsWith('#token=')) {
    // An invalid or short token must not displace a working saved token.
    history.replaceState(null, '', location.pathname + location.search);
  }
  return read();
}

/** The token used for every request, shared across tabs of this origin. */
export function token() { return read(); }

/** Forget a rejected token so a fresh printed URL can be used. */
export function forgetToken() {
  try {
    sessionStorage.removeItem(TOKEN_KEY);
    localStorage.removeItem(TOKEN_KEY);
  } catch { /* storage unavailable */ }
}

/** Every request carries the bearer token. Errors are returned, never thrown. */
export async function api(path, options = {}) {
  const headers = { ...(options.headers || {}), Authorization: `Bearer ${token() || ''}` };
  if (options.body) headers['Content-Type'] = 'application/json';
  try {
    const response = await fetch(`/api${path}`, { ...options, headers });
    const text = await response.text();
    let data;
    try { data = JSON.parse(text); } catch { data = { error: 'non_json', message: text.slice(0, 500) }; }
    return { ok: response.ok, status: response.status, data };
  } catch (error) {
    return { ok: false, status: 0, data: { error: 'offline', message: String(error) } };
  }
}

export function getJSON(path) { return api(path); }
export function postJSON(path, body) {
  return api(path, { method: 'POST', body: JSON.stringify(body) });
}
