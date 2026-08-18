use gitctx_core::error::CoreError;
use gitctx_core::model::Citation;
use gitctx_index::storage::TouchRecord;

/// Builder and validator for structured `Citation` metadata payloads per D-16, INDEX-07, and Invariant I4.
pub struct CitationBuilder;

impl CitationBuilder {
    /// Construct a verified `Citation` ensuring all fields conform to Invariant I4.
    pub fn build(
        commit_hash: impl Into<String>,
        path: impl Into<String>,
        start_line: usize,
        end_line: usize,
        match_rule: impl Into<String>,
        match_confidence: f32,
    ) -> Result<Citation, CoreError> {
        let commit_hash = commit_hash.into();
        let path = path.into();
        let match_rule = match_rule.into();

        if commit_hash.trim().is_empty() {
            return Err(CoreError::InvariantViolation(
                "Citation commit_hash cannot be empty".to_string(),
            ));
        }
        if path.trim().is_empty() {
            return Err(CoreError::InvariantViolation(
                "Citation path cannot be empty".to_string(),
            ));
        }
        if match_rule.trim().is_empty() {
            return Err(CoreError::InvariantViolation(
                "Citation match_rule cannot be empty".to_string(),
            ));
        }

        let clamped_confidence = if match_confidence.is_nan() || match_confidence < 0.0 {
            0.0
        } else if match_confidence > 1.0 {
            1.0
        } else {
            match_confidence
        };

        Ok(Citation {
            commit_hash,
            path,
            start_line,
            end_line: end_line.max(start_line),
            match_rule,
            match_confidence: clamped_confidence,
        })
    }

    /// Construct a verified `Citation` directly from a database `TouchRecord`.
    pub fn from_touch(
        touch: &TouchRecord,
        match_rule: &str,
        match_confidence: f32,
    ) -> Result<Citation, CoreError> {
        Self::build(
            &touch.commit_hash,
            &touch.path,
            touch.start_line,
            touch.end_line,
            match_rule,
            match_confidence,
        )
    }
}
