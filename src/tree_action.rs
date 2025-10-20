use crate::{Repository, get_work_dir, load_workspace};
use colored::Colorize;
use std::{collections::HashMap, ffi::OsStr, fmt::Display, path::PathBuf};

#[derive(Default, Debug)]
struct Directory {
    childs: HashMap<String, Self>,
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
    ) -> std::fmt::Result {
        if self.childs.is_empty() {
            return Ok(());
        }

        let final_i = self.childs.len() - 1;

        for (i, (name, directory)) in self.childs.iter().enumerate() {
            let is_final = i == final_i;

            let a = format!(
                "{prefix}{}{}",
                if is_final { "└── " } else { "├── " },
                name.to_string().blue(),
            );

            writeln!(
                f,
                "{a:40}{}",
                if let Some(r) = &directory.repository {
                    format!(
                        " -- {}",
                        r.remote_url
                            .clone()
                            .unwrap_or_else(|| r.name.clone())
                            .green()
                    )
                } else {
                    "".to_string()
                }
            )?;

            directory.display(
                f,
                format!("{prefix}{}", if is_final { "    " } else { "│   " }),
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
        writeln!(f, "{}", format!("{}", self.path.display()).blue())?;
        self.directory.display(f, "".to_string())
    }
}

pub fn tree() -> i32 {
    let mut root = RootDirectory::new(get_work_dir());

    let repositories = load_workspace().0;

    for repository in repositories {
        root.insert(repository);
    }

    println!("{root}");

    0
}
