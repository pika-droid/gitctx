//! Mutation transformation class definitions (M1-M10).
//!
//! Provides the complete catalog of synthetic code transformation categories
//! for lineage tracking evaluation per D-09.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Catalog of synthetic code mutation classes for lineage tracking evaluation.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationClass {
    /// M1: Renames a function in-place within the same file; body remains semantically identical.
    #[serde(rename = "m1_in_place_rename", alias = "M1_InPlaceRename", alias = "m1")]
    M1_InPlaceRename,

    /// M2: Moves a function from one file to another without modifying signature or body.
    #[serde(rename = "m2_cross_file_move", alias = "M2_CrossFileMove", alias = "m2")]
    M2_CrossFileMove,

    /// M3: Moves a function to another file and renames it in the same commit.
    #[serde(rename = "m3_move_and_rename", alias = "M3_MoveAndRename", alias = "m3")]
    M3_MoveAndRename,

    /// M4: Modifies function parameters (add, remove, or reorder parameters).
    #[serde(rename = "m4_signature_change", alias = "M4_SignatureChange", alias = "m4")]
    M4_SignatureChange,

    /// M5: Modifies formatting, docstrings, or whitespace without changing AST semantics.
    #[serde(rename = "m5_reformatting", alias = "M5_Reformatting", alias = "m5")]
    M5_Reformatting,

    /// M6: Renames local variables inside the function body preserving signature and name.
    #[serde(rename = "m6_local_var_rename", alias = "M6_LocalVarRename", alias = "m6")]
    M6_LocalVarRename,

    /// M7: Splits part of a function into a new helper function (`derived_from` relation).
    #[serde(rename = "m7_function_split", alias = "M7_FunctionSplit", alias = "m7")]
    M7_FunctionSplit,

    /// M8: Deletes an old function and simultaneously adds an unrelated function in the same commit.
    #[serde(rename = "m8_decoy_delete_add", alias = "M8_DecoyDeleteAdd", alias = "m8")]
    M8_DecoyDeleteAdd,

    /// M9: Relocates an entire directory or module path.
    #[serde(rename = "m9_directory_move", alias = "M9_DirectoryMove", alias = "m9")]
    M9_DirectoryMove,

    /// M10: Duplicates a function into a second file while retaining the original (`copied` relation).
    #[serde(rename = "m10_function_copy", alias = "M10_FunctionCopy", alias = "m10")]
    M10_FunctionCopy,
}

impl MutationClass {
    /// Returns a static slice of all 10 mutation classes.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::M1_InPlaceRename,
            Self::M2_CrossFileMove,
            Self::M3_MoveAndRename,
            Self::M4_SignatureChange,
            Self::M5_Reformatting,
            Self::M6_LocalVarRename,
            Self::M7_FunctionSplit,
            Self::M8_DecoyDeleteAdd,
            Self::M9_DirectoryMove,
            Self::M10_FunctionCopy,
        ]
    }

    /// Returns the short code (e.g. "M1", "M2").
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::M1_InPlaceRename => "M1",
            Self::M2_CrossFileMove => "M2",
            Self::M3_MoveAndRename => "M3",
            Self::M4_SignatureChange => "M4",
            Self::M5_Reformatting => "M5",
            Self::M6_LocalVarRename => "M6",
            Self::M7_FunctionSplit => "M7",
            Self::M8_DecoyDeleteAdd => "M8",
            Self::M9_DirectoryMove => "M9",
            Self::M10_FunctionCopy => "M10",
        }
    }

    /// Returns the canonical name of the mutation class.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::M1_InPlaceRename => "In-Place Function Rename",
            Self::M2_CrossFileMove => "Cross-File Function Move",
            Self::M3_MoveAndRename => "Cross-File Move and Rename",
            Self::M4_SignatureChange => "Function Signature Change",
            Self::M5_Reformatting => "Reformatting and Docstrings",
            Self::M6_LocalVarRename => "Local Variable Rename",
            Self::M7_FunctionSplit => "Function Split (Helper Extraction)",
            Self::M8_DecoyDeleteAdd => "Decoy Delete and Unrelated Add",
            Self::M9_DirectoryMove => "Directory / Module Move",
            Self::M10_FunctionCopy => "Function Copy / Duplication",
        }
    }

    /// Returns a detailed description of the mutation class.
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::M1_InPlaceRename => {
                "Renames a function within the same file without altering implementation body"
            }
            Self::M2_CrossFileMove => {
                "Moves a function verbatim to another file without modifying signature or body"
            }
            Self::M3_MoveAndRename => {
                "Moves a function to a new file location and simultaneously renames it"
            }
            Self::M4_SignatureChange => {
                "Modifies function parameter list (adds, removes, or reorders arguments)"
            }
            Self::M5_Reformatting => {
                "Applies cosmetic formatting, whitespace, and docstring modifications"
            }
            Self::M6_LocalVarRename => {
                "Renames internal local variables within the function body"
            }
            Self::M7_FunctionSplit => {
                "Extracts an internal block into a dedicated helper function"
            }
            Self::M8_DecoyDeleteAdd => {
                "Deletes an existing function while introducing an unrelated function in the same commit"
            }
            Self::M9_DirectoryMove => "Moves an entire source directory or module tree",
            Self::M10_FunctionCopy => {
                "Copies a function into another file while retaining the original"
            }
        }
    }

    /// Parses a string loosely matching code, name, or snake_case identifier.
    #[must_use]
    pub fn from_str_loose(s: &str) -> Option<Self> {
        let normalized = s.trim().to_lowercase().replace(['-', ' '], "_");
        match normalized.as_str() {
            "m1" | "m1_in_place_rename" | "in_place_rename" | "rename" => {
                Some(Self::M1_InPlaceRename)
            }
            "m2" | "m2_cross_file_move" | "cross_file_move" | "move" => Some(Self::M2_CrossFileMove),
            "m3" | "m3_move_and_rename" | "move_and_rename" => Some(Self::M3_MoveAndRename),
            "m4" | "m4_signature_change" | "signature_change" | "signature" => {
                Some(Self::M4_SignatureChange)
            }
            "m5" | "m5_reformatting" | "reformatting" | "reformat" => Some(Self::M5_Reformatting),
            "m6" | "m6_local_var_rename" | "local_var_rename" | "var_rename" => {
                Some(Self::M6_LocalVarRename)
            }
            "m7" | "m7_function_split" | "function_split" | "split" => Some(Self::M7_FunctionSplit),
            "m8" | "m8_decoy_delete_add" | "decoy_delete_add" | "decoy" => {
                Some(Self::M8_DecoyDeleteAdd)
            }
            "m9" | "m9_directory_move" | "directory_move" | "dir_move" => {
                Some(Self::M9_DirectoryMove)
            }
            "m10" | "m10_function_copy" | "function_copy" | "copy" => Some(Self::M10_FunctionCopy),
            _ => None,
        }
    }
}

impl fmt::Display for MutationClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.code(), self.name())
    }
}

impl FromStr for MutationClass {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str_loose(s).ok_or_else(|| {
            format!(
                "Unknown mutation class '{s}'. Valid values: M1..M10, or descriptive names."
            )
        })
    }
}
