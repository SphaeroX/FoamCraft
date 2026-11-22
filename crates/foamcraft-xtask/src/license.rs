// SPDX-License-Identifier: GPL-3.0-or-later
//! Dependency license check against the allowed list in docs/licensing.md.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Allowed licenses (SPDX ids) plus per-package exceptions with a reason.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Policy {
    allowed: BTreeSet<String>,
    #[serde(default)]
    pub exceptions: BTreeMap<String, String>,
}

impl Policy {
    pub fn new<'a>(allowed: impl IntoIterator<Item = &'a str>) -> Self {
        Self {
            allowed: allowed
                .into_iter()
                .map(|s| s.to_ascii_lowercase())
                .collect(),
            exceptions: BTreeMap::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut policy: Policy =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        policy.allowed = policy
            .allowed
            .iter()
            .map(|s| s.to_ascii_lowercase())
            .collect();
        Ok(policy)
    }

    fn allows(&self, id: &str) -> bool {
        let id = id.to_ascii_lowercase();
        if self.allowed.contains(&id) {
            return true;
        }
        // The deprecated `+` suffix means "or later".
        id.strip_suffix('+').is_some_and(|base| {
            let base = base.trim_end_matches("-only");
            self.allowed.contains(&format!("{base}-or-later"))
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub ecosystem: String,
    pub name: String,
    pub version: String,
    pub license: Option<String>,
    /// Folder of the package sources (for its license files), when known.
    pub dir: Option<PathBuf>,
}

impl Package {
    pub fn new(ecosystem: &str, name: &str, version: &str, license: Option<&str>) -> Self {
        Self {
            ecosystem: ecosystem.into(),
            name: name.into(),
            version: version.into(),
            license: license.map(Into::into),
            dir: None,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Token<'a> {
    Open,
    Close,
    And,
    Or,
    With,
    Id(&'a str),
}

fn word_token(word: &str) -> Token<'_> {
    if word.eq_ignore_ascii_case("and") {
        Token::And
    } else if word.eq_ignore_ascii_case("or") {
        Token::Or
    } else if word.eq_ignore_ascii_case("with") {
        Token::With
    } else {
        Token::Id(word)
    }
}

fn tokenize(expr: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in expr.char_indices() {
        let is_separator = c.is_whitespace() || matches!(c, '(' | ')' | '/');
        if !is_separator {
            start.get_or_insert(i);
            continue;
        }
        if let Some(s) = start.take() {
            tokens.push(word_token(&expr[s..i]));
        }
        match c {
            '(' => tokens.push(Token::Open),
            ')' => tokens.push(Token::Close),
            // Old Cargo manifests write `MIT/Apache-2.0` for a choice.
            '/' => tokens.push(Token::Or),
            _ => {}
        }
    }
    if let Some(s) = start {
        tokens.push(word_token(&expr[s..]));
    }
    tokens
}

/// Recursive descent: or := and ("OR" and)*; and := term ("AND" term)*;
/// term := "(" or ")" | id ["WITH" id].
struct Parser<'a, 'p> {
    tokens: Vec<Token<'a>>,
    pos: usize,
    policy: &'p Policy,
}

impl<'a> Parser<'a, '_> {
    fn peek(&self) -> Option<Token<'a>> {
        self.tokens.get(self.pos).copied()
    }

    fn or(&mut self) -> Option<bool> {
        let mut value = self.and()?;
        while self.peek() == Some(Token::Or) {
            self.pos += 1;
            value |= self.and()?;
        }
        Some(value)
    }

    fn and(&mut self) -> Option<bool> {
        let mut value = self.term()?;
        while self.peek() == Some(Token::And) {
            self.pos += 1;
            value &= self.term()?;
        }
        Some(value)
    }

    fn term(&mut self) -> Option<bool> {
        match self.peek()? {
            Token::Open => {
                self.pos += 1;
                let value = self.or()?;
                if self.peek() != Some(Token::Close) {
                    return None;
                }
                self.pos += 1;
                Some(value)
            }
            Token::Id(id) => {
                self.pos += 1;
                // An exception only adds permissions; the base license decides.
                if self.peek() == Some(Token::With) {
                    self.pos += 1;
                    if !matches!(self.peek(), Some(Token::Id(_))) {
                        return None;
                    }
                    self.pos += 1;
                }
                Some(self.policy.allows(id))
            }
            _ => None,
        }
    }
}

/// True when the SPDX expression can be satisfied with allowed licenses only. Malformed or empty
/// expressions are not allowed.
pub fn expression_allowed(expr: &str, policy: &Policy) -> bool {
    let mut parser = Parser {
        tokens: tokenize(expr),
        pos: 0,
        policy,
    };
    if parser.tokens.is_empty() {
        return false;
    }
    match parser.or() {
        Some(value) => value && parser.pos == parser.tokens.len(),
        None => false,
    }
}

/// Packages whose license is missing or not allowed and that have no exception.
pub fn violations<'a>(packages: &'a [Package], policy: &Policy) -> Vec<&'a Package> {
    packages
        .iter()
        .filter(|p| !policy.exceptions.contains_key(&p.name))
        .filter(|p| {
            !p.license
                .as_deref()
                .is_some_and(|l| expression_allowed(l, policy))
        })
        .collect()
}

#[derive(Deserialize)]
struct NpmLock {
    #[serde(default)]
    packages: BTreeMap<String, NpmEntry>,
}

#[derive(Deserialize)]
struct NpmEntry {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    license: Option<serde_json::Value>,
    #[serde(default)]
    link: bool,
}

/// Packages of an npm `package-lock.json` (lockfile version 2 or 3). The root and workspace
/// links are skipped; they are this project.
pub fn npm_packages(lock_path: &Path) -> Result<Vec<Package>, String> {
    let text =
        std::fs::read_to_string(lock_path).map_err(|e| format!("{}: {e}", lock_path.display()))?;
    let lock: NpmLock =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", lock_path.display()))?;
    let mut out = Vec::new();
    for (path, entry) in lock.packages {
        if entry.link || !path.contains("node_modules/") {
            continue;
        }
        let name = entry.name.unwrap_or_else(|| {
            path.rsplit_once("node_modules/")
                .map_or(path.clone(), |(_, n)| n.to_string())
        });
        let license = match entry.license {
            Some(serde_json::Value::String(s)) => Some(s),
            Some(serde_json::Value::Object(o)) => {
                o.get("type").and_then(|t| t.as_str()).map(String::from)
            }
            _ => None,
        };
        out.push(Package {
            ecosystem: "npm".into(),
            name,
            version: entry.version.unwrap_or_default(),
            license,
            dir: lock_path.parent().map(|d| d.join(&path)),
        });
    }
    Ok(out)
}

#[derive(Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoPackage>,
}

#[derive(Deserialize)]
struct CargoPackage {
    name: String,
    version: String,
    license: Option<String>,
    manifest_path: PathBuf,
}

/// All packages in the Cargo dependency graph of the workspace in `dir`.
pub fn cargo_packages(dir: &Path) -> Result<Vec<Package>, String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let output = std::process::Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--locked"])
        .current_dir(dir)
        .output()
        .map_err(|e| format!("cargo metadata: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let meta: CargoMetadata =
        serde_json::from_slice(&output.stdout).map_err(|e| format!("cargo metadata: {e}"))?;
    Ok(meta
        .packages
        .into_iter()
        .map(|p| Package {
            ecosystem: "cargo".into(),
            name: p.name,
            version: p.version,
            license: p.license,
            dir: p.manifest_path.parent().map(Path::to_path_buf),
        })
        .collect())
}
