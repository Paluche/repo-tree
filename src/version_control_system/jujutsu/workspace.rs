//! Interact with jujutsu workspaces.

use std::error::Error;
use std::fs::create_dir_all;
use std::path::Path;
use std::path::PathBuf;

use super::super::VcsWorkspace;
use super::command::JujutsuCommand;

/// List all workspaces associated with the specified repository.
pub fn list_workspaces(
    repo_path: &Path,
) -> Result<Vec<VcsWorkspace>, Box<dyn Error>> {
    Ok(JujutsuCommand::repo(repo_path)?
        .arg("workspace")
        .arg("list")
        .arg("--template")
        .arg(r#"name ++ "\t" ++ root ++ "\n""#)
        .output_lines()?
        .iter()
        .map(|l| {
            let mut parts = l.split("\t");
            let name = parts.next().unwrap().to_string();
            let path = PathBuf::from(parts.next().unwrap());
            VcsWorkspace { name, path }
        })
        .collect())
}

/// Add a new workspace.
pub fn add_workspace(
    repo_path: &Path,
    name: &str,
    destination: &Path,
) -> Result<(), Box<dyn Error>> {
    if destination.is_dir() {
        panic!("Destination directory already exists"); // XXX Use of panic -> Return Err
    }

    // Create the parent directory where to put the workspace.
    if let Some(parent) = destination.parent() {
        if parent.exists() {
            if !parent.is_file() {
                panic!("Destination parent directory is actually a file"); // XXX Use of panic -> Return Err
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
