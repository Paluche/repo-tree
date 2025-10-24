use crate::{Repository, get_work_dir, load_workspace};
use colored::Colorize;
use std::{collections::HashMap, ffi::OsStr, fmt::Display, path::PathBuf};

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
            "{:40}{}",
            format!(
                "{prefix}{}{}",
                dir_state.get_dir_prefix(),
                current_dir.to_string().blue()
            ),
            if let Some(r) = &current.repository {
                format!(
                    " -- {}",
                    r.id.remote_url
                        .clone()
                        .unwrap_or_else(|| r.id.name.clone())
                        .green()
                )
            } else {
                "".to_string()
            }
        )?;

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

pub fn tree() -> i32 {
    let mut root = RootDirectory::new(get_work_dir());

    let repositories = load_workspace().0;

    for repository in repositories {
        root.insert(repository);
    }

    println!("{root}");

    0
}
