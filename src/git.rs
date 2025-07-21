//! Module for retrieving git information.
use chrono::{DateTime, Utc};
use colored::{ColoredString, Colorize};
use std::{
    collections::HashMap, error::Error, fmt::Display, fs::metadata, path::Path,
    process::Command, str::Chars, time::SystemTime,
};

#[derive(Debug)]
pub enum Status {
    Unmodified,
    Modified,
    FileTypeChanged,
    Added,
    Deleted,
    Renamed,
    Copied,
    Updated,
    Untracked,
    Ignored,
}

impl Status {
    fn from_chars(chars: &mut Chars) -> Self {
        match chars.next().unwrap() {
            '.' => Self::Unmodified,
            'M' => Self::Modified,
            'T' => Self::FileTypeChanged,
            'A' => Self::Added,
            'D' => Self::Deleted,
            'R' => Self::Renamed,
            'C' => Self::Copied,
            'U' => Self::Updated,
            character => panic!("Unexpected character {character}"),
        }
    }

    pub fn is_modified(&self) -> bool {
        matches!(self, Self::Unmodified)
    }

    pub fn is_deleted(&self) -> bool {
        matches!(self, Self::Deleted)
    }

    pub fn is_unmerged(&self) -> bool {
        matches!(self, Self::Updated)
    }

    pub fn is_untracked(&self) -> bool {
        matches!(self, Self::Untracked)
    }

    fn to_colored_string(&self, staged: bool) -> ColoredString {
        let ret = match self {
            Self::Unmodified => " ",
            Self::Modified => "M",
            Self::FileTypeChanged => "T",
            Self::Added => "A",
            Self::Deleted => "D",
            Self::Renamed => "R",
            Self::Copied => "C",
            Self::Updated => "U",
            Self::Untracked => "?",
            Self::Ignored => "!",
        };

        if matches!(self, Self::Unmodified | Self::Untracked | Self::Ignored) {
            ret.white()
        } else if matches!(self, Self::Updated) {
            ret.red()
        } else if staged {
            ret.green()
        } else {
            ret.red()
        }
    }
}

// A 4 character field describing the submodule state.
// "N..." when the entry is not a submodule.
// "S<c><m><u>" when the entry is a submodule.
// <c> is "C" if the commit changed; otherwise ".".
// <m> is "M" if it has tracked changes; otherwise ".".
// <u> is "U" if there are untracked changes; otherwise ".".
#[derive(Debug)]
pub enum SubmoduleStatus {
    NotASubmodule,
    Submodule {
        commit_changed: bool,
        tracked_changed: bool,
        has_untracked: bool,
    },
    Untracked,
    Ignored,
}

impl SubmoduleStatus {
    fn from_chars(chars: &mut Chars) -> Self {
        let is_submodule = chars.next().unwrap();
        let commit_changed = chars.next().unwrap();
        let tracked_changed = chars.next().unwrap();
        let has_untracked = chars.next().unwrap();

        assert!(matches!(is_submodule, 'N' | 'S'));
        assert!(matches!(commit_changed, 'C' | '.'));
        assert!(matches!(tracked_changed, 'M' | '.'));
        assert!(matches!(has_untracked, 'U' | '.'));

        if is_submodule == 'N' {
            Self::NotASubmodule
        } else if is_submodule == 'S' {
            Self::Submodule {
                commit_changed: commit_changed != '.',
                tracked_changed: tracked_changed != '.',
                has_untracked: has_untracked != '.',
            }
        } else {
            panic!("Unexpected format for submodule changed")
        }
    }

    pub fn is_submodule(&self) -> bool {
        !matches!(self, Self::NotASubmodule)
    }

    pub fn has_commit_changed(&self) -> bool {
        if let Self::Submodule { commit_changed, .. } = self {
            *commit_changed
        } else {
            false
        }
    }

    pub fn has_tracked_changed(&self) -> bool {
        if let Self::Submodule {
            tracked_changed, ..
        } = self
        {
            *tracked_changed
        } else {
            false
        }
    }

    pub fn has_untracked(&self) -> bool {
        if let Self::Submodule { has_untracked, .. } = self {
            *has_untracked
        } else {
            false
        }
    }

    fn to_colored_string(&self) -> ColoredString {
        match self {
            Self::NotASubmodule => "    ".blue(),
            Self::Untracked => "????".blue(),
            Self::Ignored => "!!!!".blue(),
            &Self::Submodule {
                commit_changed,
                tracked_changed,
                has_untracked,
            } => format!(
                "S{}{}{}",
                if commit_changed { "C" } else { " " },
                if tracked_changed { "M" } else { " " },
                if has_untracked { "?" } else { " " },
            )
            .blue(),
        }
    }
}

#[derive(Debug)]
pub struct ItemStatus {
    pub staged: Status,
    pub unstaged: Status,
    pub submodule_status: SubmoduleStatus,
    // In case the file is renamed or copied, the orig_path variable will
    // contain the path the file was before (respectively originally).
    pub orig_path: Option<String>,
    pub path: String,
}

impl Display for ItemStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{} {} ",
            self.staged.to_colored_string(true),
            self.unstaged.to_colored_string(false),
            self.submodule_status.to_colored_string(),
        )?;
        if let Some(orig_path) = &self.orig_path {
            write!(f, "{orig_path} -> ")?;
        }
        write!(f, "{}", self.path)
    }
}

enum ParseOutput {
    BranchInfo(String, String),
    ItemStatus(ItemStatus),
}

fn parse_line(line: &str) -> ParseOutput {
    let mut chars = line.chars();

    // Common part for all entries.
    let entry_type = chars.next().unwrap();
    assert!(matches!(chars.next(), Some(' ')));

    if entry_type == '#' {
        let key = chars.by_ref().take_while(|c| c != &' ').collect::<String>();
        let value = chars.collect::<String>();
        ParseOutput::BranchInfo(key, value)
    } else if entry_type == '?' {
        ParseOutput::ItemStatus(ItemStatus {
            staged: Status::Untracked,
            unstaged: Status::Untracked,
            submodule_status: SubmoduleStatus::Untracked,
            path: chars.collect::<String>(),
            orig_path: None,
        })
    } else if entry_type == '!' {
        ParseOutput::ItemStatus(ItemStatus {
            staged: Status::Ignored,
            unstaged: Status::Ignored,
            submodule_status: SubmoduleStatus::Ignored,
            path: chars.collect::<String>(),
            orig_path: None,
        })
    } else {
        let staged = Status::from_chars(&mut chars);
        let unstaged = Status::from_chars(&mut chars);
        assert!(matches!(chars.next(), Some(' ')));
        let submodule_status = SubmoduleStatus::from_chars(&mut chars);
        // <mH>        The octal file mode in HEAD.
        // or
        // <m1>        The octal file mode in stage 1.
        let mut i = 0;
        // if entry_type is 'u', skip fields:
        // <m1>        The octal file mode in stage 1.
        // <m2>        The octal file mode in stage 2.
        // <m3>        The octal file mode in stage 3.
        // <mW>        The octal file mode in the worktree.
        // <h1>        The object name in stage 1.
        // <h2>        The object name in stage 2.
        // <h3>        The object name in stage 3.
        // Otherwise skip fields:
        // <XY>        A 2 character field containing the staged and
        //             unstaged XY values described in the short format,
        //             with unchanged indicated by a "." rather than
        //             a space.
        // <sub>       A 4 character field describing the submodule state.
        //             "N..." when the entry is not a submodule.
        //             "S<c><m><u>" when the entry is a submodule.
        //             <c> is "C" if the commit changed; otherwise ".".
        //             <m> is "M" if it has tracked changes; otherwise ".".
        //             <u> is "U" if there are untracked changes; otherwise ".".
        // <mH>        The octal file mode in HEAD.
        // <mI>        The octal file mode in the index.
        // <mW>        The octal file mode in the worktree.
        // <hH>        The object name in HEAD.
        // <hI>        The object name in the index.
        // and skip one more field if entry_type is '2'.
        // <X><score>  The rename or copy score (denoting the percentage
        //             of similarity between the source and target of the
        //             move or copy). For example "R100" or "C75".
        let skip = match entry_type {
            'u' => 7,
            '1' => 5,
            '2' => 6,
            _ => panic!("Unexpected entry type"),
        };

        let mut chars = chars.skip_while(|c| {
            if c == &' ' {
                i += 1;
                i <= skip
            } else {
                true
            }
        });
        let (path, orig_path) = match entry_type {
            '1' => (chars.collect::<String>(), None),
            '2' => {
                let path = chars
                    .by_ref()
                    .take_while(|c| c != &'\t')
                    .collect::<String>();
                let orig_path = chars.collect::<String>();
                (path, Some(orig_path))
            }
            'u' => {
                // <h2>        The object name in stage 2.
                let chars = chars.take_while(|c| c != &' ');
                // <h3>        The object name in stage 3.
                let chars = chars.take_while(|c| c != &' ');
                // <path>      The pathname.
                (chars.collect::<String>(), None)
            }
            _ => panic!("Unexpected entry type"),
        };
        ParseOutput::ItemStatus(ItemStatus {
            staged,
            unstaged,
            submodule_status,
            path,
            orig_path,
        })
    }
}

#[derive(Debug)]
pub struct UpstreamInfo {
    pub name: String,
    pub ahead: u32,
    pub behind: u32,
}

impl Display for UpstreamInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}  {}{}  {}",
            self.ahead.to_string().green(),
            "".green(),
            self.behind.to_string().red(),
            "".red(),
            self.name.cyan()
        )
    }
}

#[derive(Debug)]
pub struct BranchInfo {
    pub oid: String,
    pub head: String,
    pub upstream: Option<UpstreamInfo>,
}

impl BranchInfo {
    fn from_raw(raw: HashMap<String, String>) -> Self {
        let oid = raw.get("branch.oid").expect("Missing oid key").clone();
        let head = raw.get("branch.head").expect("Missing head key").clone();
        let upstream = if let Some(name) = raw.get("branch.upstream").cloned() {
            let (ahead, behind) = raw
                .get("branch.ab")
                .expect("Missing ab key")
                .split_once(" -")
                .expect("Invalid ab value");
            Some(UpstreamInfo {
                name,
                ahead: ahead.parse().unwrap(),
                behind: behind.parse().unwrap(),
            })
        } else {
            None
        };

        Self {
            oid,
            head,
            upstream,
        }
    }
}

#[derive(Debug)]
pub struct GitStatus {
    pub branch: BranchInfo,
    pub status: Vec<ItemStatus>,
}

pub fn git_status_porcelain(
    repo_path: &str,
) -> Result<GitStatus, Box<dyn Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("status")
        .arg("--porcelain=v2")
        .arg("--branch")
        .output()?;

    let output = String::from_utf8(output.stdout)?;
    let mut branch_raw = HashMap::<String, String>::new();
    let mut status = Vec::<ItemStatus>::new();

    for line in output.lines() {
        match parse_line(line) {
            ParseOutput::BranchInfo(key, value) => {
                branch_raw.insert(key, value);
            }
            ParseOutput::ItemStatus(item_status) => {
                status.push(item_status);
            }
        }
    }

    Ok(GitStatus {
        branch: BranchInfo::from_raw(branch_raw),
        status,
    })
}

pub fn get_git_dir(repo_path: &str) -> Option<String> {
    let mut ret = String::from_utf8(
        Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .arg("rev-parse")
            .arg("--git-dir")
            .output()
            .ok()?
            .stdout,
    )
    .ok()?;

    // Pop new line character.
    ret.pop();

    Some(ret)
}

pub fn get_last_fetched(git_dir: &String) -> Option<DateTime<Utc>> {
    let mut fetch_head = Path::new(git_dir).to_path_buf();
    fetch_head.push("FETCH_HEAD");

    DateTime::from_timestamp_millis(
        metadata(fetch_head.as_path())
            .ok()?
            .modified()
            .ok()?
            .duration_since(SystemTime::UNIX_EPOCH)
            .ok()?
            .as_millis()
            .try_into()
            .unwrap(),
    )
}
