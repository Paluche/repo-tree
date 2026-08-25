//! Interact with jujutsu workspaces.

use std::error::Error;
use std::fs::create_dir_all;
use std::path::Path;
use std::path::PathBuf;
use std::path::absolute;
use std::process::Command;

use which::which;

use crate::error::CommandError;

/// Representation of a Jujutsu workspace.
pub struct JujutsuWorkspace {
    /// Name of the workspace.
    pub name: String,

    /// Path where the workspace is located.
    pub path: PathBuf,
}

/// List all workspaces associated with the specified repository.
pub fn list_workspaces(
    repo_path: &Path,
) -> Result<Vec<JujutsuWorkspace>, Box<dyn Error>> {
    let mut command = Command::new(which("jj").expect("Jujutsu not installed"));
    let output = command
        .arg("--ignore-working-copy")
        .arg("--repository")
        .arg(repo_path)
        .arg("workspace")
        .arg("list")
        .arg("--template")
        .arg(r#"name ++ "\t" ++ "\n""#)
        .output()?;

    if !output.status.success() {
        return Err(Box::new(CommandError::new(command, output)));
    }

    Ok(String::from_utf8(output.stdout)?
        .split("\n")
        .filter(|l| !l.is_empty())
        .map(|l| {
            let mut parts = l.split("\t");
            let name = parts.next().unwrap().to_string();
            let path =
                absolute(repo_path.join(PathBuf::from(parts.next().unwrap())))
                    .unwrap();

            JujutsuWorkspace { name, path }
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

    let mut command = Command::new(which("jj").expect("Jujutsu not installed"));
    let output = command
        .arg("--ignore-working-copy")
        .arg("--repository")
        .arg(repo_path)
        .arg("workspace")
        .arg("add")
        .arg("--name")
        .arg(name)
        .arg(destination)
        .output()?;

    if !output.status.success() {
        return Err(Box::new(CommandError::new(command, output)));
    }

    Ok(())
}
