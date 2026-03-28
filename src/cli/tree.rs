//! Get a tree view of all your repositories in your repo tree. This is inspired
//! by the `tree` command line.
use std::collections::BTreeMap;
use std::fmt::Display;
use std::path::PathBuf;

use clap::Args;
use colored::ColoredString;
use colored::Colorize;

use crate::Config;
use crate::Repositories;
use crate::Repository;

/// Display a tree of your repo_tree.
#[derive(Args, Debug, PartialEq)]
pub struct TreeArgs {}

/// The different states a directory entry in the tree might take.
enum DirState {
    /// Root directory.
    Root,
    /// Sub-directory.
    SubDir,
    /// Last sub-directory to be list within a parent directory.
    FinalSubDir,
}

impl DirState {
    /// Prefix to display a new directory entry.
    fn get_dir_prefix(&self) -> &str {
        match self {
            Self::Root => "",
            Self::SubDir => "├── ",
            Self::FinalSubDir => "└── ",
        }
    }

    /// Prefix to display a sub-content for the last printed directory entry.
    fn get_subdir_prefix(&self) -> &str {
        match self {
            Self::Root => "",
            Self::SubDir => "│   ",
            Self::FinalSubDir => "    ",
        }
    }
}

/// Directory representation.
#[derive(Default, Debug)]
struct Directory<'config> {
    /// Childs directories within this directory.
    childs: BTreeMap<String, Self>,
    /// Repository present in this directory.
    repository: Option<&'config Repository<'config>>,
}

impl<'config> Directory<'config> {
    /// Get the components of the repository root path.
    fn get_repo_components(
        config: &'config Config,
        repository: &Repository<'config>,
    ) -> Vec<String> {
        assert!(repository.root.starts_with(&config.repo_tree_dir));

        repository
            .root
            .iter()
            .skip(config.repo_tree_dir.iter().count())
            .map(|os_str| os_str.to_str().unwrap().to_owned())
            .collect()
    }

    /// Create a new Directory and all its childs recursively leading to the
    /// repository.
    fn new<T>(
        mut components: T,
        repository: &'config Repository<'config>,
    ) -> Directory<'config>
    where
        T: Iterator<Item = String>,
    {
        if let Some(child_name) = components.next() {
            let mut childs = BTreeMap::new();
            childs.insert(child_name, Directory::new(components, repository));
            Self {
                childs,
                repository: None,
            }
        } else {
            Self {
                childs: BTreeMap::new(),
                repository: Some(repository),
            }
        }
    }

    /// Internal recursive function to insert a repository.
    fn insert_internal<T>(
        &mut self,
        mut components: T,
        repository: &'config Repository<'config>,
    ) where
        T: Iterator<Item = String>,
    {
        if let Some(child_name) = components.next() {
            if let Some(sub_dir) = self.childs.get_mut(&child_name) {
                sub_dir.insert_internal(components, repository);
            } else {
                self.childs
                    .insert(child_name, Directory::new(components, repository));
            }
        }
    }

    /// Insert a repository. Provide the path directory as an iterator on the
    /// component of the path. This will create all Directory struct linked each
    /// other, so you obtain a tree of Directory struct.
    fn insert(
        &mut self,
        config: &'config Config,
        repository: &'config Repository<'config>,
    ) {
        self.insert_internal(
            Self::get_repo_components(config, repository).into_iter(),
            repository,
        );
    }

    /// Get pretty string representation of the Directory.
    fn display<T: Display>(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        prefix: String,
        name: T,
        dir_state: DirState,
    ) -> std::fmt::Result {
        let mut current = self;
        let mut current_dir = name.to_string();
        while current.childs.len() == 1 {
            current_dir.push('/');
            let (child_name, next) = current.childs.iter().next().unwrap();
            current_dir.push_str(child_name);
            current = next;
        }

        writeln!(
            f,
            "{prefix}{}{}",
            dir_state.get_dir_prefix(),
            current_dir.to_string().blue(),
        )?;

        if let Some(r) = &current.repository {
            let prefix = format!("{prefix}{}", dir_state.get_subdir_prefix(),);
            let submodules = r.submodules().unwrap();
            if let Some(remote_url) = r.id.remote_url.clone() {
                writeln!(
                    f,
                    "{prefix}{}{} {}",
                    if submodules.is_empty() {
                        DirState::FinalSubDir
                    } else {
                        DirState::SubDir
                    }
                    .get_subdir_prefix(),
                    remote_url.green(),
                    r.vcs.short_display(),
                )?;
            }
            if !submodules.is_empty() {
                let final_i = submodules.len() - 1;
                for (i, submodule) in submodules.iter().enumerate() {
                    let dir_state = if i == final_i {
                        DirState::FinalSubDir
                    } else {
                        DirState::SubDir
                    };
                    writeln!(
                        f,
                        "{prefix}{}{}",
                        dir_state.get_dir_prefix(),
                        submodule.sub_path.display()
                    )?;

                    let prefix = format!(
                        "{prefix}{}{}",
                        dir_state.get_subdir_prefix(),
                        DirState::FinalSubDir.get_subdir_prefix()
                    );

                    fn option_to_str(
                        o: Option<String>,
                        color: u8,
                    ) -> ColoredString {
                        o.map_or("None".to_string().red(), |v| {
                            v.ansi_color(color)
                        })
                    }

                    let head_id = option_to_str(
                        submodule.head.map(|o| o.to_string()),
                        88,
                    );
                    writeln!(f, "{prefix}{head_id}")?;
                    let submodule_url = {
                        let config_url = submodule.config_url.clone();
                        let resolved_url = submodule.url.clone();

                        if config_url == resolved_url {
                            option_to_str(config_url, 2).to_string()
                        } else {
                            format!(
                                "{} {} {}",
                                option_to_str(config_url, 8),
                                "=>".bright_black(),
                                option_to_str(resolved_url, 2),
                            )
                        }
                    };
                    writeln!(f, "{prefix}{submodule_url}",)?;
                }
            }
        }

        if current.childs.is_empty() {
            return Ok(());
        }

        let final_i = current.childs.len() - 1;
        for (i, (name, directory)) in current.childs.iter().enumerate() {
            directory.display(
                f,
                format!("{prefix}{}", dir_state.get_subdir_prefix()),
                name,
                if i == final_i {
                    DirState::FinalSubDir
                } else {
                    DirState::SubDir
                },
            )?;
        }

        Ok(())
    }
}

/// Representation of the repo tree root directory.
#[derive(Debug)]
struct RootDirectory<'config> {
    /// Actual path to the repo tree root.
    repo_tree_dir: &'config PathBuf,
    /// Associated Directory struct, head of the Directory struct tree.
    directory: Directory<'config>,
}

impl<'config> RootDirectory<'config> {
    /// Instantiate a RootDirectory.
    fn new(
        config: &'config Config,
        repositories: &'config Repositories<'config>,
    ) -> Self {
        let mut directory: Directory<'config> = Directory::default();

        for repository in repositories.iter() {
            directory.insert(config, repository);
        }

        Self {
            repo_tree_dir: &config.repo_tree_dir,
            directory,
        }
    }
}

impl<'config> Display for RootDirectory<'config> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.directory.display(
            f,
            "".to_string(),
            self.repo_tree_dir.display(),
            DirState::Root,
        )
    }
}

/// Execute the `rt tree` command.
pub fn run(config: &Config, _: TreeArgs) -> i32 {
    println!(
        "{}",
        RootDirectory::new(config, &Repositories::load(config))
    );

    0
}
