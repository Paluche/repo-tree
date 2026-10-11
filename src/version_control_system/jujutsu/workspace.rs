//! Interact with jujutsu workspaces.

use std::error::Error;
use std::fs::create_dir_all;
use std::path::Path;
use std::path::PathBuf;
use std::path::absolute;

use super::super::VcsWorkspace;
use super::command::JujutsuCommand;
use crate::config::Config;
use crate::ui::Ui;

/// List all workspaces associated with the specified repository.
pub fn list_workspaces(
    ui: &Ui<'_>,
    config: &Config,
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
            let path = absolute(PathBuf::from(parts.next().unwrap())).unwrap();
            VcsWorkspace { name, path }
        })
        .filter(|w| {
            if w.path.exists() {
                true
            } else {
                ui.warning(format!(
                    "Deleting unexisting workspace {}{} of repository {}",
                    w.display_name(config),
                    w.path.display(),
                    repo_path.display()
                ));
                match forget_workspace(repo_path, &w.name) {
                    Ok(()) => false,
                    Err(err) => {
                        ui.error(err);
                        true
                    }
                }
            }
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
        panic!(
            "Destination directory {} already exists",
            destination.display()
        ); // XXX Use of panic -> Return Err
    }

    // Create the parent directory where to put the workspace.
    if let Some(parent) = destination.parent() {
        if parent.exists() {
            if !parent.is_file() {
                panic!(
                    "Destination parent directory {} is actually a file",
                    destination.display()
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
    if name == "default" {
        panic!("Attempted to forget default workspace");
    }

    JujutsuCommand::repo(repo_path)?
        .arg("workspace")
        .arg("forget")
        .arg(name)
        .status()
}
