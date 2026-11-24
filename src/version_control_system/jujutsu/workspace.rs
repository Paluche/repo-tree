//! Interact with jujutsu workspaces.

use std::error::Error;
use std::fs::create_dir_all;
use std::path::Path;
use std::path::PathBuf;

use super::super::VcsWorkspace;
use super::command::JujutsuCommand;

/// Name of the workspace Jujutsu creates with the repository.
pub const DEFAULT_WORKSPACE: &str = "default";

/// List all workspaces associated with the specified repository.
pub fn list_workspaces(
    repo_path: &Path,
) -> Result<Vec<VcsWorkspace>, Box<dyn Error>> {
    JujutsuCommand::repo(repo_path)?
        .arg("workspace")
        .arg("list")
        .arg("--template")
        .arg(r#"name ++ "\t" ++ root ++ "\n""#)
        .output_lines()?
        .iter()
        .map(|line| {
            let (name, path) = line.split_once('\t').ok_or_else(|| {
                format!("Unexpected line in jj workspace list output: {line:?}")
            })?;
            Ok(VcsWorkspace {
                name: name.to_string(),
                path: PathBuf::from(path),
            })
        })
        .collect()
}

/// Add a new workspace.
pub fn add_workspace(
    repo_path: &Path,
    name: &str,
    destination: &Path,
) -> Result<(), Box<dyn Error>> {
    if destination.is_dir() {
        panic!(
            "Destination directory {} already exists",
            destination.display()
        ); // XXX Use of panic -> Return Err
    }

    // Create the parent directory where to put the workspace.
    if let Some(parent) = destination.parent() {
        if parent.exists() {
            if parent.is_file() {
                panic!(
                    "Destination parent directory {} is actually a file",
                    parent.display()
                ); // XXX Use of panic -> Return Err
            }
        } else {
            create_dir_all(parent)?;
        }
    }

    JujutsuCommand::repo_with_snapshot(repo_path)?
        .arg("workspace")
        .arg("add")
        .arg("--name")
        .arg(name)
        .arg(destination)
        .status()
}

/// Forget a Jujutsu workspace.
/// This will not delete the workspace repository files and folder.
/// A workspace which repository files and folder are deleted, will not be
/// forgotten automatically by Jujutsu.
pub fn forget_workspace(
    repo_path: &Path,
    name: &str,
) -> Result<(), Box<dyn Error>> {
    if name == DEFAULT_WORKSPACE {
        return Err(format!(
            "Refusing to forget the default workspace of {}",
            repo_path.display()
        )
        .into());
    }

    JujutsuCommand::repo(repo_path)?
        .arg("workspace")
        .arg("forget")
        .arg(name)
        .status()
}
