//! Deterministic integration tests for gitctx synthetic fixture generator.

#[path = "../../../tests/fixtures/mod.rs"]
mod fixtures;

use fixtures::languages;
use fixtures::mkrepo::RepoBuilder;
use fixtures::scenarios::*;
use std::fs;

#[test]
fn test_deterministic_hashes() {
    // Run linear history across 3 independent repository instances
    let oids_run1 = {
        let mut builder = RepoBuilder::new().expect("Failed to create builder 1");
        scenario_linear_history(&mut builder).expect("Run 1 failed")
    };

    let oids_run2 = {
        let mut builder = RepoBuilder::new().expect("Failed to create builder 2");
        scenario_linear_history(&mut builder).expect("Run 2 failed")
    };

    let oids_run3 = {
        let mut builder = RepoBuilder::new().expect("Failed to create builder 3");
        scenario_linear_history(&mut builder).expect("Run 3 failed")
    };

    assert_eq!(oids_run1.len(), 5);
    assert_eq!(
        oids_run1, oids_run2,
        "Commit OIDs must be identical across run 1 and run 2"
    );
    assert_eq!(
        oids_run2, oids_run3,
        "Commit OIDs must be identical across run 2 and run 3"
    );

    // Also verify determinism across all other scenarios
    {
        let mut b1 = RepoBuilder::new().unwrap();
        let mut b2 = RepoBuilder::new().unwrap();
        assert_eq!(
            scenario_file_rename(&mut b1).unwrap(),
            scenario_file_rename(&mut b2).unwrap()
        );
    }

    {
        let mut b1 = RepoBuilder::new().unwrap();
        let mut b2 = RepoBuilder::new().unwrap();
        assert_eq!(
            scenario_cross_file_move(&mut b1).unwrap(),
            scenario_cross_file_move(&mut b2).unwrap()
        );
    }

    {
        let mut b1 = RepoBuilder::new().unwrap();
        let mut b2 = RepoBuilder::new().unwrap();
        assert_eq!(
            scenario_squash_merge(&mut b1).unwrap(),
            scenario_squash_merge(&mut b2).unwrap()
        );
    }

    {
        let mut b1 = RepoBuilder::new().unwrap();
        let mut b2 = RepoBuilder::new().unwrap();
        assert_eq!(
            scenario_evil_merge(&mut b1).unwrap(),
            scenario_evil_merge(&mut b2).unwrap()
        );
    }

    {
        let mut b1 = RepoBuilder::new().unwrap();
        let mut b2 = RepoBuilder::new().unwrap();
        assert_eq!(
            scenario_broken_syntax(&mut b1).unwrap(),
            scenario_broken_syntax(&mut b2).unwrap()
        );
    }
}

#[test]
fn test_all_scenarios_structure_and_files() {
    // 1. Linear History
    {
        let mut builder = RepoBuilder::new().expect("linear");
        let oids = scenario_linear_history(&mut builder).expect("linear scenario");
        assert_eq!(oids.len(), 5);
        assert_eq!(builder.commit_count(), 5);

        let calc_content = fs::read_to_string(builder.path().join("calculator.py")).unwrap();
        assert_eq!(calc_content, languages::python::CALCULATOR_V5);

        // Verify each commit has 1 parent (except root)
        for (i, &oid) in oids.iter().enumerate() {
            let commit = builder.repo().find_commit(oid).unwrap();
            if i == 0 {
                assert_eq!(commit.parent_count(), 0);
            } else {
                assert_eq!(commit.parent_count(), 1);
                assert_eq!(commit.parent_id(0).unwrap(), oids[i - 1]);
            }
        }
    }

    // 2. File Rename
    {
        let mut builder = RepoBuilder::new().expect("rename");
        let oids = scenario_file_rename(&mut builder).expect("rename scenario");
        assert_eq!(oids.len(), 3);
        assert!(!builder.path().join("utils.py").exists());
        assert!(builder.path().join("formatter.py").exists());

        let fmt_content = fs::read_to_string(builder.path().join("formatter.py")).unwrap();
        assert_eq!(fmt_content, languages::python::FORMATTER_UPDATED);
    }

    // 3. Cross-File Move
    {
        let mut builder = RepoBuilder::new().expect("move");
        let oids = scenario_cross_file_move(&mut builder).expect("move scenario");
        assert_eq!(oids.len(), 3);
        assert!(builder.path().join("monolith.py").exists());
        assert!(builder.path().join("helpers.py").exists());

        let helpers_content = fs::read_to_string(builder.path().join("helpers.py")).unwrap();
        assert_eq!(helpers_content, languages::python::HELPERS_MODIFIED);
    }

    // 4. Squash Merge
    {
        let mut builder = RepoBuilder::new().expect("squash");
        let oids = scenario_squash_merge(&mut builder).expect("squash scenario");
        // Returns [base_oid, squash_oid]
        assert_eq!(oids.len(), 2);

        let squash_commit = builder.repo().find_commit(oids[1]).unwrap();
        // Squash merge must have exactly 1 parent on master
        assert_eq!(squash_commit.parent_count(), 1);
        assert_eq!(squash_commit.parent_id(0).unwrap(), oids[0]);

        // All feature files exist on master after squash merge
        assert!(builder.path().join("app.py").exists());
        assert!(builder.path().join("auth.py").exists());
        assert!(builder.path().join("test_auth.py").exists());
    }

    // 5. Evil Merge
    {
        let mut builder = RepoBuilder::new().expect("evil");
        let oids = scenario_evil_merge(&mut builder).expect("evil scenario");
        assert_eq!(oids.len(), 2); // base commit, evil merge commit

        let merge_commit = builder.repo().find_commit(oids[1]).unwrap();
        // Evil merge has 2 parents
        assert_eq!(merge_commit.parent_count(), 2);

        let service_content = fs::read_to_string(builder.path().join("service.py")).unwrap();
        assert!(service_content.contains("evil_merge_v1"));
        assert!(service_content.contains("telemetry"));
    }

    // 6. Broken Syntax
    {
        let mut builder = RepoBuilder::new().expect("broken");
        let oids = scenario_broken_syntax(&mut builder).expect("broken scenario");
        assert_eq!(oids.len(), 3);

        // Commit 2 (broken syntax) tree verification
        let broken_commit = builder.repo().find_commit(oids[1]).unwrap();
        let tree = broken_commit.tree().unwrap();
        let entry = tree.get_name("parser.py").unwrap();
        let blob = builder.repo().find_blob(entry.id()).unwrap();
        let blob_text = std::str::from_utf8(blob.content()).unwrap();
        assert_eq!(blob_text, languages::python::PARSER_BROKEN);

        // HEAD is fixed
        let parser_content = fs::read_to_string(builder.path().join("parser.py")).unwrap();
        assert_eq!(parser_content, languages::python::PARSER_FIXED);
        assert!(builder.path().join("config.py").exists());
    }
}

#[test]
fn test_author_and_timestamp_invariants() {
    let mut builder = RepoBuilder::new().expect("builder");
    let oids = scenario_linear_history(&mut builder).expect("linear");

    for (i, &oid) in oids.iter().enumerate() {
        let commit = builder.repo().find_commit(oid).unwrap();
        let author = commit.author();
        let committer = commit.committer();

        assert_eq!(author.name(), Some("gitctx-test"));
        assert_eq!(author.email(), Some("test@gitctx.local"));
        assert_eq!(committer.name(), Some("gitctx-test"));
        assert_eq!(committer.email(), Some("test@gitctx.local"));

        let expected_time = 1_700_000_000 + i64::try_from(i).unwrap_or(0) * 60;
        assert_eq!(author.when().seconds(), expected_time);
        assert_eq!(author.when().offset_minutes(), 0);
        assert_eq!(committer.when().seconds(), expected_time);
        assert_eq!(committer.when().offset_minutes(), 0);
    }
}
