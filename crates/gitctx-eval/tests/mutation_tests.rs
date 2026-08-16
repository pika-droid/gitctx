//! Integration test suite for synthetic mutation engine and ground truth manifests.

use gitctx_eval::mutator::{
    GroundTruthRelation, LineageGroundTruthManifest, MutationClass, MutationEngine,
};
use gitctx_eval::RepoBuilder;
use tempfile::TempDir;

#[test]
fn test_m1_m10_classes() {
    for &class in MutationClass::all() {
        let mut builder = RepoBuilder::new().expect("Failed to create RepoBuilder");
        let link = MutationEngine::apply_mutation(&mut builder, class)
            .unwrap_or_else(|e| panic!("Mutation {class:?} failed: {e}"));

        // Assert 2 commits (initial base repo + mutation commit)
        assert_eq!(
            builder.commit_count(),
            2,
            "Mutation {class:?} should have 2 commits total"
        );

        // Verify LineageLink populated correctly
        assert_eq!(link.mutation_class, class);
        assert!(!link.commit_sha.is_empty());
        assert!(!link.source.path.is_empty());
        assert!(!link.source.qualname.is_empty());
        assert!(!link.target.path.is_empty());
        assert!(!link.target.qualname.is_empty());
        assert!(link.confidence_expected >= 0.0 && link.confidence_expected <= 1.0);

        // Verify commit OID matches HEAD
        let head_oid = builder.head_oid().expect("head oid");
        assert_eq!(link.commit_sha, head_oid.to_string());

        // Verify relation specific checks
        match class {
            MutationClass::M1_InPlaceRename => {
                assert_eq!(link.relation, GroundTruthRelation::Renamed);
                assert_eq!(link.source.path, link.target.path);
                assert_ne!(link.source.qualname, link.target.qualname);
            }
            MutationClass::M2_CrossFileMove | MutationClass::M9_DirectoryMove => {
                assert_eq!(link.relation, GroundTruthRelation::Moved);
                assert_ne!(link.source.path, link.target.path);
            }
            MutationClass::M3_MoveAndRename => {
                assert_eq!(link.relation, GroundTruthRelation::Moved);
                assert_ne!(link.source.path, link.target.path);
                assert_ne!(link.source.qualname, link.target.qualname);
            }
            MutationClass::M4_SignatureChange
            | MutationClass::M5_Reformatting
            | MutationClass::M6_LocalVarRename => {
                assert_eq!(link.relation, GroundTruthRelation::Exact);
            }
            MutationClass::M7_FunctionSplit => {
                assert_eq!(link.relation, GroundTruthRelation::SplitFrom);
            }
            MutationClass::M8_DecoyDeleteAdd => {
                assert_eq!(link.relation, GroundTruthRelation::Decoy);
                assert!(link.confidence_expected.abs() < f32::EPSILON);
            }
            MutationClass::M10_FunctionCopy => {
                assert_eq!(link.relation, GroundTruthRelation::Copied);
                assert_ne!(link.source.path, link.target.path);
            }
        }
    }
}

#[test]
fn test_composite_chains() {
    let mut builder = RepoBuilder::new().expect("builder");
    let chain = [
        MutationClass::M1_InPlaceRename,
        MutationClass::M4_SignatureChange,
        MutationClass::M2_CrossFileMove,
    ];

    let manifest =
        MutationEngine::apply_chain(&mut builder, &chain).expect("apply chain should succeed");

    assert_eq!(manifest.chain_depth, 3);
    assert_eq!(manifest.links.len(), 3);
    // 1 base commit + 3 mutation commits = 4 commits
    assert_eq!(builder.commit_count(), 4);

    assert_eq!(
        manifest.links[0].mutation_class,
        MutationClass::M1_InPlaceRename
    );
    assert_eq!(
        manifest.links[1].mutation_class,
        MutationClass::M4_SignatureChange
    );
    assert_eq!(
        manifest.links[2].mutation_class,
        MutationClass::M2_CrossFileMove
    );
}

#[test]
fn test_ground_truth_manifest_roundtrip_and_export() {
    let temp = TempDir::new().expect("tempdir");
    let manifest_path = temp.path().join("lineage_ground_truth.json");

    let (builder, manifest) = MutationEngine::generate_and_save(Some(temp.path()), Some("all"), 1)
        .expect("generate_and_save");

    assert_eq!(manifest.links.len(), 10);
    assert_eq!(builder.commit_count(), 11);
    assert!(manifest_path.exists());

    // Roundtrip reload
    let loaded = LineageGroundTruthManifest::load_from_file(&manifest_path).expect("load manifest");
    assert_eq!(loaded.schema_version, 1);
    assert_eq!(loaded.links.len(), 10);
    assert_eq!(loaded.links, manifest.links);
}
