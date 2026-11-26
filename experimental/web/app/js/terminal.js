// The embedded CLI. It accepts a command line, tokenizes it into discrete
// argv elements, and asks the host to invoke the canonical Aether binary.
import { tokenize, formatArgv, isBuiltin } from './argv.js';
import { esc, resultHtml } from './format.js';

const HISTORY_KEY = 'aether.history';
const HISTORY_CAP = 100;

function loadHistory() {
  try {
    const value = JSON.parse(localStorage.getItem(HISTORY_KEY) || '[]');
    return Array.isArray(value) ? value.slice(0, HISTORY_CAP) : [];
  } catch { return []; }
}

function saveHistory(history) {
  localStorage.setItem(HISTORY_KEY, JSON.stringify(history.slice(0, HISTORY_CAP)));
}

export class Terminal {
  constructor(options) {
    this.options = options;
    this.history = loadHistory();
    this.historyIndex = -1;
    this.entries = [];
    this.suggestions = [];
    this.suggestionIndex = 0;
    this.structured = true;
    this.input = null;
    this.log = null;
    this.suggestHost = null;
  }

  mount(container) {
    container.innerHTML = `
      <div class="terminal">
        <div class="term-head">
          <div><div class="eyebrow">Tools · CLI</div><h2>The canonical interface</h2></div>
          <div class="term-head-actions">
            <span class="pill">argv · no shell</span>
            <button class="btn ghost" data-term-structured>${this.structured ? 'JSON on' : 'JSON off'}</button>
            <button class="btn ghost" data-term-clear>Clear</button>
          </div>
        </div>
        <div class="term-log" id="term-log" aria-live="polite"></div>
        <div class="term-suggest" id="term-suggest"></div>
        <div class="term-prompt"><span class="term-sigil">aether &rsaquo;</span><input id="term-input" autocomplete="off" spellcheck="false" placeholder="status, schema list, vv coverage …"></div>
        <div class="term-foot"><span>127.0.0.1 · offline · shell operators rejected</span><span>&uarr;&darr; history · Tab complete · &amp;? help</span></div>
      </div>`;
    this.input = container.querySelector('#term-input');
    this.log = container.querySelector('#term-log');
    this.suggestHost = container.querySelector('#term-suggest');
    container.querySelector('[data-term-clear]').addEventListener('click', () => this.clear());
    container.querySelector('[data-term-structured]').addEventListener('click', (event) => {
      this.structured = !this.structured;
      event.currentTarget.textContent = this.structured ? 'JSON on' : 'JSON off';
      this.input.focus();
    });
    this.input.addEventListener('keydown', (event) => this.onKey(event));
    this.input.addEventListener('input', () => this.updateSuggestions());
    this.renderLog();
    return this.input;
  }

  focus() { this.input?.focus(); }

  clear() {
    this.entries = [];
    this.renderLog();
    this.input?.focus();
  }

  push(entry) {
    this.entries.push(entry);
    if (this.entries.length > 200) this.entries.shift();
    this.renderLog();
  }

  renderLog() {
    if (!this.log) return;
    if (!this.entries.length) {
      this.log.innerHTML = `<div class="term-intro">
        <p>Every line is passed to the canonical CLI as separate argv elements.</p>
        <p class="dim">Try <code>status</code>, <code>schema list</code>, or <code>vv coverage</code>. Type <code>help</code> for the built-ins.</p>
      </div>`;
      return;
    }
    this.log.innerHTML = this.entries.map((entry) => {
      if (entry.kind === 'local') {
        return `<div class="term-entry local"><div class="term-line"><span class="term-sigil">&rsaquo;</span>${esc(entry.line)}</div><pre class="term-message ${entry.tone || ''}">${esc(entry.message)}</pre></div>`;
      }
      return `<div class="term-entry"><div class="term-line"><span class="term-sigil">aether &rsaquo;</span>${esc(formatArgv(entry.argv))}</div>${resultHtml(entry.result, { commandName: entry.argv.slice(0, 2).join(' '), compact: true })}</div>`;
    }).join('');
    this.log.scrollTop = this.log.scrollHeight;
  }

  onKey(event) {
    if (event.key === 'Enter') {
      event.preventDefault();
      this.submit();
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      this.moveHistory(1);
    } else if (event.key === 'ArrowDown') {
      event.preventDefault();
      this.moveHistory(-1);
    } else if (event.key === 'Tab') {
      event.preventDefault();
      this.complete();
    } else if (event.key === 'Escape') {
      this.suggestions = [];
      this.renderSuggestions();
    } else if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'l') {
      event.preventDefault();
      this.clear();
    } else if (event.ctrlKey && event.key.toLowerCase() === 'c') {
      event.preventDefault();
      this.input.value = '';
      this.suggestions = [];
      this.renderSuggestions();
    }
  }

  moveHistory(direction) {
    if (!this.history.length) return;
    if (this.historyIndex < 0 && direction > 0) this.draft = this.input.value;
    this.historyIndex = Math.max(0, Math.min(this.history.length - 1, this.historyIndex + direction));
    this.input.value = this.history[this.historyIndex] || '';
    this.updateSuggestions();
  }

  complete() {
    if (!this.suggestions.length) this.updateSuggestions();
    const choice = this.suggestions[this.suggestionIndex];
    if (!choice) return;
    const parts = this.input.value.split(/\s+/);
    if (choice.startsWith('--')) {
      if (parts.length > 1) parts[parts.length - 1] = choice;
      else this.input.value = `${choice} `;
      if (parts.length > 1) this.input.value = `${parts.join(' ')} `;
    } else {
      this.input.value = `${choice} `;
    }
    this.suggestions = [];
    this.renderSuggestions();
    this.input.focus();
  }

  updateSuggestions() {
    const value = this.input.value;
    const last = value.split(/\s+/).pop() || '';
    const surface = this.options.surface?.();
    if (!surface || !last.trim()) {
      this.suggestions = [];
      this.renderSuggestions();
      return;
    }
    if (last.startsWith('--')) {
      const tokens = tokenize(value).tokens;
      const name = tokens.filter((t) => !t.startsWith('--')).join(' ');
      const help = this.options.getHelp?.(name);
      this.suggestions = help?.options?.map((o) => o.flag).filter((f) => f.startsWith(last)).slice(0, 8) || [];
    } else {
      this.suggestions = surface.commands
        .filter((c) => c.name.startsWith(last) && !c.group)
        .map((c) => c.name)
        .slice(0, 8);
    }
    this.suggestionIndex = 0;
    this.renderSuggestions();
  }

  renderSuggestions() {
    if (!this.suggestHost) return;
    if (!this.suggestions.length) {
      this.suggestHost.innerHTML = '';
      this.suggestHost.classList.remove('open');
      return;
    }
    this.suggestHost.innerHTML = this.suggestions.map((item, index) =>
      `<button class="suggestion ${index === this.suggestionIndex ? 'active' : ''}" data-suggest="${esc(item)}">${esc(item)}</button>`
    ).join('');
    this.suggestHost.classList.add('open');
    this.suggestHost.querySelectorAll('[data-suggest]').forEach((button) => {
      button.addEventListener('mousedown', (event) => {
        event.preventDefault();
        this.input.value = `${button.dataset.suggest} `;
        this.suggestions = [];
        this.renderSuggestions();
        this.input.focus();
      });
    });
  }

  async submit() {
    const line = this.input.value.trim();
    if (!line) return;
    this.input.value = '';
    this.suggestions = [];
    this.renderSuggestions();
    const parsed = tokenize(line);
    if (parsed.error) {
      this.push({ kind: 'local', line, message: parsed.error, tone: 'bad' });
      return;
    }
    const argv = parsed.tokens;
    if (!argv.length) return;
    this.history = [line, ...this.history.filter((h) => h !== line)].slice(0, HISTORY_CAP);
    saveHistory(this.history);
    this.historyIndex = -1;

    if (isBuiltin(argv)) {
      const message = await this.options.onBuiltin?.(argv, this);
      this.push({ kind: 'local', line, message: message || '', tone: '' });
      return;
    }
    const entry = { argv, pending: true };
    this.push(entry);
    const index = this.entries.indexOf(entry);
    const result = await this.options.onInvoke?.(argv, this.structured);
    if (result?.job && result.done) {
      this.entries[index] = {
        argv,
        result: {
          state: 'running', exit_code: null, duration_ms: 0,
          command: argv.join(' '), value: null,
          text: 'Running — follow the live output in the run dock.',
          stderr: '',
        },
      };
      this.renderLog();
      result.done.then((final) => {
        this.entries[index] = { argv, result: final };
        this.renderLog();
      });
      return;
    }
    if (index >= 0) this.entries[index] = { argv, result };
    this.renderLog();
  }
}

export function terminalHelp() {
  return [
    'help                 this message',
    'clear                clear the scrollback',
    'history              show the last commands',
    'theme dark|light     switch the interface theme',
    'open <workspace>     open a workspace by id',
    '',
    'Any other line is passed to the Aether CLI as argv.',
    'Shell operators (| > < & ; $) are rejected by design.',
  ].join('\n');
}
