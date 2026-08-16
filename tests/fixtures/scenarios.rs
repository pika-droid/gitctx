//! Six canonical scenario presets for git history testing per D-06.

use super::mkrepo::RepoBuilder;

/// Scenario 1: Linear History
pub fn scenario_linear_history(_builder: &mut RepoBuilder) -> Result<Vec<git2::Oid>, git2::Error> {
    Ok(Vec::new())
}
