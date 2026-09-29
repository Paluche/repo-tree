//! Module for retrieving git information.
use std::collections::HashMap;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt::Display;
use std::path::Path;
use std::path::PathBuf;
use std::str::Chars;

use chrono::DateTime;
use chrono::Utc;
use pathdiff::diff_paths;
use strum::EnumIter;
use strum::IntoEnumIterator;

use super::new_git_command;
use crate::config::GitStatusCommandConfig;
use crate::config::GitSummarizeStatusConfig;
use crate::config::GitSummarizeSubmoduleStatusConfig;
use crate::utils::get_last_modified;

#[derive(Hash, PartialEq, Eq, EnumIter)]
/// All the different entry status you can have in the porcelain v2 output of
/// git status.
pub enum EntryStatus {
    /// The entry is unmodified '.'.
    Unmodified,
    /// The entry is modified 'M'.
    Modified,
    /// File type of the entry has changed 'T'.
    FileTypeChanged,
    /// Entry is newly added 'A'.
    Added,
    /// Entry has been deleted 'D'.
    Deleted,
    /// Entry has been renamed 'R'.
    Renamed,
    /// Entry has been copied 'C'.
    Copied,
    /// Entry has been updated 'U'.
    Updated,
    /// Entry is untracked.
    Untracked,
    /// Entry is ignored.
    Ignored,
}

impl EntryStatus {
    /// Associate a character from the porcelain v2 git status output,
    /// representing an entry status to a value of the EntryStatus enum.
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

    /// Obtain a struct which implements the Display trait for EntryStatus.
    pub fn display<'entry_status, 'config>(
        &'entry_status self,
        git_status_config: &'config GitStatusCommandConfig,
        staged: bool,
    ) -> EntryStatusDisplay<'entry_status, 'config> {
        EntryStatusDisplay {
            entry_status: self,
            git_status_config,
            staged,
        }
    }
}

/// Implement the Display for the EntryStatus struct.
pub struct EntryStatusDisplay<'entry_status, 'config> {
    /// EntryStatus to display.
    entry_status: &'entry_status EntryStatus,
    /// Configuration dictating how to display the content of the entry status.
    git_status_config: &'config GitStatusCommandConfig,
    /// Set to true if the entry status represents a staged entry.
    staged: bool,
}

impl<'entry_status, 'config> Display
    for EntryStatusDisplay<'entry_status, 'config>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let entry_config = &self.git_status_config.entry;
        let char_repr = match self.entry_status {
            EntryStatus::Unmodified => entry_config.unmodified,
            EntryStatus::Modified => entry_config.modified,
            EntryStatus::FileTypeChanged => entry_config.file_type_changed,
            EntryStatus::Added => entry_config.added,
            EntryStatus::Deleted => entry_config.deleted,
            EntryStatus::Renamed => entry_config.renamed,
            EntryStatus::Copied => entry_config.copied,
            EntryStatus::Updated => entry_config.updated,
            EntryStatus::Untracked => entry_config.untracked,
            EntryStatus::Ignored => entry_config.ignored,
        };

        let color = match self.entry_status {
            EntryStatus::Unmodified
            | EntryStatus::Untracked
            | EntryStatus::Ignored => &self.git_status_config.unimportant,
            EntryStatus::Updated => &self.git_status_config.updated,
            _ => {
                if self.staged {
                    &self.git_status_config.staged
                } else {
                    &self.git_status_config.unstaged
                }
            }
        };

        write!(f, "{}", color.colorize(char_repr))
    }
}

/// Parsing result of the submodule status in the porcelain v2 output of git
/// status. Which is initially a 4 character field.
pub enum SubmoduleStatus {
    /// The entry is not a submodule "N...".
    NotASubmodule,
    /// The entry is a submodule. "S<c><m><u>":
    /// - <c> is "C" if the commit changed; otherwise ".".
    /// - <m> is "M" if it has tracked changes; otherwise ".".
    /// - <u> is "U" if there are untracked changes; otherwise ".".
    Submodule {
        /// The commit the subdmodule is at is different from the one set for
        /// it in the main repository.
        commit_changed: bool,
        /// Tracked files within the submodule have changed.
        tracked_changed: bool,
        /// Submodule contains untracked files.
        has_untracked: bool,
    },
    /// The entry is untracked.
    Untracked,
    /// The entry is ignored.
    Ignored,
}

impl SubmoduleStatus {
    /// Parse the 4 characters from the porcelain v2 git status output,
    /// representing a submodule status to a value of the SubmoduleStatus enum.
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

    /// Obtain a struct which implements the Display trait for SubmoduleStatus.
    pub fn display<'submodule_status, 'config>(
        &'submodule_status self,
        git_status_config: &'config GitStatusCommandConfig,
    ) -> SubmoduleStatusDisplay<'submodule_status, 'config> {
        SubmoduleStatusDisplay {
            submodule_status: self,
            git_status_config,
        }
    }
}

/// Implement the Display for the SubmoduleStatus struct.
pub struct SubmoduleStatusDisplay<'submodule_status, 'config> {
    /// SubmoduleStatus to display.
    submodule_status: &'submodule_status SubmoduleStatus,
    /// Configuration dictating how to display the content of the submodule
    /// status.
    git_status_config: &'config GitStatusCommandConfig,
}

impl<'submodule_status, 'config> Display
    for SubmoduleStatusDisplay<'submodule_status, 'config>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self.submodule_status {
            SubmoduleStatus::NotASubmodule => "    ".to_string(),
            SubmoduleStatus::Untracked => {
                format!("{0}{0}{0}{0}", self.git_status_config.entry.untracked)
            }

            SubmoduleStatus::Ignored => {
                format!("{0}{0}{0}{0}", self.git_status_config.entry.ignored)
            }
            &SubmoduleStatus::Submodule {
                commit_changed,
                tracked_changed,
                has_untracked,
            } => format!(
                "S{}{}{}",
                if commit_changed { "C" } else { " " },
                if tracked_changed { "M" } else { " " },
                if has_untracked { "?" } else { " " },
            ),
        };

        write!(f, "{}", self.git_status_config.submodule.colorize(text))
    }
}

#[derive(Default)]
/// Summary stats for a submodule status.
pub struct SummarizeSubmoduleStatus {
    /// Number of submodules with commits changed.
    commit_changed: usize,
    /// Number of submodules with tracked changed.
    tracked_changed: usize,
    /// Number of submodules with untracked files.
    has_untracked: usize,
}

impl SummarizeSubmoduleStatus {
    /// Increment the internal counters based on the content of the provided
    /// submodule status.
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

    /// Obtain a struct which implements the Display trait for
    /// SummarizeSubmoduleStatus.
    pub fn display<'submodule_status, 'config>(
        &'submodule_status self,
        config: &'config GitSummarizeSubmoduleStatusConfig,
    ) -> SummarizeSubmoduleStatusDisplay<'submodule_status, 'config> {
        SummarizeSubmoduleStatusDisplay {
            summarize_submodule_status: self,
            config,
        }
    }
}

/// Implement the Display for the SummarizeSubmoduleStatus struct.
pub struct SummarizeSubmoduleStatusDisplay<'submodule_status, 'config> {
    /// SummarizeSubmoduleStatus to display.
    summarize_submodule_status: &'submodule_status SummarizeSubmoduleStatus,
    /// Configuration dictating how to display the content of the submodule
    /// status.
    config: &'config GitSummarizeSubmoduleStatusConfig,
}

impl<'submodule_status, 'config> Display
    for SummarizeSubmoduleStatusDisplay<'submodule_status, 'config>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let color = &self.config.color;
        if self.summarize_submodule_status.commit_changed != 0 {
            write!(f, "{}", color.colorize(self.config.commit_changed))?;
        }
        if self.summarize_submodule_status.tracked_changed != 0 {
            write!(f, "{}", color.colorize(self.config.tracked_changed))?;
        }
        if self.summarize_submodule_status.has_untracked != 0 {
            write!(f, "{}", color.colorize(self.config.has_untracked))?;
        }

        Ok(())
    }
}

/// Status of an item.
pub struct ItemStatus {
    /// Staged entry status.
    pub staged: EntryStatus,
    /// Unstaged entry status.
    pub unstaged: EntryStatus,
    /// Submodule entry status.
    pub submodule_status: SubmoduleStatus,
    /// In case the entry is renamed or copied, you will find here the path
    /// where the file was before, respectively where is the source file is.
    pub orig_path: Option<String>,
    /// Path to the entry.
    pub path: String,
}

impl ItemStatus {
    /// Obtain a struct which implements the Display trait for ItemStatus.
    pub fn display<'item_status, 'config, 'cwd, 'repo_root, 'rel_path>(
        &'item_status self,
        git_status_config: &'config GitStatusCommandConfig,
        cwd: &'cwd Path,
        repo_root: &'repo_root Path,
        rel_path: Option<&'rel_path str>,
    ) -> ItemStatusDisplay<'item_status, 'config, 'cwd, 'repo_root, 'rel_path>
    {
        ItemStatusDisplay {
            item_status: self,
            git_status_config,
            cwd,
            repo_root,
            rel_path,
        }
    }
}

/// Implement the Display for the ItemStatus struct.
pub struct ItemStatusDisplay<'item_status, 'config, 'cwd, 'repo_root, 'rel_path>
{
    /// ItemStatus to display.
    item_status: &'item_status ItemStatus,
    /// Configuration dictating how to display the content of the item status.
    git_status_config: &'config GitStatusCommandConfig,
    /// Current working directory.
    cwd: &'cwd Path,
    /// Absolute path to the root of the main repository.
    repo_root: &'repo_root Path,
    /// Relative path to the root of the repository, if it is a submodule.
    rel_path: Option<&'rel_path str>,
}

impl<'item_status, 'config, 'cwd, 'repo_root, 'rel_path> Display
    for ItemStatusDisplay<'item_status, 'config, 'cwd, 'repo_root, 'rel_path>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{} {} ",
            self.item_status
                .staged
                .display(self.git_status_config, true),
            self.item_status
                .unstaged
                .display(self.git_status_config, false),
            self.item_status
                .submodule_status
                .display(self.git_status_config),
        )?;

        fn format_path(
            cwd: &Path,
            repo_root: &Path,
            rel_path: Option<&str>,
            path: &String,
        ) -> String {
            let mut ret = PathBuf::from(repo_root);
            if let Some(rel_path) = rel_path {
                ret.push(rel_path);
            }
            ret.push(path);

            diff_paths(ret, cwd).unwrap().display().to_string()
        }

        if let Some(orig_path) = &self.item_status.orig_path {
            write!(
                f,
                "{} -> ",
                format_path(self.cwd, self.repo_root, self.rel_path, orig_path)
            )?;
        }
        write!(
            f,
            "{}",
            format_path(
                self.cwd,
                self.repo_root,
                self.rel_path,
                &self.item_status.path
            )
        )
    }
}

/// Enum representing the different parse result of a line from the porcelain v2
/// output of git status.
enum ParseOutput {
    /// The parsed line was representing branch information.
    BranchInfo(String, String),
    /// The parsed line is representing stash information.
    StashInfo(u32),
    /// The parsed line is representing an item status.
    ItemStatus(ItemStatus),
}

/// Parse a line from the porcelain v2 output of git status.
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
        // If entry_type is 'u', skip fields:
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

/// Summarization of the status.
pub struct SummarizeStatus {
    /// Total number of entries for each entry type detected in the status.
    map: HashMap<EntryStatus, usize>,
}

impl SummarizeStatus {
    /// Initialize a new SummarizeStatus struct.
    fn new() -> Self {
        let mut map = HashMap::new();
        EntryStatus::iter().for_each(|status| {
            map.insert(status, 0);
        });
        Self { map }
    }

    /// Increment the internal counters based on the provided EntryStatus value.
    fn increment(&mut self, entry_status: &EntryStatus) {
        *self.map.get_mut(entry_status).unwrap() += 1;
    }

    /// Obtain a struct which implements the Display trait for SummarizeStatus.
    pub fn display<'summarize_status, 'config>(
        &'summarize_status self,
        config: &'config GitSummarizeStatusConfig,
    ) -> SummarizeStatusDisplay<'summarize_status, 'config> {
        SummarizeStatusDisplay {
            summarize_status: self,
            config,
        }
    }
}

/// Implement the Display trait for SummarizeStatus.
pub struct SummarizeStatusDisplay<'summarize_status, 'config> {
    /// SummarizeStatus to display.
    summarize_status: &'summarize_status SummarizeStatus,
    /// Configuration dictating how to display the content of the item status.
    config: &'config GitSummarizeStatusConfig,
}

impl<'summarize_status, 'config> Display
    for SummarizeStatusDisplay<'summarize_status, 'config>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let map = &self.summarize_status.map;
        if *map.get(&EntryStatus::Added).unwrap() != 0 {
            write!(f, "{}", self.config.added)?;
        }

        if *map.get(&EntryStatus::Modified).unwrap() != 0 {
            write!(f, "{}", self.config.modified)?;
        }

        if *map.get(&EntryStatus::FileTypeChanged).unwrap() != 0 {
            write!(f, "{}", self.config.file_type_changed)?;
        }

        if *map.get(&EntryStatus::Copied).unwrap() != 0 {
            write!(f, "{}", self.config.copied)?;
        }

        if *map.get(&EntryStatus::Renamed).unwrap() != 0 {
            write!(f, "{}", self.config.renamed)?;
        }

        if *map.get(&EntryStatus::Deleted).unwrap() != 0 {
            write!(f, "{}", self.config.deleted)?;
        }

        if *map.get(&EntryStatus::Untracked).unwrap() != 0 {
            write!(f, "{}", self.config.untracked)?;
        }

        Ok(())
    }
}

/// Information related to an upstream.
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

impl UpstreamInfo {
    /// Obtain a struct which implements the Display trait for SubmoduleStatus.
    pub fn display<'upstream_info, 'config>(
        &'upstream_info self,
        git_status_config: &'config GitStatusCommandConfig,
    ) -> UpstreamInfoDisplay<'upstream_info, 'config> {
        UpstreamInfoDisplay {
            upstream_info: self,
            git_status_config,
        }
    }
}

/// Implement the Display for the UpstreamInfo struct.
pub struct UpstreamInfoDisplay<'upstream_info, 'config> {
    /// UpstreamInfo to display.
    upstream_info: &'upstream_info UpstreamInfo,
    /// Configuration dictating how to display the content of the item status.
    git_status_config: &'config GitStatusCommandConfig,
}

impl<'upstream_info, 'config> Display
    for UpstreamInfoDisplay<'upstream_info, 'config>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let upstream_config = &self.git_status_config.upstream;
        write!(
            f,
            "{}{}  {}{}  {}",
            upstream_config
                .ahead
                .color
                .colorize(self.upstream_info.ahead),
            upstream_config.ahead,
            upstream_config
                .behind
                .color
                .colorize(self.upstream_info.behind),
            upstream_config.behind,
            upstream_config.name.colorize(&self.upstream_info.name)
        )
    }
}

/// Get the names of all branches which points at a specific commit.
fn get_branches_pointing_at<S>(
    repo_path: &S,
    pointing_at: &str,
) -> Result<Vec<String>, Box<dyn Error>>
where
    S: AsRef<OsStr>,
{
    let output = new_git_command()
        .arg("-C")
        .arg(repo_path)
        .arg("branch")
        .arg(format!("--points-at={pointing_at}"))
        .arg("--color=never")
        .output()?;

    let output = String::from_utf8(output.stdout)?;
    let mut ret = Vec::new();

    for line in output.lines() {
        if line[2..].starts_with("(HEAD detached ") {
            continue;
        }
        ret.push(line[2..].to_string())
    }

    Ok(ret)
}

/// Get the names of all tags which points at a specific commit.
fn get_tags_pointing_at<S>(
    repo_path: &S,
    pointing_at: &str,
) -> Result<Vec<String>, Box<dyn Error>>
where
    S: AsRef<OsStr>,
{
    let output = new_git_command()
        .arg("-C")
        .arg(repo_path)
        .arg("tag")
        .arg(format!("--points-at={pointing_at}"))
        .output()?;

    let output = String::from_utf8(output.stdout)?;

    Ok(output.lines().map(|s| s.to_string()).collect())
}

/// Information related to the HEAD.
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
    /// Initialize a new HeadInfo struct.
    fn new<S>(
        branch_info: HashMap<String, String>,
        repo_path: &S,
    ) -> Result<Self, Box<dyn Error>>
    where
        S: AsRef<OsStr>,
    {
        let oid = branch_info
            .get("branch.oid")
            .map_or("unknown".to_string(), |s| s.to_owned());

        let branch = branch_info
            .get("branch.head")
            .map_or("unknown".to_string(), |s| s.to_owned());
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

/// Find out when the repository was last fetched.
fn get_last_fetched(git_dir: &Path) -> Option<DateTime<Utc>> {
    get_last_modified(&git_dir.join("FETCH_HEAD")).ok()
}

#[derive(PartialEq)]
/// The different Git operation you might be stuck in a middle of its execution
/// to resolve a conflict.
pub enum GitOperation {
    /// git rebase.
    Rebase,
    /// git am.
    AM,
    /// git cherry-pick.
    CherryPick,
    /// git bisect.
    Bisect,
    /// git merge.
    Merge,
    /// git revert.
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

/// Get all currently ongoing operations.
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

/// Parsed result of a `git status --porcelain=v2` command.
pub struct GitStatus {
    /// Information about the current HEAD.
    pub head: HeadInfo,
    /// Number of entries in the stash.
    pub nb_stash: u32,
    /// Status of the items.
    pub status: Vec<ItemStatus>,
    /// Date the repository has been synchronized with the remote.
    pub last_fetched: Option<DateTime<Utc>>,
    /// List on currently ongoing operations.
    pub ongoing_operations: Vec<GitOperation>,
}

impl GitStatus {
    /// Obtain a short summarized status.
    pub fn short_status(
        &self,
    ) -> (SummarizeStatus, SummarizeStatus, SummarizeSubmoduleStatus) {
        let mut staged = SummarizeStatus::new();
        let mut unstaged = SummarizeStatus::new();
        let mut submodules = SummarizeSubmoduleStatus::default();

        for item in self.status.iter() {
            staged.increment(&item.staged);
            unstaged.increment(&item.unstaged);
            submodules.increment(&item.submodule_status);
        }

        (staged, unstaged, submodules)
    }
}

/// Get the status of the repository.
pub fn status<S>(repo_path: &S) -> Result<GitStatus, Box<dyn Error>>
where
    S: AsRef<OsStr> + Sized,
{
    let git_dir = {
        let mut ret = String::from_utf8(
            new_git_command()
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

    let output = new_git_command()
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
    let last_fetched = get_last_fetched(&git_dir);
    let ongoing_operations = get_ongoing_operations(&git_dir);

    Ok(GitStatus {
        head: HeadInfo::new(branch_raw, repo_path)?,
        nb_stash,
        status,
        last_fetched,
        ongoing_operations,
    })
}
