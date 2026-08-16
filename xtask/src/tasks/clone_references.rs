use crate::util::{project_root, run_cmd};
use anyhow::{bail, Result};
use std::process::Command;

struct RefRepo {
    name: &'static str,
    url: &'static str,
    description: &'static str,
}

const REPOSITORIES: &[RefRepo] = &[
    RefRepo {
        name: "codanna",
        url: "https://github.com/bartolli/codanna.git",
        description: "Local code intelligence tool and persistent MCP server in Rust",
    },
    RefRepo {
        name: "code-maat",
        url: "https://github.com/adamtornhill/code-maat.git",
        description: "VCS temporal coupling and behavioral code analysis tool",
    },
    RefRepo {
        name: "tree-sitter-python",
        url: "https://github.com/tree-sitter/tree-sitter-python.git",
        description: "Tree-sitter Python grammar reference",
    },
    RefRepo {
        name: "pydriller",
        url: "https://github.com/ishepard/pydriller.git",
        description: "Python framework for mining software repositories and commit analysis",
    },
];

pub fn run_clone_references(filter_repo: Option<&str>) -> Result<()> {
    let root = project_root();
    let ref_dir = root.join("reference_repos");
    if !ref_dir.exists() {
        std::fs::create_dir_all(&ref_dir)?;
    }

    let selected: Vec<&RefRepo> = match filter_repo {
        Some(name) => {
            let matches: Vec<&RefRepo> = REPOSITORIES.iter().filter(|r| r.name == name).collect();
            if matches.is_empty() {
                bail!(
                    "Unknown reference repository '{name}'. Available: {}",
                    REPOSITORIES
                        .iter()
                        .map(|r| r.name)
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            matches
        }
        None => REPOSITORIES.iter().collect(),
    };

    println!(
        "Synchronizing {} reference repository(ies)...",
        selected.len()
    );

    for repo in selected {
        let dest = ref_dir.join(repo.name);
        if dest.exists() && dest.join(".git").exists() {
            println!(
                "==> '{}' already exists at {}. Updating shallow fetch...",
                repo.name,
                dest.display()
            );
            let mut cmd = Command::new("git");
            cmd.args(["fetch", "--depth", "1"]).current_dir(&dest);
            let _ = run_cmd(&mut cmd, &format!("git fetch {}", repo.name));
        } else {
            println!(
                "==> Cloning '{}' ({}) from {}...",
                repo.name, repo.description, repo.url
            );
            let mut cmd = Command::new("git");
            cmd.args(["clone", "--depth", "1", repo.url, dest.to_str().unwrap()]);
            run_cmd(&mut cmd, &format!("git clone {}", repo.name))?;
        }
    }

    println!(
        "Reference repositories synchronized in {}",
        ref_dir.display()
    );
    Ok(())
}
