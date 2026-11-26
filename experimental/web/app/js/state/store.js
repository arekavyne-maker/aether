// Minimal observable store. No framework, no external code.

export function createStore(initial = {}) {
  let state = { ...initial };
  const listeners = new Set();
  return {
    get() { return state; },
    set(patch) {
      state = { ...state, ...(typeof patch === 'function' ? patch(state) : patch) };
      for (const listener of listeners) listener(state);
      return state;
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
}

/** Stable, project-scoped key for persisted view preferences. */
export function prefKey(projectRoot, name) {
  return `aether:${projectRoot || 'local'}:${name}`;
}
