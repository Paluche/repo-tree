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
    parse_workspace_list(
        &JujutsuCommand::repo(repo_path)?
            .arg("workspace")
            .arg("list")
            .arg("--template")
            .arg(r#"name ++ "\t" ++ root ++ "\n""#)
            .output_lines()?,
    )
}

/// Parse the output of `jj workspace list`, with a template giving a
/// workspace per line, with its name and root separated by a tab.
fn parse_workspace_list(
    lines: &[String],
) -> Result<Vec<VcsWorkspace>, Box<dyn Error>> {
    lines
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
    if destination.exists() {
        return Err(format!(
            "Destination {} of the workspace already exists",
            destination.display()
        )
        .into());
    }

    // Create the parent directory where to put the workspace.
    if let Some(parent) = destination.parent() {
        if parent.exists() && !parent.is_dir() {
            return Err(format!(
                "Destination parent directory {} is not a directory",
                parent.display()
            )
            .into());
        }
        create_dir_all(parent)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_workspaces() {
        assert_eq!(
            parse_workspace_list(&[
                "default\t/tree/dev/example/o/r".to_string(),
                "agent\t/tree/agent/example/o/r with space".to_string(),
            ])
            .expect("valid workspaces"),
            [
                VcsWorkspace {
                    name: "default".into(),
                    path: PathBuf::from("/tree/dev/example/o/r"),
                },
                VcsWorkspace {
                    name: "agent".into(),
                    path: PathBuf::from("/tree/agent/example/o/r with space"),
                },
            ]
        );
    }

    #[test]
    fn parse_malformed_workspace_fails() {
        assert!(parse_workspace_list(&["default /tree".to_string()]).is_err());
    }

    #[test]
    fn add_workspace_to_existing_destination_fails() {
        let dir = tempfile::tempdir().expect("cannot create directory");
        let file = dir.path().join("file");
        std::fs::write(&file, "").expect("cannot create file");

        for destination in [dir.path().to_path_buf(), file.clone()] {
            let error = add_workspace(dir.path(), "agent", &destination)
                .expect_err("existing destination");
            assert!(error.to_string().contains("already exists"), "{error}");
        }

        let error = add_workspace(dir.path(), "agent", &file.join("ws"))
            .expect_err("parent is a file");
        assert!(error.to_string().contains("not a directory"), "{error}");
    }
}
