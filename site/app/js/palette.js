// Command palette: fuzzy search across commands, workspaces and actions.
import { esc } from './format.js';
import { paletteCandidates } from './surface.js';

let options = {};
let list = [];
let index = 0;
let isOpen = false;

const ACTIONS = [
  { name: 'Open terminal', about: 'Switch to the embedded CLI', action: 'terminal', type: 'action' },
  { name: 'Open session ledger', about: 'Review every invocation', action: 'session', type: 'action' },
  { name: 'Toggle theme', about: 'Night / Paper', action: 'theme', type: 'action' },
  { name: 'Copy last command', about: 'Copy the most recent argv', action: 'copy-last', type: 'action' },
  { name: 'Clear session', about: 'Erase the local ledger', action: 'clear-session', type: 'action' },
  { name: 'Keyboard shortcuts', about: 'Show the help overlay', action: 'help', type: 'action' },
];

export function configure(next) { options = next; }

export function open() {
  const overlay = document.getElementById('palette-overlay');
  if (!overlay) return;
  isOpen = true;
  overlay.classList.add('open');
  const input = document.getElementById('palette-input');
  input.value = '';
  index = 0;
  render();
  input.focus();
  input.oninput = () => { index = 0; render(); };
  input.onkeydown = onKey;
}

export function close() {
  isOpen = false;
  document.getElementById('palette-overlay')?.classList.remove('open');
}

export function toggle() { isOpen ? close() : open(); }
export function isPaletteOpen() { return isOpen; }

function candidates(query) {
  const surface = options.getSurface?.();
  if (!surface) return [];
  const recent = options.getRecent?.() || [];
  const commands = paletteCandidates(surface, query, recent);
  const actions = ACTIONS.filter((a) => !query || `${a.name} ${a.about}`.toLowerCase().includes(query.toLowerCase()));
  return [...commands, ...actions].slice(0, 40);
}

function render() {
  const host = document.getElementById('palette-list');
  if (!host) return;
  const query = document.getElementById('palette-input')?.value || '';
  list = candidates(query);
  index = Math.min(index, Math.max(0, list.length - 1));
  host.innerHTML = list.length ? list.map((item, i) => {
    const label = item.type === 'workspace' ? 'Workspace' : item.type === 'action' ? 'Action' : (item.group ? 'Group' : item.kind || 'Command');
    return `<button class="palette-item ${i === index ? 'active' : ''}" data-pick="${i}">
      <span class="palette-kind">${esc(label)}</span>
      <span class="palette-name">${esc(item.name)}</span>
      <span class="palette-about">${esc(item.about || '')}</span>
    </button>`;
  }).join('') : '<div class="empty small">No matches.</div>';
  host.querySelectorAll('[data-pick]').forEach((button) => {
    button.addEventListener('click', () => pick(Number(button.dataset.pick)));
  });
}

function pick(i) {
  const item = list[i];
  if (!item) return;
  close();
  if (item.type === 'workspace') options.onWorkspace?.(item.target);
  else if (item.type === 'action') options.onAction?.(item.action);
  else options.onCommand?.(item);
}

function onKey(event) {
  if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
    event.preventDefault();
    index = (index + (event.key === 'ArrowDown' ? 1 : -1) + Math.max(list.length, 1)) % Math.max(list.length, 1);
    render();
    document.querySelector('.palette-item.active')?.scrollIntoView({ block: 'nearest' });
  } else if (event.key === 'Enter') {
    event.preventDefault();
    pick(index);
  } else if (event.key === 'Escape') {
    event.preventDefault();
    close();
  }
}
