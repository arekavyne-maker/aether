// Command surface helpers: grouping, filtering and fuzzy matching.

const ROMAN = ['I', 'II', 'III', 'IV', 'V', 'VI', 'VII', 'VIII', 'IX', 'X', 'XI', 'XII'];

export function roman(index) {
  return ROMAN[index] ?? String(index + 1);
}

/** Group commands by CLI family within a workspace. */
export function groupCommands(surface, workspaceId, query = '', kind = 'all', favorites = new Set()) {
  const terms = query.toLowerCase().trim().split(/\s+/).filter(Boolean);
  const matches = (command) => {
    if (workspaceId && command.workspace !== workspaceId) return false;
    if (kind !== 'all' && command.kind !== kind) return false;
    if (!terms.length) return true;
    const haystack = `${command.name} ${command.about} ${command.family}`.toLowerCase();
    return terms.every((term) => haystack.includes(term));
  };
  const commands = surface.commands.filter(matches);
  const pinned = commands.filter((c) => favorites.has(c.name));
  const families = new Map();
  for (const command of commands) {
    if (!families.has(command.family)) families.set(command.family, []);
    families.get(command.family).push(command);
  }
  const groups = [...families.entries()].map(([family, items]) => ({
    family,
    commands: items.sort((a, b) => a.name.localeCompare(b.name)),
  }));
  groups.sort((a, b) => a.family.localeCompare(b.family));
  return { pinned, groups, total: commands.length };
}

/** Simple subsequence score: lower is better, -1 means no match. */
export function fuzzyScore(query, text) {
  const q = query.toLowerCase().trim();
  if (!q) return 0;
  const t = text.toLowerCase();
  const direct = t.indexOf(q);
  if (direct >= 0) return direct;
  let at = 0;
  let score = 0;
  for (const char of q) {
    const found = t.indexOf(char, at);
    if (found < 0) return -1;
    score += found - at;
    at = found + 1;
  }
  return score + 40;
}

/** Palette candidates: commands, workspaces and actions. */
export function paletteCandidates(surface, query, recent = []) {
  const items = [];
  for (const name of recent.slice(0, 6)) {
    const command = surface.commands.find((c) => c.name === name);
    if (command) items.push({ ...command, type: 'recent' });
  }
  for (const command of surface.commands) items.push({ ...command, type: 'command' });
  for (const workspace of surface.workspaces) {
    items.push({
      name: workspace.title,
      about: workspace.description,
      type: 'workspace',
      target: workspace.id,
    });
  }
  const scored = items.map((item) => ({ item, score: fuzzyScore(query, `${item.name} ${item.about || ''}`) }))
    .filter((entry) => entry.score >= 0);
  scored.sort((a, b) => a.score - b.score || a.item.name.localeCompare(b.item.name));
  const seen = new Set();
  return scored.map((entry) => entry.item).filter((item) => {
    const key = `${item.type}:${item.name}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  }).slice(0, 40);
}
