//! Salary benchmark lookup with fuzzy matching.
//!
//! Implements the matching algorithm from the original Python tool:
//! - Normalize strings (lowercase, strip legal suffixes, remove non-alphanumeric)
//! - Exact match → 100 points
//! - Substring match → 80-90 points
//! - Word overlap → 30-70 points
//! - Anglicized variants for Nordic characters (kept for compatibility)

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use tracing::{debug, instrument};
use unicode_normalization::UnicodeNormalization;

use crate::error::{JobsmithError, Result};

/// Legal suffixes and noise to strip when matching company names.
/// Adapted for Russian market (ООО, АО, ИП, ПАО, etc.).
const STRIP_PATTERNS: &[&str] = &[
    r"\bооо\b",
    r"\bао\b",
    r"\bпао\b",
    r"\bнао\b",
    r"\bип\b",
    r"\bзао\b",
    r"\bоао\b",
    r"\bгрупп\b",
    r"\bgroup\b",
    r"\bхолдинг\b",
    r"\bholding\b",
    r"\bроссия\b",
    r"\bроссийская\b",
    r"\bfederation\b",
    r"\bфедерация\b",
    r"\(.*\)",
    r",\s*.*$",
];

/// Minimum match score to include a result.
const MIN_SCORE: i32 = 30;

/// Salary data entry.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SalaryEntry {
    pub company: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub extra: serde_json::Value,
}

/// Salary dataset with metadata.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SalaryData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<SalaryMetadata>,
    pub companies: Vec<SalaryEntry>,
}

/// Dataset metadata.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SalaryMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_baseline: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline_description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Salary lookup engine.
#[derive(Debug)]
pub struct SalaryLookup {
    data: SalaryData,
    strip_re: Vec<Regex>,
}

impl SalaryLookup {
    /// Load salary data from a JSON file.
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(JobsmithError::Io)?;
        let data: SalaryData = serde_json::from_str(&content).map_err(JobsmithError::Json)?;
        Self::new(data)
    }

    /// Create a lookup engine from in-memory data.
    pub fn new(data: SalaryData) -> Result<Self> {
        let mut strip_re = Vec::new();
        for pat in STRIP_PATTERNS {
            match Regex::new(pat) {
                Ok(re) => strip_re.push(re),
                Err(e) => {
                    debug!(pattern = %pat, error = %e, "failed to compile strip pattern");
                }
            }
        }
        Ok(Self { data, strip_re })
    }

    /// Search for a company by name.
    #[instrument(skip(self), fields(query = %query))]
    pub fn search(&self, query: &str, city_filter: Option<&str>) -> Vec<&SalaryEntry> {
        let mut scored: Vec<(i32, &SalaryEntry)> = self
            .data
            .companies
            .iter()
            .filter_map(|entry| {
                if let Some(city) = city_filter {
                    let entry_city = entry.city.as_deref().unwrap_or("").to_lowercase();
                    if !entry_city.contains(&city.to_lowercase()) {
                        return None;
                    }
                }
                let score = self.match_score(query, &entry.company);
                if score >= MIN_SCORE {
                    Some((score, entry))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.company.cmp(&b.1.company)));
        scored.into_iter().map(|(_, entry)| entry).collect()
    }

    /// List all company names.
    pub fn list_all(&self) -> Vec<(&str, Option<&str>)> {
        self.data
            .companies
            .iter()
            .map(|e| (e.company.as_str(), e.city.as_deref()))
            .collect()
    }

    /// Compute a match score between 0 and 100.
    fn match_score(&self, query: &str, entry_name: &str) -> i32 {
        let q_norm = self.normalize(query);
        let n_norm = self.normalize(entry_name);

        if q_norm.is_empty() || n_norm.is_empty() {
            return 0;
        }

        // Exact match
        if q_norm == n_norm {
            return 100;
        }

        // Substring match (query in name)
        if n_norm.contains(&q_norm) {
            let ratio = q_norm.len() as f64 / n_norm.len() as f64;
            // chars(), not len(): byte length disables the guard for Cyrillic
            if q_norm.chars().count() <= 4 && ratio < 0.5 {
                let q_words: HashSet<_> = self.extract_words(query).into_iter().collect();
                let n_words: HashSet<_> = self.extract_words(entry_name).into_iter().collect();
                if !q_words.is_disjoint(&n_words) {
                    return 80 + (ratio * 10.0) as i32;
                }
            } else {
                return 80 + (ratio * 10.0) as i32;
            }
        }

        // Substring match (name in query)
        if q_norm.contains(&n_norm) {
            let ratio = n_norm.len() as f64 / q_norm.len() as f64;
            // chars(), not len(): byte length disables the guard for Cyrillic
            if n_norm.chars().count() <= 4 && ratio < 0.5 {
                let q_words: HashSet<_> = self.extract_words(query).into_iter().collect();
                let n_words: HashSet<_> = self.extract_words(entry_name).into_iter().collect();
                if !q_words.is_disjoint(&n_words) {
                    return 80 + (ratio * 10.0) as i32;
                }
            } else {
                return 80 + (ratio * 10.0) as i32;
            }
        }

        // Word overlap
        let q_words: HashSet<_> = self.extract_words(query).into_iter().collect();
        let n_words: HashSet<_> = self.extract_words(entry_name).into_iter().collect();

        if q_words.is_empty() || n_words.is_empty() {
            return 0;
        }

        let overlap: HashSet<_> = q_words.intersection(&n_words).collect();
        if !overlap.is_empty() {
            if q_words.len() == 1 {
                if let Some(q_word) = q_words.iter().next() {
                    if n_words.contains(q_word) {
                        return 70;
                    }
                }
                return 0;
            }
            let coverage = overlap.len() as f64 / q_words.len() as f64;
            return 30 + (coverage * 40.0) as i32;
        }

        0
    }

    /// Normalize a string for matching.
    fn normalize(&self, s: &str) -> String {
        let mut s = s.to_lowercase().trim().to_string();
        for re in &self.strip_re {
            s = re.replace_all(&s, "").to_string();
        }
        s.nfc()
            .collect::<String>()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .trim()
            .to_string()
    }

    /// Extract meaningful words from a string.
    fn extract_words(&self, s: &str) -> Vec<String> {
        s.to_lowercase()
            .split_whitespace()
            .map(|w| {
                let mut w = w.to_string();
                for re in &self.strip_re {
                    w = re.replace_all(&w, "").to_string();
                }
                w
            })
            .filter(|w| w.len() > 1)
            .collect::<Vec<_>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_data() -> SalaryData {
        SalaryData {
            metadata: None,
            companies: vec![
                SalaryEntry {
                    company: "Яндекс".to_string(),
                    city: Some("Москва".to_string()),
                    categories: None,
                    extra: serde_json::json!({}),
                },
                SalaryEntry {
                    company: "Яндекс.Такси".to_string(),
                    city: Some("Москва".to_string()),
                    categories: None,
                    extra: serde_json::json!({}),
                },
                SalaryEntry {
                    company: "СберБанк".to_string(),
                    city: Some("Москва".to_string()),
                    categories: None,
                    extra: serde_json::json!({}),
                },
                SalaryEntry {
                    company: "ООО СберТех".to_string(),
                    city: Some("Москва".to_string()),
                    categories: None,
                    extra: serde_json::json!({}),
                },
            ],
        }
    }

    #[test]
    fn test_exact_match() {
        let lookup = SalaryLookup::new(test_data()).unwrap();
        let results = lookup.search("Яндекс", None);
        assert!(!results.is_empty());
        assert_eq!(results[0].company, "Яндекс");
    }

    #[test]
    fn test_strip_ooo() {
        let lookup = SalaryLookup::new(test_data()).unwrap();
        let results = lookup.search("СберТех", None);
        assert!(!results.is_empty());
        assert_eq!(results[0].company, "ООО СберТех");
    }

    #[test]
    fn short_cyrillic_substring_needs_word_overlap() {
        let data = SalaryData {
            metadata: None,
            companies: vec![SalaryEntry {
                company: "Котировкабанк".to_string(),
                city: None,
                categories: None,
                extra: serde_json::json!({}),
            }],
        };
        let lookup = SalaryLookup::new(data).unwrap();
        // "кот" is 3 chars (6 UTF-8 bytes): the short-query guard must fire
        // and reject a substring-only match without shared words.
        let results = lookup.search("кот", None);
        assert!(
            results.is_empty(),
            "short query must not score 80+ on substring alone: {:?}",
            results.iter().map(|e| &e.company).collect::<Vec<_>>()
        );
    }

    #[test]
    fn short_cyrillic_name_inside_query_needs_word_overlap() {
        let data = SalaryData {
            metadata: None,
            companies: vec![SalaryEntry {
                company: "Сбер".to_string(),
                city: None,
                categories: None,
                extra: serde_json::json!({}),
            }],
        };
        let lookup = SalaryLookup::new(data).unwrap();
        let results = lookup.search("сберегательный союз вкладчиков", None);
        assert!(
            results.is_empty(),
            "short name must not score 80+ on substring alone: {:?}",
            results.iter().map(|e| &e.company).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_city_filter() {
        let lookup = SalaryLookup::new(test_data()).unwrap();
        let results = lookup.search("Яндекс", Some("Санкт-Петербург"));
        assert!(results.is_empty());
    }
}
