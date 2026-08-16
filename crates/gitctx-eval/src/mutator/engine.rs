//! Synthetic mutation execution engine.
//!
//! Applies transformation classes M1-M10 directly onto a `RepoBuilder` git repository
//! and generates corresponding `LineageLink` ground truth entries per D-09, D-11.

use crate::fixtures::mkrepo::RepoBuilder;
use crate::mutator::manifest::{
    GroundTruthRelation, LineageGroundTruthManifest, LineageLink, SymbolLocation,
};
use crate::mutator::transformations::MutationClass;
use std::fs;
use std::path::Path;
use thiserror::Error;

/// Error types encountered during synthetic mutation execution.
#[derive(Debug, Error)]
pub enum MutationError {
    /// Underlying Git operation failed.
    #[error("Git error during mutation: {0}")]
    Git(#[from] git2::Error),

    /// Filesystem I/O operation failed.
    #[error("I/O error during mutation: {0}")]
    Io(#[from] std::io::Error),

    /// Manifest JSON serialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Invalid mutation argument or state.
    #[error("Mutation execution failed: {0}")]
    Execution(String),
}

/// Synthetic mutation transformation engine.
pub struct MutationEngine;

impl MutationEngine {
    /// Base sample code placed in `src/service.py` before any mutations.
    pub const BASE_SERVICE_PY: &'static str = r#""""Order processing service module."""

def calculate_tax(amount: float, rate: float = 0.05) -> float:
    """Calculate the tax for a given amount and rate."""
    tax = amount * rate
    return round(tax, 2)

def validate_order(order_id: int, items: list) -> bool:
    """Validate that an order has valid ID and non-empty items."""
    if order_id <= 0:
        return False
    return len(items) > 0

def process_item(item_id: str, quantity: int, price: float) -> float:
    """Compute total price for an item quantity."""
    total = quantity * price
    return total

def legacy_cleanup(temp_files: list) -> None:
    """Perform legacy temporary cleanup routine."""
    for f in temp_files:
        pass
"#;

    /// Initializes base service files in the repo builder if empty.
    pub fn init_base_repo(builder: &mut RepoBuilder) -> Result<git2::Oid, MutationError> {
        builder.write_file("src/service.py", Self::BASE_SERVICE_PY)?;
        let oid = builder.commit("feat: initial service module codebase")?;
        Ok(oid)
    }

    /// Applies a single mutation class to the repository and records the lineage link.
    pub fn apply_mutation(
        builder: &mut RepoBuilder,
        class: MutationClass,
    ) -> Result<LineageLink, MutationError> {
        if builder.commit_count() == 0 {
            Self::init_base_repo(builder)?;
        }

        match class {
            MutationClass::M1_InPlaceRename => Self::apply_m1_rename(builder),
            MutationClass::M2_CrossFileMove => Self::apply_m2_move(builder),
            MutationClass::M3_MoveAndRename => Self::apply_m3_move_and_rename(builder),
            MutationClass::M4_SignatureChange => Self::apply_m4_signature(builder),
            MutationClass::M5_Reformatting => Self::apply_m5_reformat(builder),
            MutationClass::M6_LocalVarRename => Self::apply_m6_var_rename(builder),
            MutationClass::M7_FunctionSplit => Self::apply_m7_split(builder),
            MutationClass::M8_DecoyDeleteAdd => Self::apply_m8_decoy(builder),
            MutationClass::M9_DirectoryMove => Self::apply_m9_dir_move(builder),
            MutationClass::M10_FunctionCopy => Self::apply_m10_copy(builder),
        }
    }

    /// Applies a composite chain of mutations sequentially across separate Git commits per D-11.
    pub fn apply_chain(
        builder: &mut RepoBuilder,
        classes: &[MutationClass],
    ) -> Result<LineageGroundTruthManifest, MutationError> {
        if classes.is_empty() {
            return Err(MutationError::Execution(
                "Cannot apply an empty mutation chain".to_string(),
            ));
        }

        let mut manifest = LineageGroundTruthManifest::new(classes.len());

        for &class in classes {
            let link = Self::apply_mutation(builder, class)?;
            manifest.add_link(link);
        }

        Ok(manifest)
    }

    // --- Private Transformation Implementations ---

    fn apply_m1_rename(builder: &mut RepoBuilder) -> Result<LineageLink, MutationError> {
        let service_path = builder.path().join("src/service.py");
        let content = if service_path.exists() {
            fs::read_to_string(&service_path)?
        } else {
            Self::BASE_SERVICE_PY.to_string()
        };

        let updated = content.replace("def calculate_tax(", "def compute_tax(");
        builder.write_file("src/service.py", &updated)?;
        let oid = builder.commit("refactor(M1): rename calculate_tax to compute_tax")?;

        Ok(LineageLink::new(
            oid.to_string(),
            MutationClass::M1_InPlaceRename,
            SymbolLocation::new("src/service.py", "service.calculate_tax", 3, 6),
            SymbolLocation::new("src/service.py", "service.compute_tax", 3, 6),
            GroundTruthRelation::Renamed,
            1.0,
        ))
    }

    fn apply_m2_move(builder: &mut RepoBuilder) -> Result<LineageLink, MutationError> {
        let billing_code = r#""""Billing and invoice validation module."""

def validate_order(order_id: int, items: list) -> bool:
    """Validate that an order has valid ID and non-empty items."""
    if order_id <= 0:
        return False
    return len(items) > 0
"#;
        builder.write_file("src/billing.py", billing_code)?;

        let service_path = builder.path().join("src/service.py");
        if service_path.exists() {
            let content = fs::read_to_string(&service_path)?;
            let updated = content.replace(
                "def validate_order(order_id: int, items: list) -> bool:\n    \"\"\"Validate that an order has valid ID and non-empty items.\"\"\"\n    if order_id <= 0:\n        return False\n    return len(items) > 0\n\n",
                "",
            );
            builder.write_file("src/service.py", &updated)?;
        }

        let oid = builder.commit("refactor(M2): move validate_order to src/billing.py")?;

        Ok(LineageLink::new(
            oid.to_string(),
            MutationClass::M2_CrossFileMove,
            SymbolLocation::new("src/service.py", "service.validate_order", 8, 12),
            SymbolLocation::new("src/billing.py", "billing.validate_order", 3, 7),
            GroundTruthRelation::Moved,
            1.0,
        ))
    }

    fn apply_m3_move_and_rename(builder: &mut RepoBuilder) -> Result<LineageLink, MutationError> {
        let tracker_code = r#""""Inventory item tracking module."""

def track_inventory_item(item_id: str, quantity: int, unit_price: float) -> float:
    """Compute total inventory value for item quantity."""
    total = quantity * unit_price
    return total
"#;
        builder.write_file("src/inventory/tracker.py", tracker_code)?;

        let service_path = builder.path().join("src/service.py");
        if service_path.exists() {
            let content = fs::read_to_string(&service_path)?;
            let updated = content.replace(
                "def process_item(item_id: str, quantity: int, price: float) -> float:\n    \"\"\"Compute total price for an item quantity.\"\"\"\n    total = quantity * price\n    return total\n\n",
                "",
            );
            builder.write_file("src/service.py", &updated)?;
        }

        let oid = builder.commit(
            "refactor(M3): move and rename process_item to src/inventory/tracker.py::track_inventory_item",
        )?;

        Ok(LineageLink::new(
            oid.to_string(),
            MutationClass::M3_MoveAndRename,
            SymbolLocation::new("src/service.py", "service.process_item", 14, 18),
            SymbolLocation::new(
                "src/inventory/tracker.py",
                "inventory.tracker.track_inventory_item",
                3,
                7,
            ),
            GroundTruthRelation::Moved,
            0.9,
        ))
    }

    fn apply_m4_signature(builder: &mut RepoBuilder) -> Result<LineageLink, MutationError> {
        let service_path = builder.path().join("src/service.py");
        let content = if service_path.exists() {
            fs::read_to_string(&service_path)?
        } else {
            Self::BASE_SERVICE_PY.to_string()
        };

        let updated = content.replace(
            "def process_item(item_id: str, quantity: int, price: float) -> float:",
            "def process_item(item_id: str, quantity: int, price: float, discount: float = 0.0, currency: str = \"USD\") -> float:",
        );
        builder.write_file("src/service.py", &updated)?;
        let oid = builder.commit("refactor(M4): extend process_item signature with discount")?;

        Ok(LineageLink::new(
            oid.to_string(),
            MutationClass::M4_SignatureChange,
            SymbolLocation::new("src/service.py", "service.process_item", 14, 18),
            SymbolLocation::new("src/service.py", "service.process_item", 14, 18),
            GroundTruthRelation::Exact,
            0.95,
        ))
    }

    fn apply_m5_reformat(builder: &mut RepoBuilder) -> Result<LineageLink, MutationError> {
        let service_path = builder.path().join("src/service.py");
        let content = if service_path.exists() {
            fs::read_to_string(&service_path)?
        } else {
            Self::BASE_SERVICE_PY.to_string()
        };

        let reformatted = format!("# Formatted according to PEP-8\n\n{content}\n# End of service file\n");
        builder.write_file("src/service.py", &reformatted)?;
        let oid = builder.commit("style(M5): apply formatting, comments, and spacing")?;

        Ok(LineageLink::new(
            oid.to_string(),
            MutationClass::M5_Reformatting,
            SymbolLocation::new("src/service.py", "service.calculate_tax", 3, 6),
            SymbolLocation::new("src/service.py", "service.calculate_tax", 5, 8),
            GroundTruthRelation::Exact,
            1.0,
        ))
    }

    fn apply_m6_var_rename(builder: &mut RepoBuilder) -> Result<LineageLink, MutationError> {
        let service_path = builder.path().join("src/service.py");
        let content = if service_path.exists() {
            fs::read_to_string(&service_path)?
        } else {
            Self::BASE_SERVICE_PY.to_string()
        };

        let updated = content.replace(
            "total = quantity * price\n    return total",
            "grand_total = quantity * price\n    return grand_total",
        );
        builder.write_file("src/service.py", &updated)?;
        let oid = builder.commit("refactor(M6): rename local variable total -> grand_total")?;

        Ok(LineageLink::new(
            oid.to_string(),
            MutationClass::M6_LocalVarRename,
            SymbolLocation::new("src/service.py", "service.process_item", 14, 18),
            SymbolLocation::new("src/service.py", "service.process_item", 14, 18),
            GroundTruthRelation::Exact,
            0.98,
        ))
    }

    fn apply_m7_split(builder: &mut RepoBuilder) -> Result<LineageLink, MutationError> {
        let service_path = builder.path().join("src/service.py");
        let content = if service_path.exists() {
            fs::read_to_string(&service_path)?
        } else {
            Self::BASE_SERVICE_PY.to_string()
        };

        let split_addition = r#"def _compute_item_subtotal(quantity: int, price: float) -> float:
    """Helper extracted from process_item to compute raw subtotal."""
    return quantity * price

def process_item(item_id: str, quantity: int, price: float) -> float:
    """Compute total price for an item quantity using helper."""
    return _compute_item_subtotal(quantity, price)
"#;

        let updated = content.replace(
            "def process_item(item_id: str, quantity: int, price: float) -> float:\n    \"\"\"Compute total price for an item quantity.\"\"\"\n    total = quantity * price\n    return total\n",
            split_addition,
        );
        builder.write_file("src/service.py", &updated)?;
        let oid = builder.commit("refactor(M7): split _compute_item_subtotal helper from process_item")?;

        Ok(LineageLink::new(
            oid.to_string(),
            MutationClass::M7_FunctionSplit,
            SymbolLocation::new("src/service.py", "service.process_item", 14, 18),
            SymbolLocation::new("src/service.py", "service._compute_item_subtotal", 14, 17),
            GroundTruthRelation::SplitFrom,
            0.85,
        ))
    }

    fn apply_m8_decoy(builder: &mut RepoBuilder) -> Result<LineageLink, MutationError> {
        let service_path = builder.path().join("src/service.py");
        let content = if service_path.exists() {
            fs::read_to_string(&service_path)?
        } else {
            Self::BASE_SERVICE_PY.to_string()
        };

        let decoy_replacement = r#"def audit_telemetry(payload: dict) -> None:
    """Emit telemetry payload for security auditing."""
    if not payload:
        return
"#;

        let updated = content.replace(
            "def legacy_cleanup(temp_files: list) -> None:\n    \"\"\"Perform legacy temporary cleanup routine.\"\"\"\n    for f in temp_files:\n        pass\n",
            decoy_replacement,
        );
        builder.write_file("src/service.py", &updated)?;
        let oid = builder.commit("feat(M8): replace legacy_cleanup with audit_telemetry")?;

        Ok(LineageLink::new(
            oid.to_string(),
            MutationClass::M8_DecoyDeleteAdd,
            SymbolLocation::new("src/service.py", "service.legacy_cleanup", 20, 24),
            SymbolLocation::new("src/service.py", "service.audit_telemetry", 20, 24),
            GroundTruthRelation::Decoy,
            0.0,
        ))
    }

    fn apply_m9_dir_move(builder: &mut RepoBuilder) -> Result<LineageLink, MutationError> {
        let service_path = builder.path().join("src/service.py");
        let content = if service_path.exists() {
            fs::read_to_string(&service_path)?
        } else {
            Self::BASE_SERVICE_PY.to_string()
        };

        builder.remove_file("src/service.py")?;
        builder.write_file("src/core/service.py", &content)?;
        let oid = builder.commit("refactor(M9): move src/service.py to src/core/service.py")?;

        Ok(LineageLink::new(
            oid.to_string(),
            MutationClass::M9_DirectoryMove,
            SymbolLocation::new("src/service.py", "service.calculate_tax", 3, 6),
            SymbolLocation::new("src/core/service.py", "core.service.calculate_tax", 3, 6),
            GroundTruthRelation::Moved,
            1.0,
        ))
    }

    fn apply_m10_copy(builder: &mut RepoBuilder) -> Result<LineageLink, MutationError> {
        let backup_code = r#""""Backup tax calculations module."""

def calculate_tax(amount: float, rate: float = 0.05) -> float:
    """Calculate the tax for a given amount and rate (backup copy)."""
    tax = amount * rate
    return round(tax, 2)
"#;
        builder.write_file("src/backup/tax.py", backup_code)?;
        let oid = builder.commit("feat(M10): duplicate calculate_tax to src/backup/tax.py")?;

        Ok(LineageLink::new(
            oid.to_string(),
            MutationClass::M10_FunctionCopy,
            SymbolLocation::new("src/service.py", "service.calculate_tax", 3, 6),
            SymbolLocation::new("src/backup/tax.py", "backup.tax.calculate_tax", 3, 7),
            GroundTruthRelation::Copied,
            0.9,
        ))
    }

    /// Convenience runner to generate a complete repository and ground-truth manifest
    /// for a single mutation class or all classes with optional destination directory.
    pub fn generate_and_save(
        target_dir: Option<&Path>,
        class_filter: Option<&str>,
        chain_depth: usize,
    ) -> Result<(RepoBuilder, LineageGroundTruthManifest), MutationError> {
        let mut builder = if let Some(dir) = target_dir {
            if dir.exists() {
                let _ = fs::remove_dir_all(dir);
            }
            RepoBuilder::new_in(dir)?
        } else {
            RepoBuilder::new()?
        };

        let classes_to_apply: Vec<MutationClass> = if let Some(filter) = class_filter {
            if filter.eq_ignore_ascii_case("all") {
                MutationClass::all().to_vec()
            } else if let Some(c) = MutationClass::from_str_loose(filter) {
                vec![c; chain_depth.max(1)]
            } else {
                return Err(MutationError::Execution(format!(
                    "Unknown mutation class filter: '{filter}'"
                )));
            }
        } else {
            MutationClass::all().to_vec()
        };

        let manifest = Self::apply_chain(&mut builder, &classes_to_apply)?;

        if let Some(dir) = target_dir {
            let manifest_path = dir.join("lineage_ground_truth.json");
            manifest.save_to_file(&manifest_path)?;
        }

        Ok((builder, manifest))
    }
}
