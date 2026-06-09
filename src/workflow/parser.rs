//! Response parser for Kimi markdown outputs.
//!
//! Extracts structured fields from markdown responses using regex.

use std::sync::LazyLock;

use regex::{Regex, RegexBuilder};

use crate::error::{JobsmithError, Result};

/// Parsed fit evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evaluation {
    /// Fit score (0–100).
    pub score: i32,
    /// Verdict: accept, reject, or marginal.
    pub verdict: String,
    /// Reasoning text.
    pub reasoning: String,
}

/// Known section headers that terminate the reasoning block.
const REASONING_TERMINATORS: &[&str] = &[
    "GAPS:",
    "STRENGTHS:",
    "SCORES:",
    "CRITIQUE:",
    "ACTION_ITEMS:",
    "---CV---",
    "---COVER---",
];

static SCORE_RE: LazyLock<std::result::Result<Regex, String>> = LazyLock::new(|| {
    Regex::new(r"(?im)^SCORE:\s*(\d+)")
        .map_err(|e| format!("invalid regex: {e}"))
});

static VERDICT_RE: LazyLock<std::result::Result<Regex, String>> = LazyLock::new(|| {
    Regex::new(r"(?im)^VERDICT:\s*(accept|reject|marginal)")
        .map_err(|e| format!("invalid regex: {e}"))
});

static REASONING_START_RE: LazyLock<std::result::Result<Regex, String>> = LazyLock::new(|| {
    Regex::new(r"(?im)^REASONING:\s*")
        .map_err(|e| format!("invalid regex: {e}"))
});

static REASONING_END_RE: LazyLock<std::result::Result<Regex, String>> = LazyLock::new(|| {
    let pattern = REASONING_TERMINATORS
        .iter()
        .map(|t| regex::escape(t))
        .collect::<Vec<_>>()
        .join("|");
    RegexBuilder::new(&format!("(?i){}", pattern))
        .build()
        .map_err(|e| format!("invalid regex: {e}"))
});

/// Parse a fit evaluation response.
///
/// Looks for `SCORE: <n>`, `VERDICT: <accept|reject|marginal>`, and
/// `REASONING: <text>` in the markdown.
pub fn parse_evaluation(text: &str) -> Result<Evaluation> {
    let score_re = SCORE_RE
        .as_ref()
        .map_err(|e| JobsmithError::Process(e.clone()))?;
    let score: i32 = score_re
        .captures(text)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
        .ok_or_else(|| JobsmithError::Process("failed to parse SCORE from response".to_string()))?;

    if score > 100 {
        return Err(JobsmithError::Process(format!(
            "score {score} exceeds maximum of 100"
        )));
    }

    let verdict_re = VERDICT_RE
        .as_ref()
        .map_err(|e| JobsmithError::Process(e.clone()))?;
    let verdict = verdict_re
        .captures(text)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_lowercase())
        .ok_or_else(|| JobsmithError::Process("failed to parse VERDICT from response".to_string()))?;

    let reasoning = extract_reasoning(text)?;

    Ok(Evaluation {
        score,
        verdict,
        reasoning,
    })
}

fn extract_reasoning(text: &str) -> Result<String> {
    let re = REASONING_START_RE
        .as_ref()
        .map_err(|e| JobsmithError::Process(e.clone()))?;
    let Some(m) = re.find(text) else {
        return Ok(String::new());
    };
    let rest = &text[m.end()..];
    let end = REASONING_END_RE
        .as_ref()
        .map_err(|e| JobsmithError::Process(e.clone()))?
        .find(rest)
        .map(|m| m.start())
        .unwrap_or(rest.len());
    Ok(rest[..end].trim().to_string())
}

/// Parse a revision response into CV and cover letter.
///
/// Splits on `---CV---` and `---COVER---` markers.
pub fn parse_revised(text: &str) -> Result<(String, String)> {
    let cv_marker = "---CV---";
    let cover_marker = "---COVER---";

    let cv_start = text
        .find(cv_marker)
        .map(|i| i + cv_marker.len())
        .ok_or_else(|| JobsmithError::Process("failed to parse CV from revision response".to_string()))?;

    let cover_pos = text.find(cover_marker);

    let (cv, cover) = if let Some(pos) = cover_pos {
        if cv_start > pos {
            return Err(JobsmithError::Process(
                "invalid marker order: ---COVER--- before ---CV---".to_string(),
            ));
        }
        let cv_text = text[cv_start..pos].trim().to_string();
        let cover_text = text[pos + cover_marker.len()..].trim().to_string();
        (cv_text, cover_text)
    } else {
        let cv_text = text[cv_start..].trim().to_string();
        (cv_text, String::new())
    };

    if cover.is_empty() {
        return Err(JobsmithError::Process(
            "failed to parse cover letter from revision response".to_string(),
        ));
    }

    Ok((cv, cover))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_evaluation_full() -> Result<()> {
        let text = r#"SCORE: 75
VERDICT: accept
REASONING: Great fit for the role.
Skills align well with requirements.

GAPS: None
STRENGTHS: Rust experience
"#;
        let eval = parse_evaluation(text)?;
        assert_eq!(eval.score, 75);
        assert_eq!(eval.verdict, "accept");
        assert!(eval.reasoning.contains("Great fit"));
        Ok(())
    }

    #[test]
    fn parse_evaluation_score_out_of_range_fails() {
        let text = "SCORE: 150\nVERDICT: accept\nREASONING: ok";
        assert!(parse_evaluation(text).is_err());
    }

    #[test]
    fn parse_evaluation_score_at_boundary_ok() -> Result<()> {
        let text = "SCORE: 100\nVERDICT: accept\nREASONING: ok";
        let eval = parse_evaluation(text)?;
        assert_eq!(eval.score, 100);
        Ok(())
    }

    #[test]
    fn parse_evaluation_reject() -> Result<()> {
        let text = "SCORE: 30\nVERDICT: reject\nREASONING: poor fit";
        let eval = parse_evaluation(text)?;
        assert_eq!(eval.score, 30);
        assert_eq!(eval.verdict, "reject");
        Ok(())
    }

    #[test]
    fn parse_evaluation_missing_score_fails() {
        let text = "VERDICT: accept\nREASONING: ok";
        assert!(parse_evaluation(text).is_err());
    }

    #[test]
    fn parse_revised_ok() -> Result<()> {
        let text = r#"---CV---
# John Doe

Rust developer

---COVER---
Dear hiring manager,

I am applying.
"#;
        let (cv, cover) = parse_revised(text)?;
        assert!(cv.contains("John Doe"));
        assert!(cover.contains("Dear hiring manager"));
        Ok(())
    }

    #[test]
    fn parse_revised_missing_cv_fails() {
        let text = "---COVER---\nHello\n";
        assert!(parse_revised(text).is_err());
    }
}
