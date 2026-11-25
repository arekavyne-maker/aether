// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Best-effort parser for clap's `--help` output.
//!
//! The CLI command tree is the source of truth. Its discovery surface gives
//! names and descriptions, but not argument shapes; clap's help text does.
//! Parsing is deliberately defensive: anything unrecognised is preserved in
//! `raw`, and the terminal always remains available as an escape hatch.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};

/// A positional argument from the `Arguments:` section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositionalSpec {
    /// Upper-case name as printed by clap, e.g. `FILE`.
    pub name: String,
    /// True for `<FILE>`, false for `[FILE]`.
    pub required: bool,
    /// Description text.
    pub help: String,
    /// Declared default, if any.
    #[serde(default)]
    pub default: Option<String>,
}

/// An option from the `Options:` section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptionSpec {
    /// Canonical long flag, e.g. `--project`.
    pub flag: String,
    /// Optional short flag, e.g. `-p`.
    #[serde(default)]
    pub short: Option<String>,
    /// Value placeholder, e.g. `PROJECT`. None for boolean flags.
    #[serde(default)]
    pub value_name: Option<String>,
    /// Description text.
    pub help: String,
    /// Declared default, if any.
    #[serde(default)]
    pub default: Option<String>,
}

/// Parsed help for one CLI command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandHelp {
    /// Full command name.
    pub name: String,
    /// One-line description.
    pub about: String,
    /// Usage line without the `Usage:` prefix.
    pub usage: String,
    /// Positional arguments.
    pub positionals: Vec<PositionalSpec>,
    /// Options, including the universal CLI flags.
    pub options: Vec<OptionSpec>,
    /// Child command names, for group commands.
    pub subcommands: Vec<String>,
    /// Original help text, always preserved.
    pub raw: String,
}

impl CommandHelp {
    /// True when this command is a group (has child commands).
    pub fn is_group(&self) -> bool {
        !self.subcommands.is_empty()
    }
}

fn default_in(text: &str) -> Option<String> {
    let start = text.find("[default: ")? + "[default: ".len();
    let rest = &text[start..];
    let end = rest.find(']')?;
    Some(rest[..end].trim().to_string())
}

fn split_columns(line: &str) -> Option<(&str, &str)> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b' ' && bytes[i + 1] == b' ' {
            let left = line[..i].trim();
            let right = line[i..].trim();
            if !left.is_empty() {
                return Some((left, right));
            }
        }
        i += 1;
    }
    None
}

/// Parse clap help text into structured fields.
pub fn parse_help(name: &str, text: &str) -> CommandHelp {
    let mut about = String::new();
    let mut usage = String::new();
    let mut positionals: Vec<PositionalSpec> = Vec::new();
    let mut options: Vec<OptionSpec> = Vec::new();
    let mut subcommands: Vec<String> = Vec::new();
    let mut section = "";
    let mut last_positional: Option<usize> = None;
    let mut last_option: Option<usize> = None;

    for raw in text.lines() {
        let line = raw.trim_end();
        if line.trim().is_empty() {
            continue;
        }
        if let Some(value) = line.strip_prefix("Usage:") {
            usage = value.trim().to_string();
            section = "";
            continue;
        }
        match line.trim() {
            "Arguments:" => {
                section = "arguments";
                continue;
            }
            "Options:" => {
                section = "options";
                continue;
            }
            "Commands:" => {
                section = "commands";
                continue;
            }
            _ => {}
        }

        let trimmed = line.trim_start();
        let indented = line.len() - trimmed.len() >= 2;
        if !indented {
            if about.is_empty() && section.is_empty() && usage.is_empty() {
                about = trimmed.to_string();
            }
            continue;
        }

        match section {
            "arguments" => {
                let (token, help) = match split_columns(trimmed) {
                    Some(parts) => parts,
                    None if trimmed.starts_with('<') || trimmed.starts_with('[') => {
                        // clap prints bare tokens when the argument has no help.
                        (trimmed, "")
                    }
                    None => {
                        if let Some(index) = last_positional {
                            positionals[index].help.push(' ');
                            positionals[index].help.push_str(trimmed);
                        }
                        continue;
                    }
                };
                if token.starts_with('<') || token.starts_with('[') {
                    let required = token.starts_with('<');
                    let clean = token
                        .trim_start_matches(['<', '['])
                        .trim_end_matches(['>', ']'])
                        .to_string();
                    positionals.push(PositionalSpec {
                        name: clean,
                        required,
                        help: help.to_string(),
                        default: default_in(help),
                    });
                    last_positional = Some(positionals.len() - 1);
                    last_option = None;
                } else if let Some(index) = last_positional {
                    positionals[index].help.push(' ');
                    positionals[index].help.push_str(token);
                }
            }
            "options" => {
                if trimmed.starts_with('-') {
                    let (head, help) = split_columns(trimmed).unwrap_or((trimmed, ""));
                    // Split the value placeholder, if present.
                    let (flags, value_name) = match head.find('<') {
                        Some(at) => (
                            head[..at].trim().to_string(),
                            Some(
                                head[at..]
                                    .trim_start_matches('<')
                                    .trim_end_matches('>')
                                    .to_string(),
                            ),
                        ),
                        None => (head.to_string(), None),
                    };
                    let mut flag = String::new();
                    let mut short = None;
                    for part in flags.split(',') {
                        let part = part.trim();
                        if part.starts_with("--") {
                            flag = part.to_string();
                        } else if part.starts_with('-') && short.is_none() {
                            short = Some(part.to_string());
                        }
                    }
                    if flag.is_empty() {
                        flag = flags.clone();
                    }
                    options.push(OptionSpec {
                        flag,
                        short,
                        value_name,
                        help: help.to_string(),
                        default: default_in(help),
                    });
                    last_option = Some(options.len() - 1);
                    last_positional = None;
                } else if let Some(index) = last_option {
                    options[index].help.push(' ');
                    options[index].help.push_str(trimmed);
                }
            }
            "commands" => {
                let (token, _) = split_columns(trimmed).unwrap_or((trimmed, ""));
                if token != "help" && !token.is_empty() {
                    subcommands.push(token.to_string());
                }
            }
            _ => {}
        }
    }

    CommandHelp {
        name: name.to_string(),
        about: if about.is_empty() { name.to_string() } else { about },
        usage,
        positionals,
        options,
        subcommands,
        raw: text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_required_positional_and_flags() {
        let text = r#"Report verification coverage; exits 30 if any requirement is not verified

Usage: aether vv coverage [OPTIONS] <FILE>

Arguments:
  <FILE>  Requirements package (YAML)

Options:
      --project <PROJECT>  Project root [default: .]
      --json               Machine-readable JSON output
      --offline            Refuse any network access
  -h, --help               Print help
"#;
        let help = parse_help("vv coverage", text);
        assert_eq!(help.positionals.len(), 1);
        assert_eq!(help.positionals[0].name, "FILE");
        assert!(help.positionals[0].required);
        assert_eq!(help.options.len(), 4);
        assert_eq!(help.options[0].flag, "--project");
        assert_eq!(help.options[0].value_name.as_deref(), Some("PROJECT"));
        assert_eq!(help.options[0].default.as_deref(), Some("."));
        assert_eq!(help.options[3].short.as_deref(), Some("-h"));
        assert!(!help.is_group());
    }

    #[test]
    fn parses_optional_positional_and_subcommands() {
        let text = r#"Engineering Graph operations

Usage: aether graph [OPTIONS] <COMMAND>

Commands:
  query  AQL subset query
  chain  Verify the event chain
  help   Print this message

Options:
      --json  Machine-readable JSON output
"#;
        let help = parse_help("graph", text);
        assert!(help.is_group());
        assert_eq!(help.subcommands, vec!["query", "chain"]);
        assert_eq!(help.positionals.len(), 0);
        assert_eq!(help.options.len(), 1);
    }

    #[test]
    fn preserves_raw_text() {
        let text = "anything at all\nUsage: aether status\n";
        let help = parse_help("status", text);
        assert_eq!(help.raw, text);
        assert_eq!(help.about, "anything at all");
    }
}
