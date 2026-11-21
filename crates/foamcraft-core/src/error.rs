// SPDX-License-Identifier: GPL-3.0-or-later
//! Errors with stable codes and an English and German message plus a suggested fix.
//!
//! The catalog lives in `data/errors.json`. A code never changes once it is released.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    En,
    De,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Message {
    pub text: String,
    pub fix: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub en: Message,
    pub de: Message,
}

impl CatalogEntry {
    pub fn get(&self, lang: Language) -> &Message {
        match lang {
            Language::En => &self.en,
            Language::De => &self.de,
        }
    }
}

/// All known error codes with their messages.
pub fn catalog() -> &'static BTreeMap<String, CatalogEntry> {
    static CATALOG: OnceLock<BTreeMap<String, CatalogEntry>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        // The file is embedded at compile time and checked by tests/core/errors.rs, so a parse
        // failure can only come from a broken build.
        serde_json::from_str(include_str!("../data/errors.json")).unwrap_or_default()
    })
}

pub fn message(code: &str, lang: Language) -> Option<&'static Message> {
    catalog().get(code).map(|e| e.get(lang))
}

/// An error with a stable code and a technical detail (not translated).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FoamError {
    pub code: String,
    pub detail: String,
}

impl FoamError {
    pub fn new(code: &str, detail: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            detail: detail.into(),
        }
    }

    /// The translated message. Unknown codes fall back to the code itself.
    pub fn localized(&self, lang: Language) -> Message {
        message(&self.code, lang)
            .cloned()
            .unwrap_or_else(|| Message {
                text: self.code.clone(),
                fix: String::new(),
            })
    }
}

impl std::fmt::Display for FoamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = self.localized(Language::En).text;
        if self.detail.is_empty() {
            write!(f, "[{}] {text}", self.code)
        } else {
            write!(f, "[{}] {text} ({})", self.code, self.detail)
        }
    }
}

impl std::error::Error for FoamError {}
