//! Six canonical scenario presets for git history testing per D-06.
//!
//! Presets:
//! 1. `scenario_linear_history`: 5 sequential commits modifying `calculator.py`.
//! 2. `scenario_file_rename`: 3 commits demonstrating in-place file rename.
//! 3. `scenario_cross_file_move`: 3 commits extracting a helper from monolith to helper module.
//! 4. `scenario_squash_merge`: 3 feature commits collapsed into a single squash commit.
//! 5. `scenario_evil_merge`: 2 diverging branches merged with custom code not in either parent.
//! 6. `scenario_broken_syntax`: 3 commits with an intermediate syntax error for fault-tolerance testing.

use super::languages::python::{
    APP_BASE, AUTH_INITIAL, AUTH_TESTS, AUTH_TWEAKED, CALCULATOR_V1, CALCULATOR_V2, CALCULATOR_V3,
    CALCULATOR_V4, CALCULATOR_V5, CONFIG_VALID, FORMATTER_UPDATED, HELPERS_EXTRACTED,
    HELPERS_MODIFIED, MONOLITH_INITIAL, MONOLITH_REFACTORED, PARSER_BROKEN, PARSER_FIXED,
    PARSER_VALID, SERVICE_BASE, SERVICE_BRANCH_A, SERVICE_BRANCH_B, SERVICE_EVIL_MERGE,
    UTILS_FORMATTER,
};
use super::mkrepo::RepoBuilder;

fn io_err(e: &std::io::Error) -> git2::Error {
    git2::Error::from_str(&format!("IO error during fixture setup: {e}"))
}

/// Scenario 1: Linear History (5 sequential commits).
pub fn scenario_linear_history(builder: &mut RepoBuilder) -> Result<Vec<git2::Oid>, git2::Error> {
    let mut oids = Vec::with_capacity(5);

    // Commit 1: Initial calculator with add and subtract
    builder
        .write_file("calculator.py", CALCULATOR_V1)
        .map_err(|e| io_err(&e))?;
    oids.push(builder.commit("feat: initial calculator with add and subtract")?);

    // Commit 2: Add multiply
    builder
        .write_file("calculator.py", CALCULATOR_V2)
        .map_err(|e| io_err(&e))?;
    oids.push(builder.commit("feat: add multiply function")?);

    // Commit 3: Modify add implementation and docstrings
    builder
        .write_file("calculator.py", CALCULATOR_V3)
        .map_err(|e| io_err(&e))?;
    oids.push(builder.commit("refactor: update add validation and docstring")?);

    // Commit 4: Add divide with error handling
    builder
        .write_file("calculator.py", CALCULATOR_V4)
        .map_err(|e| io_err(&e))?;
    oids.push(builder.commit("feat: add divide with zero division handling")?);

    // Commit 5: Structure calculator as a class
    builder
        .write_file("calculator.py", CALCULATOR_V5)
        .map_err(|e| io_err(&e))?;
    oids.push(builder.commit("refactor: encapsulate math operations into Calculator class")?);

    Ok(oids)
}

/// Scenario 2: File Rename (3 commits).
pub fn scenario_file_rename(builder: &mut RepoBuilder) -> Result<Vec<git2::Oid>, git2::Error> {
    let mut oids = Vec::with_capacity(3);

    // Commit 1: Initial utils.py
    builder
        .write_file("utils.py", UTILS_FORMATTER)
        .map_err(|e| io_err(&e))?;
    oids.push(builder.commit("feat: add string formatting utils")?);

    // Commit 2: Rename utils.py to formatter.py (exact same content)
    builder.remove_file("utils.py").map_err(|e| io_err(&e))?;
    builder
        .write_file("formatter.py", UTILS_FORMATTER)
        .map_err(|e| io_err(&e))?;
    oids.push(builder.commit("refactor: rename utils.py to formatter.py")?);

    // Commit 3: Update format_output signature
    builder
        .write_file("formatter.py", FORMATTER_UPDATED)
        .map_err(|e| io_err(&e))?;
    oids.push(builder.commit("feat: add uppercase parameter to format_output")?);

    Ok(oids)
}

/// Scenario 3: Cross-File Move (3 commits).
pub fn scenario_cross_file_move(builder: &mut RepoBuilder) -> Result<Vec<git2::Oid>, git2::Error> {
    let mut oids = Vec::with_capacity(3);

    // Commit 1: Monolith with helper and main
    builder
        .write_file("monolith.py", MONOLITH_INITIAL)
        .map_err(|e| io_err(&e))?;
    oids.push(builder.commit("feat: initial monolith pipeline")?);

    // Commit 2: Extract helper to helpers.py and import in monolith.py
    builder
        .write_file("helpers.py", HELPERS_EXTRACTED)
        .map_err(|e| io_err(&e))?;
    builder
        .write_file("monolith.py", MONOLITH_REFACTORED)
        .map_err(|e| io_err(&e))?;
    oids.push(builder.commit("refactor: extract helper_fn to helpers.py")?);

    // Commit 3: Modify helper in helpers.py
    builder
        .write_file("helpers.py", HELPERS_MODIFIED)
        .map_err(|e| io_err(&e))?;
    oids.push(builder.commit("feat: add deduplication to helper_fn in helpers.py")?);

    Ok(oids)
}

/// Scenario 4: Squash Merge (1 base + 3 feature commits squashed into 1).
pub fn scenario_squash_merge(builder: &mut RepoBuilder) -> Result<Vec<git2::Oid>, git2::Error> {
    let mut oids = Vec::new();

    // Commit 1 (base branch): Base app
    builder
        .write_file("app.py", APP_BASE)
        .map_err(|e| io_err(&e))?;
    oids.push(builder.commit("feat: initial application base")?);

    let base_branch = builder.current_branch_name()?;

    // Branch feature
    builder.create_branch("feature")?;
    builder.checkout_branch("feature")?;

    // Feature Commit A: Add auth.py
    builder
        .write_file("auth.py", AUTH_INITIAL)
        .map_err(|e| io_err(&e))?;
    let _ = builder.commit("feat(auth): initial authentication logic")?;

    // Feature Commit B: Tweak auth.py
    builder
        .write_file("auth.py", AUTH_TWEAKED)
        .map_err(|e| io_err(&e))?;
    let _ = builder.commit("feat(auth): add role extraction")?;

    // Feature Commit C: Add auth tests
    builder
        .write_file("test_auth.py", AUTH_TESTS)
        .map_err(|e| io_err(&e))?;
    let _ = builder.commit("test(auth): add unit tests for auth module")?;

    // Switch back to base branch and squash merge feature
    builder.checkout_branch(&base_branch)?;
    let squash_oid =
        builder.squash_merge("feature", "feat: squash merge feature/auth into base")?;
    oids.push(squash_oid);

    Ok(oids)
}

/// Scenario 5: Evil Merge (Base + 2 branches merged with unparented edits).
pub fn scenario_evil_merge(builder: &mut RepoBuilder) -> Result<Vec<git2::Oid>, git2::Error> {
    let mut oids = Vec::new();

    // Base commit on initial branch
    builder
        .write_file("service.py", SERVICE_BASE)
        .map_err(|e| io_err(&e))?;
    oids.push(builder.commit("feat: initial data ingestion service")?);

    let base_branch = builder.current_branch_name()?;

    // Branch A
    builder.create_branch("branch_a")?;
    builder.checkout_branch("branch_a")?;
    builder
        .write_file("service.py", SERVICE_BRANCH_A)
        .map_err(|e| io_err(&e))?;
    let _ = builder.commit("feat: add timeout support to process_data")?;

    // Branch B from base branch
    builder.checkout_branch(&base_branch)?;
    builder.create_branch("branch_b")?;
    builder.checkout_branch("branch_b")?;
    builder
        .write_file("service.py", SERVICE_BRANCH_B)
        .map_err(|e| io_err(&e))?;
    let _ = builder.commit("feat: add retry support to process_data")?;

    // Checkout Branch A and perform evil merge of Branch B
    builder.checkout_branch("branch_a")?;
    builder
        .write_file("service.py", SERVICE_EVIL_MERGE)
        .map_err(|e| io_err(&e))?;
    let merge_oid = builder.merge(
        "branch_b",
        "Merge branch_b into branch_a with custom resolution",
    )?;
    oids.push(merge_oid);

    Ok(oids)
}

/// Scenario 6: Broken Syntax (Valid -> Invalid syntax -> Fixed).
pub fn scenario_broken_syntax(builder: &mut RepoBuilder) -> Result<Vec<git2::Oid>, git2::Error> {
    let mut oids = Vec::with_capacity(3);

    // Commit 1: Valid parser and config
    builder
        .write_file("config.py", CONFIG_VALID)
        .map_err(|e| io_err(&e))?;
    builder
        .write_file("parser.py", PARSER_VALID)
        .map_err(|e| io_err(&e))?;
    oids.push(builder.commit("feat: initial parser and config")?);

    // Commit 2: Break parser syntax while config remains valid
    builder
        .write_file("parser.py", PARSER_BROKEN)
        .map_err(|e| io_err(&e))?;
    oids.push(builder.commit("wip: parser refactoring (syntax broken)")?);

    // Commit 3: Fix parser syntax and add batch parser
    builder
        .write_file("parser.py", PARSER_FIXED)
        .map_err(|e| io_err(&e))?;
    oids.push(builder.commit("fix: correct parser syntax and add batch processing")?);

    Ok(oids)
}
