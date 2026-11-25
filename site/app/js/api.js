// Aether console — token handling and the local API client. No dependencies.
const TOKEN_KEY = 'aether.token';

/** Move the fragment token into sessionStorage and scrub it from the URL. */
export function initToken() {
  const match = location.hash.match(/token=([0-9a-fA-F]{64})/);
  if (match) {
    sessionStorage.setItem(TOKEN_KEY, match[1].toLowerCase());
    history.replaceState(null, '', location.pathname + location.search);
  }
  return sessionStorage.getItem(TOKEN_KEY);
}

export function token() {
  return sessionStorage.getItem(TOKEN_KEY);
}

/** Every request carries the bearer token. Errors are returned, never thrown. */
export async function api(path, options = {}) {
  const headers = { ...(options.headers || {}), Authorization: `Bearer ${token()}` };
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
