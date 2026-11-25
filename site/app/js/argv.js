// Pure, dependency-free command-line tokenizer shared by the terminal and
// its Node tests. Shell operators are rejected: the browser never invokes a
// shell, only discrete argv elements.

const OPERATORS = new Set(['|', '&', ';', '<', '>', '`', '$', '\n', '\r']);

/**
 * Tokenize one command line into argv elements.
 * Returns { tokens, error }. The optional leading `aether` is removed.
 */
export function tokenize(line) {
  const tokens = [];
  let current = '';
  let quote = null;
  let active = false;

  const push = () => {
    if (active) {
      tokens.push(current);
      current = '';
      active = false;
    }
  };

  for (let i = 0; i < line.length; i += 1) {
    const char = line[i];
    if (quote) {
      if (char === quote) {
        quote = null;
      } else if (char === '\\' && i + 1 < line.length) {
        i += 1;
        current += line[i];
      } else {
        current += char;
      }
      active = true;
      continue;
    }
    if (OPERATORS.has(char)) {
      return { tokens: [], error: `shell operator "${char}" is not supported; Aether runs argv directly` };
    }
    if (char === '"' || char === "'") {
      quote = char;
      active = true;
    } else if (char === '\\' && i + 1 < line.length) {
      i += 1;
      current += line[i];
      active = true;
    } else if (/\s/.test(char)) {
      push();
    } else {
      current += char;
      active = true;
    }
  }
  if (quote) return { tokens: [], error: `unterminated ${quote} quote` };
  push();
  if (tokens[0] === 'aether') tokens.shift();
  return { tokens, error: null };
}

/** Quote tokens for display so a copied line is unambiguous. */
export function formatArgv(tokens) {
  return tokens.map((token) => {
    if (/[\s'"\\]/.test(token)) {
      return `"${token.replace(/(["\\])/g, '\\$1')}"`;
    }
    return token;
  }).join(' ');
}

export const BUILTINS = ['help', 'clear', 'history', 'theme', 'open'];

export function isBuiltin(tokens) {
  return tokens.length > 0 && BUILTINS.includes(tokens[0]);
}
