//! Module for retrieving git information.
use crate::url_parsing::parse_repo_url;
use chrono::{DateTime, Utc};
use colored::{ColoredString, Colorize};
use git2::Repository;
use strum::{EnumIter, IntoEnumIterator};

use std::{
    collections::HashMap,
    env,
    error::Error,
    ffi::OsStr,
    fmt::Display,
    fs::metadata,
    path::{Path, PathBuf},
    process::Command,
    str::Chars,
    time::SystemTime,
};

#[derive(Debug, Hash, PartialEq, Eq, EnumIter)]
pub enum EntryStatus {
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

impl EntryStatus {
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

#[derive(Default)]
pub struct SummarizeSubmoduleStatus {
    commit_changed: usize,
    tracked_changed: usize,
    has_untracked: usize,
}

impl SummarizeSubmoduleStatus {
    fn new() -> Self {
        Self {
            commit_changed: 0,
            tracked_changed: 0,
            has_untracked: 0,
        }
    }

    fn increment(&mut self, submodule_status: &SubmoduleStatus) {
        if let &SubmoduleStatus::Submodule {
            commit_changed,
            tracked_changed,
            has_untracked,
        } = submodule_status
        {
            if commit_changed {
                self.commit_changed += 1;
            }
            if tracked_changed {
                self.tracked_changed += 1;
            }
            if has_untracked {
                self.has_untracked += 1;
            }
        }
    }

    pub fn as_string(&self) -> String {
        let mut ret = String::new();
        if self.commit_changed != 0 {
            ret.push('');
        }
        if self.tracked_changed != 0 {
            ret.push('');
        }
        if self.has_untracked != 0 {
            ret.push('')
        }

        ret
    }
}

#[derive(Debug)]
pub struct ItemStatus {
    pub staged: EntryStatus,
    pub unstaged: EntryStatus,
    pub submodule_status: SubmoduleStatus,
    // In case the file is renamed or copied, the orig_path variable will
    // contain the path the file was before (respectively originally).
    pub orig_path: Option<String>,
    pub path: String,
}

impl ItemStatus {
    pub fn display(&self, rel_path: Option<&str>) -> String {
        let mut ret = format!(
            "{}{} {} ",
            self.staged.to_colored_string(true),
            self.unstaged.to_colored_string(false),
            self.submodule_status.to_colored_string(),
        );

        fn format_path(rel_path: Option<&str>, path: &String) -> String {
            if let Some(rel_path) = rel_path {
                format!("{rel_path}/{path}")
            } else {
                path.to_string()
            }
        }

        if let Some(orig_path) = &self.orig_path {
            ret.push_str(&format_path(rel_path, orig_path));
            ret.push_str(" -> ");
        }
        ret.push_str(&format_path(rel_path, &self.path));
        ret
    }
}

enum ParseOutput {
    BranchInfo(String, String),
    StashInfo(u32),
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

        if key.starts_with("branch") {
            ParseOutput::BranchInfo(key, value)
        } else if key == "stash" {
            ParseOutput::StashInfo(value.parse().unwrap())
        } else {
            panic!("")
        }
    } else if entry_type == '?' {
        ParseOutput::ItemStatus(ItemStatus {
            staged: EntryStatus::Untracked,
            unstaged: EntryStatus::Untracked,
            submodule_status: SubmoduleStatus::Untracked,
            path: chars.collect::<String>(),
            orig_path: None,
        })
    } else if entry_type == '!' {
        ParseOutput::ItemStatus(ItemStatus {
            staged: EntryStatus::Ignored,
            unstaged: EntryStatus::Ignored,
            submodule_status: SubmoduleStatus::Ignored,
            path: chars.collect::<String>(),
            orig_path: None,
        })
    } else {
        let staged = EntryStatus::from_chars(&mut chars);
        let unstaged = EntryStatus::from_chars(&mut chars);
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
                true
            } else {
                i <= skip
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

pub struct SummarizeStatus {
    map: HashMap<EntryStatus, usize>,
}

impl SummarizeStatus {
    fn new() -> Self {
        let mut map = HashMap::new();
        EntryStatus::iter().for_each(|status| {
            map.insert(status, 0);
        });
        Self { map }
    }

    fn increment(&mut self, entry_status: &EntryStatus) {
        *self.map.get_mut(entry_status).unwrap() += 1;
    }

    pub fn as_string(&self) -> String {
        let mut ret = String::new();
        if *self.map.get(&EntryStatus::Added).unwrap() != 0 {
            ret.push('');
        }

        if *self.map.get(&EntryStatus::Modified).unwrap() != 0 {
            ret.push('');
        }

        if *self.map.get(&EntryStatus::FileTypeChanged).unwrap() != 0 {
            ret.push('');
        }

        if *self.map.get(&EntryStatus::Copied).unwrap() != 0 {
            ret.push('')
        }

        if *self.map.get(&EntryStatus::Renamed).unwrap() != 0 {
            ret.push('')
        }

        if *self.map.get(&EntryStatus::Deleted).unwrap() != 0 {
            ret.push('')
        }

        if *self.map.get(&EntryStatus::Untracked).unwrap() != 0 {
            ret.push('')
        }

        ret
    }
}

#[derive(Debug)]
pub struct UpstreamInfo {
    /// Name of the upstream branch.
    pub name: String,
    /// Number of commits the local branch is ahead of the upstream one.
    pub ahead: u32,
    /// Number of commits the local branch is behind of the upstream one.
    pub behind: u32,
    /// True if the upstream branch is gone (deleted).
    pub gone: bool,
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

fn get_branches_pointing_at<S>(
    repo_path: &S,
    pointing_at: &str,
) -> Result<Vec<String>, Box<dyn Error>>
where
    S: AsRef<OsStr>,
{
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("branch")
        .arg(format!("--points-at={pointing_at}"))
        .arg("--color=never")
        .output()?;

    let output = String::from_utf8(output.stdout)?;
    let mut ret = Vec::new();

    for line in output.lines() {
        ret.push(line[2..].to_string())
    }

    Ok(ret)
}

fn get_tags_pointing_at<S>(
    repo_path: &S,
    pointing_at: &str,
) -> Result<Vec<String>, Box<dyn Error>>
where
    S: AsRef<OsStr>,
{
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("tag")
        .arg(format!("--points-at={pointing_at}"))
        .output()?;

    let output = String::from_utf8(output.stdout)?;

    Ok(output.lines().map(|s| s.to_string()).collect())
}

#[derive(Debug)]
pub struct HeadInfo {
    /// Object Identifier of the commit the HEAD is at.
    pub oid: String,
    /// Name of the branch the head is following.
    pub branch: String,
    /// Name of the associated upstream branch.
    pub upstream: Option<UpstreamInfo>,
    /// Name of the branches pointing at that head which is not already
    /// specified in the branch attribute.
    pub branches: Vec<String>,
    /// Name of the tags located at that head.
    pub tags: Vec<String>,
}

impl HeadInfo {
    fn new<S>(
        branch_info: HashMap<String, String>,
        repo_path: &S,
    ) -> Result<Self, Box<dyn Error>>
    where
        S: AsRef<OsStr>,
    {
        let oid = branch_info
            .get("branch.oid")
            .expect("Missing oid key")
            .clone();
        let branch = branch_info
            .get("branch.head")
            .expect("Missing head key")
            .clone();
        let upstream =
            if let Some(name) = branch_info.get("branch.upstream").cloned() {
                let (ahead, behind, gone) = if let Some((ahead, behind)) =
                    branch_info
                        .get("branch.ab")
                        .map(|s| s.split_once(" -").expect("Invalid ab value"))
                {
                    (ahead.parse().unwrap(), behind.parse().unwrap(), false)
                } else {
                    (0, 0, true)
                };

                Some(UpstreamInfo {
                    name,
                    ahead,
                    behind,
                    gone,
                })
            } else {
                None
            };

        let mut branches = get_branches_pointing_at(repo_path, "HEAD")?;
        branches.retain(|b| !(b == &branch || b == "(no branch)"));
        let tags = get_tags_pointing_at(repo_path, &oid)?;

        Ok(Self {
            oid,
            branch,
            upstream,
            branches,
            tags,
        })
    }
}

fn get_last_fetched(git_dir: &Path) -> Option<DateTime<Utc>> {
    DateTime::from_timestamp_millis(
        metadata(git_dir.join("FETCH_HEAD"))
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

#[derive(Debug, PartialEq)]
pub enum GitOperation {
    Rebase,
    AM,
    CherryPick,
    Bisect,
    Merge,
    Revert,
}

impl Display for GitOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Rebase => "rebase",
                Self::AM => "am",
                Self::CherryPick => "cherry-pick",
                Self::Bisect => "bisect",
                Self::Merge => "merge",
                Self::Revert => "revert",
            }
        )
    }
}

fn get_ongoing_operations(git_dir: &Path) -> Vec<GitOperation> {
    let mut ret = Vec::new();
    {
        let mut path = git_dir.join("rebase-apply");
        if path.is_dir() {
            path.push("rebasing");
            ret.push(if path.is_file() {
                GitOperation::Rebase
            } else {
                GitOperation::AM
            })
        }
    }

    if git_dir.join("rebase-merge").is_dir() {
        ret.push(GitOperation::Rebase);
    }

    if git_dir.join("sequencer").is_dir() {
        ret.push(GitOperation::CherryPick);
    }

    if !ret.contains(&GitOperation::CherryPick)
        && git_dir.join("CHERRY_PICK_HEAD").is_file()
    {
        ret.push(GitOperation::CherryPick);
    }

    if git_dir.join("BISECT_START").is_file() {
        ret.push(GitOperation::Bisect);
    }

    if git_dir.join("MERGE_HEAD").is_file() {
        ret.push(GitOperation::Merge);
    }

    if git_dir.join("REVERT_HEAD").is_file() {
        ret.push(GitOperation::Revert);
    }

    ret
}

#[derive(Debug)]
pub struct GitStatus {
    pub head: HeadInfo,
    pub nb_stash: u32,
    pub status: Vec<ItemStatus>,
    pub last_fetched: Option<DateTime<Utc>>,
    pub ongoing_operations: Vec<GitOperation>,
}

impl GitStatus {
    pub fn short_status(
        &self,
    ) -> (SummarizeStatus, SummarizeStatus, SummarizeSubmoduleStatus) {
        let mut staged = SummarizeStatus::new();
        let mut unstaged = SummarizeStatus::new();
        let mut submodules = SummarizeSubmoduleStatus::new();

        for item in self.status.iter() {
            staged.increment(&item.staged);
            unstaged.increment(&item.unstaged);
            submodules.increment(&item.submodule_status);
        }

        (staged, unstaged, submodules)
    }
}

fn git_status_internal<S>(
    repo_path: &S,
    git_dir: &Path,
) -> Result<GitStatus, Box<dyn Error>>
where
    S: AsRef<OsStr> + Sized,
{
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("status")
        .arg("--show-stash")
        .arg("--porcelain=v2")
        .arg("--branch")
        .output()?;

    let output = String::from_utf8(output.stdout)?;
    let mut branch_raw = HashMap::<String, String>::new();
    let mut nb_stash = 0;
    let mut status = Vec::new();

    for line in output.lines() {
        match parse_line(line) {
            ParseOutput::BranchInfo(key, value) => {
                branch_raw.insert(key, value);
            }
            ParseOutput::StashInfo(n) => {
                assert_eq!(nb_stash, 0, "Unexpected several stash info");
                nb_stash = n
            }
            ParseOutput::ItemStatus(item_status) => {
                status.push(item_status);
            }
        }
    }
    let last_fetched = get_last_fetched(git_dir);
    let ongoing_operations = get_ongoing_operations(git_dir);

    Ok(GitStatus {
        head: HeadInfo::new(branch_raw, repo_path)?,
        nb_stash,
        status,
        last_fetched,
        ongoing_operations,
    })
}

pub fn git_status<S>(repo_path: &S) -> Result<GitStatus, Box<dyn Error>>
where
    S: AsRef<OsStr> + Sized,
{
    let git_dir = {
        let mut ret = String::from_utf8(
            Command::new("git")
                .arg("-C")
                .arg(repo_path)
                .arg("rev-parse")
                .arg("--absolute-git-dir")
                .output()?
                .stdout,
        )?;

        // Pop new line character.
        ret.pop();
        PathBuf::from(ret)
    };

    git_status_internal(repo_path, &git_dir)
}

pub struct RepoInfo {
    pub forge: Option<String>,
    pub name: String,
    pub in_work_dir: bool,
    pub is_submodule: bool,
    /// None is the repository is a submodule.
    pub repo: Repository,
}

impl RepoInfo {
    pub fn top_level(&self) -> Option<&Path> {
        self.repo.workdir()
    }

    pub fn expected_top_level(&self) -> Option<PathBuf> {
        if self.is_submodule || self.forge.is_none() {
            None
        } else {
            let mut path = PathBuf::from(&env::var("WORK_DIR").unwrap());
            path.push(self.forge.clone().unwrap());
            path.push(&self.name);
            Some(path)
        }
    }

    pub fn status(&self) -> Result<GitStatus, Box<dyn Error>> {
        git_status_internal(
            &self.top_level().expect("Bare git repository"),
            self.repo.path(),
        )
    }
}

fn get_work_dir() -> PathBuf {
    PathBuf::from(
        &env::var("WORK_DIR").expect("Missing WORK_DIR environment variable."),
    )
}

pub fn get_repo_info(
    repo_path: Option<String>,
) -> Result<RepoInfo, Box<dyn Error>> {
    let repo_path = repo_path
        .unwrap_or(String::from(env::current_dir().unwrap().to_str().unwrap()));
    let repo = Repository::discover(repo_path)?;
    let (forge, name) = parse_repo_url(&repo);
    let top_level = repo.workdir();

    let is_submodule = top_level.is_some_and(|value| {
        let mut git_dir = value.to_path_buf();
        git_dir.push(".git");
        git_dir.is_file()
    });

    let in_work_dir = !is_submodule
        && top_level.is_some_and(|v| v.starts_with(get_work_dir()));

    Ok(RepoInfo {
        forge,
        name,
        in_work_dir,
        is_submodule,
        repo,
    })
}
