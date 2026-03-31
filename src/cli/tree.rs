use std::{collections::BTreeMap, ffi::OsStr, fmt::Display, path::PathBuf};

use clap::Args;
use colored::{ColoredString, Colorize};

use crate::{Config, Repository, load_repositories};

/// Display a tree of your repo_tree.
#[derive(Args, Debug, PartialEq)]
pub struct TreeArgs {}

enum DirState {
    Root,
    SubDir,
    FinalSubDir,
}

impl DirState {
    fn get_dir_prefix(&self) -> &str {
        match self {
            Self::Root => "",
            Self::SubDir => "├── ",
            Self::FinalSubDir => "└── ",
        }
    }

    fn get_subdir_prefix(&self) -> &str {
        match self {
            Self::Root => "",
            Self::SubDir => "│   ",
            Self::FinalSubDir => "    ",
        }
    }
}

#[derive(Default, Debug)]
struct Directory {
    childs: BTreeMap<String, Self>,
    repository: Option<Repository>,
}

impl Directory {
    fn get_child(&mut self, name: &str) -> &mut Directory {
        self.childs.entry(name.to_string()).or_default()
    }

    fn insert<'a, T>(&mut self, mut dirs: T, repository: Repository)
    where
        T: Iterator<Item = &'a OsStr>,
    {
        if let Some(dir) = dirs.next() {
            let next = self.get_child(dir.to_str().unwrap());
            next.insert(dirs, repository);
        } else {
            self.repository = Some(repository);
        }
    }

    fn display(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        prefix: String,
        name: &str,
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

#[derive(Debug)]
struct RootDirectory {
    path: PathBuf,
    directory: Directory,
}

impl RootDirectory {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            directory: Directory::default(),
        }
    }

    fn insert(&mut self, repository: Repository) {
        let path = repository.root.clone();
        assert!(path.starts_with(&self.path));
        self.directory
            .insert(path.iter().skip(self.path.iter().count()), repository)
    }
}

impl Display for RootDirectory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.directory.display(
            f,
            "".to_string(),
            self.path.to_str().unwrap(),
            DirState::Root,
        )
    }
}

pub fn run(config: &Config, _: TreeArgs) -> i32 {
    let repositories = load_repositories(config);
    let mut root = RootDirectory::new(config.repo_tree_dir.clone());

    for repository in repositories {
        root.insert(repository);
    }

    println!("{root}");

    0
}
