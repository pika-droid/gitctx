use gitctx_core::git::{DeltaStatus, GitRepo, GitWalker, TreeDiffExtractor};
use std::path::Path;

#[path = "../../../tests/fixtures/mod.rs"]
mod fixtures;
use fixtures::mkrepo::RepoBuilder;

#[test]
fn test_first_parent_walk_and_root_commit() {
    let mut builder = RepoBuilder::new().expect("create repo");
    builder
        .write_file("main.py", "def first():\n    pass\n")
        .unwrap();
    let root_oid = builder.commit("root commit").unwrap();

    builder
        .write_file("main.py", "def first():\n    return 1\n")
        .unwrap();
    let second_oid = builder.commit("second commit").unwrap();

    let repo = GitRepo::open(builder.path()).expect("open repo");
    let walker = GitWalker::new(&repo).expect("create walker");
    let commits = walker.collect_all().expect("collect commits");

    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].oid, second_oid);
    assert_eq!(commits[1].oid, root_oid);
    assert_eq!(commits[1].parent_ids.len(), 0);
    assert_eq!(commits[0].parent_ids.len(), 1);
}

#[test]
fn test_branch_merge_first_parent_filtering() {
    let mut builder = RepoBuilder::new().expect("create repo");
    builder.write_file("file.txt", "base content\n").unwrap();
    let root_oid = builder.commit("base commit").unwrap();

    // Create side branch
    builder.create_branch("feature").unwrap();

    // Commit on main branch
    builder
        .write_file("main_only.txt", "main content\n")
        .unwrap();
    let main_oid = builder.commit("commit on main").unwrap();

    // Checkout feature and commit
    builder.checkout_branch("feature").unwrap();
    builder
        .write_file("feature.txt", "feature content\n")
        .unwrap();
    let _feature_oid = builder.commit("commit on feature").unwrap();

    // Checkout main and merge
    builder.checkout_branch("master").unwrap();
    let merge_oid = builder.merge("feature", "merge feature branch").unwrap();

    let repo = GitRepo::open(builder.path()).expect("open repo");
    let walker = GitWalker::new(&repo).expect("create walker");
    let commits = walker.collect_all().expect("collect commits");

    // First-parent traversal must see: merge_oid -> main_oid -> root_oid (skipping feature_oid)
    assert_eq!(commits.len(), 3);
    assert_eq!(commits[0].oid, merge_oid);
    assert_eq!(commits[1].oid, main_oid);
    assert_eq!(commits[2].oid, root_oid);
}

#[test]
fn test_tree_diff_extraction_and_root_diff() {
    let mut builder = RepoBuilder::new().expect("create repo");
    builder.write_file("app.rs", "fn init() {}\n").unwrap();
    let root_oid = builder.commit("initial commit").unwrap();

    builder
        .write_file("app.rs", "fn init() {\n    println!(\"started\");\n}\n")
        .unwrap();
    builder.write_file("new_file.txt", "hello world\n").unwrap();
    let second_oid = builder.commit("update app and add new_file").unwrap();

    let repo = GitRepo::open(builder.path()).expect("open repo");

    // 1. Root commit diff (against empty tree)
    let root_diff = TreeDiffExtractor::extract(&repo, root_oid, None).expect("root diff");
    assert_eq!(root_diff.deltas.len(), 1);
    assert_eq!(root_diff.deltas[0].status, DeltaStatus::Added);
    assert_eq!(
        root_diff.deltas[0].new_path.as_deref(),
        Some(Path::new("app.rs"))
    );

    // 2. Second commit diff
    let second_diff =
        TreeDiffExtractor::extract(&repo, second_oid, Some(root_oid)).expect("second diff");
    assert_eq!(second_diff.deltas.len(), 2);

    let modified_delta = second_diff
        .deltas
        .iter()
        .find(|d| d.new_path.as_deref() == Some(Path::new("app.rs")))
        .expect("app.rs delta");
    assert_eq!(modified_delta.status, DeltaStatus::Modified);

    let added_delta = second_diff
        .deltas
        .iter()
        .find(|d| d.new_path.as_deref() == Some(Path::new("new_file.txt")))
        .expect("new_file.txt delta");
    assert_eq!(added_delta.status, DeltaStatus::Added);

    // Check hunks
    let hunks = second_diff
        .hunks_by_path
        .get(Path::new("app.rs"))
        .expect("hunks for app.rs");
    assert!(!hunks.is_empty());
}

#[test]
fn test_rename_detection() {
    let mut builder = RepoBuilder::new().expect("create repo");
    let content = "def calculate_total(items):\n    total = 0\n    for item in items:\n        total += item.price\n    return total\n";
    builder.write_file("old_calc.py", content).unwrap();
    let root_oid = builder.commit("add old_calc.py").unwrap();

    // Rename file and modify slightly
    builder.remove_file("old_calc.py").unwrap();
    let modified_content = "def calculate_total(items):\n    total = 0\n    for item in items:\n        total += item.price * 1.05\n    return total\n";
    builder.write_file("new_calc.py", modified_content).unwrap();
    let rename_oid = builder.commit("rename to new_calc.py with tax").unwrap();

    let repo = GitRepo::open(builder.path()).expect("open repo");
    let diff = TreeDiffExtractor::extract(&repo, rename_oid, Some(root_oid)).expect("rename diff");

    let rename_delta = diff
        .deltas
        .iter()
        .find(|d| d.status == DeltaStatus::Renamed)
        .expect("should find renamed delta");

    assert_eq!(
        rename_delta.old_path.as_deref(),
        Some(Path::new("old_calc.py"))
    );
    assert_eq!(
        rename_delta.new_path.as_deref(),
        Some(Path::new("new_calc.py"))
    );
    assert!(rename_delta.similarity > 50);
}
