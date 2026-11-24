//! Module for retrieving git information.
use std::collections::HashMap;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt::Display;
use std::path::Path;
use std::path::PathBuf;

use jiff::Timestamp;
use pathdiff::diff_paths;
use strum::EnumIter;
use strum::IntoEnumIterator;

use super::command::GitCommand;
use crate::config::GitStatusCommandConfig;
use crate::config::GitSummarizeStatusConfig;
use crate::config::GitSummarizeSubmoduleStatusConfig;
use crate::error::GitStatusParseError;
use crate::utils::get_last_modified;

#[derive(Debug, Hash, PartialEq, Eq, EnumIter)]
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
    /// representing an entry status to a value of the `EntryStatus` enum.
    fn from_char(character: char) -> Option<Self> {
        Some(match character {
            '.' => Self::Unmodified,
            'M' => Self::Modified,
            'T' => Self::FileTypeChanged,
            'A' => Self::Added,
            'D' => Self::Deleted,
            'R' => Self::Renamed,
            'C' => Self::Copied,
            'U' => Self::Updated,
            _ => return None,
        })
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
#[derive(Debug, PartialEq)]
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
    /// Parse the 4 characters field from the porcelain v2 git status output,
    /// representing a submodule status to a value of the `SubmoduleStatus`
    /// enum.
    fn from_field(field: &str) -> Option<Self> {
        let flag = |character: char, set: char| match character {
            '.' => Some(false),
            c if c == set => Some(true),
            _ => None,
        };

        match field.chars().collect::<Vec<char>>()[..] {
            ['N', '.', '.', '.'] => Some(Self::NotASubmodule),
            ['S', commit_changed, tracked_changed, has_untracked] => {
                Some(Self::Submodule {
                    commit_changed: flag(commit_changed, 'C')?,
                    tracked_changed: flag(tracked_changed, 'M')?,
                    has_untracked: flag(has_untracked, 'U')?,
                })
            }
            _ => None,
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
#[derive(Debug, PartialEq)]
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

            diff_paths(&ret, cwd).unwrap_or(ret).display().to_string()
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
#[derive(Debug, PartialEq)]
enum ParseOutput {
    /// The parsed line was representing branch information.
    BranchInfo(String, String),
    /// The parsed line is representing stash information.
    StashInfo(u32),
    /// The parsed line is representing an item status.
    ItemStatus(ItemStatus),
    /// The parsed line is a header this parser does not know about, it is
    /// ignored so that new headers of future git versions do not break the
    /// parsing.
    OtherHeader,
}

/// Parse the XY field of a line from the porcelain v2 output of git status.
fn parse_xy(
    line: &str,
    field: &str,
) -> Result<(EntryStatus, EntryStatus), GitStatusParseError> {
    let mut chars = field.chars();
    match (
        chars.next().and_then(EntryStatus::from_char),
        chars.next().and_then(EntryStatus::from_char),
        chars.next(),
    ) {
        (Some(staged), Some(unstaged), None) => Ok((staged, unstaged)),
        _ => Err(GitStatusParseError::new(line, "invalid XY field")),
    }
}

/// Parse a line from the porcelain v2 output of git status.
fn parse_line(line: &str) -> Result<ParseOutput, GitStatusParseError> {
    let Some((entry_type, rest)) = line.split_once(' ') else {
        return Err(GitStatusParseError::new(line, "missing entry type"));
    };

    match entry_type {
        "#" => {
            let (key, value) = rest.split_once(' ').unwrap_or((rest, ""));
            if key.starts_with("branch.") {
                Ok(ParseOutput::BranchInfo(key.to_string(), value.to_string()))
            } else if key == "stash" {
                value.parse().map(ParseOutput::StashInfo).map_err(|_| {
                    GitStatusParseError::new(line, "invalid stash count")
                })
            } else {
                Ok(ParseOutput::OtherHeader)
            }
        }
        "?" => Ok(ParseOutput::ItemStatus(ItemStatus {
            staged: EntryStatus::Untracked,
            unstaged: EntryStatus::Untracked,
            submodule_status: SubmoduleStatus::Untracked,
            path: rest.to_string(),
            orig_path: None,
        })),
        "!" => Ok(ParseOutput::ItemStatus(ItemStatus {
            staged: EntryStatus::Ignored,
            unstaged: EntryStatus::Ignored,
            submodule_status: SubmoduleStatus::Ignored,
            path: rest.to_string(),
            orig_path: None,
        })),
        "1" | "2" | "u" => {
            // Number of fields after the entry type, the last one being the
            // path, which may contain spaces:
            // 1 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <path>
            // 2 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <X><score>
            // <path><sep><origPath> u <XY> <sub> <m1> <m2> <m3> <mW> <h1> <h2>
            // <h3> <path>
            let nb_fields = match entry_type {
                "1" => 8,
                "2" => 9,
                _ => 10,
            };
            let fields: Vec<&str> = rest.splitn(nb_fields, ' ').collect();
            if fields.len() != nb_fields || fields[nb_fields - 1].is_empty() {
                return Err(GitStatusParseError::new(line, "missing fields"));
            }
            let (staged, unstaged) = parse_xy(line, fields[0])?;
            let submodule_status = SubmoduleStatus::from_field(fields[1])
                .ok_or_else(|| {
                    GitStatusParseError::new(line, "invalid submodule field")
                })?;
            let (path, orig_path) = if entry_type == "2" {
                let (path, orig_path) = fields[nb_fields - 1]
                    .split_once('\t')
                    .ok_or_else(|| {
                        GitStatusParseError::new(line, "missing original path")
                    })?;
                (path.to_string(), Some(orig_path.to_string()))
            } else {
                (fields[nb_fields - 1].to_string(), None)
            };

            Ok(ParseOutput::ItemStatus(ItemStatus {
                staged,
                unstaged,
                submodule_status,
                orig_path,
                path,
            }))
        }
        _ => Err(GitStatusParseError::new(line, "unknown entry type")),
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

        if *map.get(&EntryStatus::Updated).unwrap() != 0 {
            write!(f, "{}", self.config.conflicted)?;
        }

        Ok(())
    }
}

/// Information related to an upstream.
#[derive(Debug, PartialEq)]
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
    Ok(GitCommand::repo(repo_path)?
        .arg("branch")
        .arg(format!("--points-at={pointing_at}"))
        .arg("--color=never")
        .output_lines_fallible()?
        .iter()
        .filter_map(|line| {
            // Each line is prefixed by 2 characters: "* " for the current
            // branch, "+ " for a branch checked out in another worktree.
            let name = line.get(2..)?;
            if name.starts_with("(HEAD detached ") {
                None
            } else {
                Some(name.to_string())
            }
        })
        .collect())
}

/// Get the names of all tags which points at a specific commit.
fn get_tags_pointing_at<S>(
    repo_path: &S,
    pointing_at: &str,
) -> Result<Vec<String>, Box<dyn Error>>
where
    S: AsRef<OsStr>,
{
    GitCommand::repo(repo_path)?
        .arg("tag")
        .arg(format!("--points-at={pointing_at}"))
        .output_lines_fallible()
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

impl UpstreamInfo {
    /// Parse the upstream information from the branch headers of the
    /// porcelain v2 output of git status. Returns `None` if the current
    /// branch has no upstream.
    fn parse(
        branch_info: &HashMap<String, String>,
    ) -> Result<Option<Self>, GitStatusParseError> {
        let Some(name) = branch_info.get("branch.upstream").cloned() else {
            return Ok(None);
        };

        // Git omits the ahead / behind header when the upstream is gone.
        let Some(ab) = branch_info.get("branch.ab") else {
            return Ok(Some(Self {
                name,
                ahead: 0,
                behind: 0,
                gone: true,
            }));
        };

        let invalid = || {
            GitStatusParseError::new(
                &format!("# branch.ab {ab}"),
                "invalid value",
            )
        };
        let (ahead, behind) = ab
            .strip_prefix('+')
            .and_then(|ab| ab.split_once(" -"))
            .ok_or_else(invalid)?;

        Ok(Some(Self {
            name,
            ahead: ahead.parse().map_err(|_| invalid())?,
            behind: behind.parse().map_err(|_| invalid())?,
            gone: false,
        }))
    }
}

impl HeadInfo {
    /// Initialize a new `HeadInfo` struct.
    fn new<S>(
        branch_info: &HashMap<String, String>,
        repo_path: &S,
    ) -> Result<Self, Box<dyn Error>>
    where
        S: AsRef<OsStr>,
    {
        let oid = branch_info
            .get("branch.oid")
            .map_or("unknown".to_string(), ToOwned::to_owned);

        let branch = branch_info
            .get("branch.head")
            .map_or("unknown".to_string(), ToOwned::to_owned);
        let upstream = UpstreamInfo::parse(branch_info)?;

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
fn get_last_fetched(git_dir: &Path) -> Option<Timestamp> {
    get_last_modified(&git_dir.join("FETCH_HEAD")).ok()
}

#[derive(Debug, PartialEq)]
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
    pub last_fetched: Option<Timestamp>,
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

        for item in &self.status {
            // Untracked and ignored entries are not staged, count them once.
            if !matches!(
                item.staged,
                EntryStatus::Untracked | EntryStatus::Ignored
            ) {
                staged.increment(&item.staged);
            }
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
    let git_dir = PathBuf::from(
        GitCommand::repo(repo_path)?
            .arg("rev-parse")
            .arg("--absolute-git-dir")
            .output_lines()?
            .pop()
            .ok_or_else(|| {
                GitStatusParseError::new("", "git rev-parse printed nothing")
            })?,
    );

    let lines = GitCommand::repo(repo_path)?
        .arg("status")
        .arg("--show-stash")
        .arg("--porcelain=v2")
        .arg("--branch")
        .output_lines()?;

    let mut branch_raw = HashMap::<String, String>::new();
    let mut nb_stash = 0;
    let mut status = Vec::new();

    for line in lines {
        match parse_line(&line)? {
            ParseOutput::BranchInfo(key, value) => {
                branch_raw.insert(key, value);
            }
            ParseOutput::StashInfo(n) => nb_stash = n,
            ParseOutput::ItemStatus(item_status) => {
                status.push(item_status);
            }
            ParseOutput::OtherHeader => (),
        }
    }
    let last_fetched = get_last_fetched(&git_dir);
    let ongoing_operations = get_ongoing_operations(&git_dir);

    Ok(GitStatus {
        head: HeadInfo::new(&branch_raw, repo_path)?,
        nb_stash,
        status,
        last_fetched,
        ongoing_operations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::Color;
    use crate::colors::ColoredText;
    use crate::test_utils::no_colors;

    /// Parse `line`, which must be an item status line.
    fn item(line: &str) -> ItemStatus {
        match parse_line(line) {
            Ok(ParseOutput::ItemStatus(item)) => item,
            other => panic!("{line}: unexpected {other:?}"),
        }
    }

    /// Hash of an object, as found in the porcelain v2 output.
    const H: &str = "0123456789abcdef0123456789abcdef01234567";

    #[test]
    fn parse_headers() {
        for (line, expected) in [
            (
                format!("# branch.oid {H}"),
                ParseOutput::BranchInfo("branch.oid".into(), H.into()),
            ),
            (
                "# branch.oid (initial)".into(),
                ParseOutput::BranchInfo(
                    "branch.oid".into(),
                    "(initial)".into(),
                ),
            ),
            (
                "# branch.head (detached)".into(),
                ParseOutput::BranchInfo(
                    "branch.head".into(),
                    "(detached)".into(),
                ),
            ),
            (
                "# branch.upstream origin/main".into(),
                ParseOutput::BranchInfo(
                    "branch.upstream".into(),
                    "origin/main".into(),
                ),
            ),
            (
                "# branch.ab +3 -0".into(),
                ParseOutput::BranchInfo("branch.ab".into(), "+3 -0".into()),
            ),
            ("# stash 2".into(), ParseOutput::StashInfo(2)),
            ("# future.header value".into(), ParseOutput::OtherHeader),
        ] {
            assert_eq!(parse_line(&line).expect("valid line"), expected);
        }
    }

    #[test]
    fn parse_untracked_and_ignored() {
        let untracked = item("? new dir/file name");
        assert_eq!(untracked.path, "new dir/file name");
        assert_eq!(untracked.unstaged, EntryStatus::Untracked);
        assert_eq!(untracked.submodule_status, SubmoduleStatus::Untracked);

        let ignored = item("! target");
        assert_eq!(ignored.path, "target");
        assert_eq!(ignored.unstaged, EntryStatus::Ignored);
        assert_eq!(ignored.submodule_status, SubmoduleStatus::Ignored);
    }

    #[test]
    fn parse_ordinary_change() {
        let entry = item(&format!(
            "1 .M N... 100644 100644 100644 {H} {H} src/a b.rs"
        ));

        assert_eq!(
            entry,
            ItemStatus {
                staged: EntryStatus::Unmodified,
                unstaged: EntryStatus::Modified,
                submodule_status: SubmoduleStatus::NotASubmodule,
                orig_path: None,
                path: "src/a b.rs".into(),
            }
        );
    }

    #[test]
    fn parse_every_entry_status() {
        for (xy, staged, unstaged) in [
            ("A.", EntryStatus::Added, EntryStatus::Unmodified),
            ("MD", EntryStatus::Modified, EntryStatus::Deleted),
            ("T.", EntryStatus::FileTypeChanged, EntryStatus::Unmodified),
            ("D.", EntryStatus::Deleted, EntryStatus::Unmodified),
        ] {
            let entry =
                item(&format!("1 {xy} N... 100644 100644 0 {H} {H} file"));

            assert_eq!((entry.staged, entry.unstaged), (staged, unstaged));
        }
    }

    #[test]
    fn parse_submodule_change() {
        let entry =
            item(&format!("1 .M SCM. 160000 160000 160000 {H} {H} libs/sub"));

        assert_eq!(
            entry.submodule_status,
            SubmoduleStatus::Submodule {
                commit_changed: true,
                tracked_changed: true,
                has_untracked: false,
            }
        );
        assert_eq!(entry.path, "libs/sub");
    }

    #[test]
    fn parse_rename() {
        let entry = item(&format!(
            "2 R. N... 100644 100644 100644 {H} {H} R100 new name\told name"
        ));

        assert_eq!(entry.staged, EntryStatus::Renamed);
        assert_eq!(entry.path, "new name");
        assert_eq!(entry.orig_path.as_deref(), Some("old name"));
    }

    #[test]
    fn parse_copy() {
        let entry =
            item(&format!("2 C. N... 100644 100644 100644 {H} {H} C75 b\ta"));

        assert_eq!(entry.staged, EntryStatus::Copied);
        assert_eq!(
            (entry.path.as_str(), entry.orig_path.as_deref()),
            ("b", Some("a"))
        );
    }

    #[test]
    fn parse_unmerged_path_with_spaces() {
        let entry = item(&format!(
            "u UU N... 100644 100644 100644 100644 {H} {H} {H} con flict.txt"
        ));

        assert_eq!(entry.staged, EntryStatus::Updated);
        assert_eq!(entry.unstaged, EntryStatus::Updated);
        assert_eq!(entry.path, "con flict.txt");
    }

    #[test]
    fn parse_malformed_lines_fails() {
        for line in [
            String::new(),
            "#".into(),
            "x".into(),
            "# stash many".into(),
            "z something".into(),
            "1 .M N... 100644".into(),
            format!("1 .X N... 100644 100644 100644 {H} {H} file"),
            format!("1 .MM N... 100644 100644 100644 {H} {H} file"),
            format!("1 .M X... 100644 100644 100644 {H} {H} file"),
            format!("1 .M SXM. 100644 100644 100644 {H} {H} file"),
            format!("1 .M N.. 100644 100644 100644 {H} {H} file"),
            format!("2 R. N... 100644 100644 100644 {H} {H} R100 no-tab"),
            format!("1 .M N... 100644 100644 100644 {H} {H} "),
        ] {
            assert!(parse_line(&line).is_err(), "{line:?}");
        }
    }

    #[test]
    fn parse_error_tells_the_line() {
        let error = parse_line("z something").expect_err("unknown type");

        assert_eq!(
            error.to_string(),
            "Unexpected git status output, unknown entry type: \"z something\""
        );
    }

    /// Branch headers from `(key, value)` pairs.
    fn headers(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    #[test]
    fn upstream_parsing() {
        assert_eq!(UpstreamInfo::parse(&headers(&[])).expect("valid"), None);
        assert_eq!(
            UpstreamInfo::parse(&headers(&[
                ("branch.upstream", "origin/main"),
                ("branch.ab", "+3 -12"),
            ]))
            .expect("valid"),
            Some(UpstreamInfo {
                name: "origin/main".into(),
                ahead: 3,
                behind: 12,
                gone: false,
            })
        );
        assert_eq!(
            UpstreamInfo::parse(&headers(&[("branch.upstream", "origin/x")]))
                .expect("valid"),
            Some(UpstreamInfo {
                name: "origin/x".into(),
                ahead: 0,
                behind: 0,
                gone: true,
            })
        );
    }

    #[test]
    fn malformed_upstream_fails() {
        for ab in ["3 -1", "+3", "+a -1", "+1 -b", ""] {
            assert!(
                UpstreamInfo::parse(&headers(&[
                    ("branch.upstream", "origin/main"),
                    ("branch.ab", ab),
                ]))
                .is_err(),
                "{ab:?}"
            );
        }
    }

    /// Git status made of the items parsed from `lines`.
    fn git_status(lines: &[String]) -> GitStatus {
        GitStatus {
            head: HeadInfo {
                oid: H.into(),
                branch: "main".into(),
                upstream: None,
                branches: vec![],
                tags: vec![],
            },
            nb_stash: 0,
            status: lines.iter().map(|line| item(line)).collect(),
            last_fetched: None,
            ongoing_operations: vec![],
        }
    }

    /// Configuration of the summary with ASCII characters.
    fn summary_config() -> GitSummarizeStatusConfig {
        GitSummarizeStatusConfig {
            added: 'A',
            modified: 'M',
            file_type_changed: 'T',
            copied: 'C',
            renamed: 'R',
            deleted: 'D',
            untracked: '?',
            conflicted: '!',
        }
    }

    #[test]
    fn short_status_summarizes_staged_and_unstaged() {
        let status = git_status(&[
            format!("1 A. N... 0 100644 100644 {H} {H} added"),
            format!("1 MD N... 100644 100644 0 {H} {H} both"),
            format!("2 R. N... 100644 100644 100644 {H} {H} R90 b\ta"),
            format!("1 .M SC.. 160000 160000 160000 {H} {H} sub"),
            format!("u UU N... 100644 100644 100644 100644 {H} {H} {H} c"),
            "? untracked".into(),
        ]);

        let (staged, unstaged, submodules) = status.short_status();
        let config = summary_config();

        assert_eq!(staged.display(&config).to_string(), "AMR!");
        assert_eq!(unstaged.display(&config).to_string(), "MD?!");
        let submodule_config = GitSummarizeSubmoduleStatusConfig::default();
        let _colors = no_colors();
        assert_eq!(
            submodules.display(&submodule_config).to_string(),
            submodule_config.commit_changed.to_string()
        );
    }

    #[test]
    fn short_status_of_clean_repository_is_empty() {
        let (staged, unstaged, submodules) = git_status(&[]).short_status();
        let _colors = no_colors();

        assert_eq!(staged.display(&summary_config()).to_string(), "");
        assert_eq!(unstaged.display(&summary_config()).to_string(), "");
        assert_eq!(
            submodules
                .display(&GitSummarizeSubmoduleStatusConfig::default())
                .to_string(),
            ""
        );
    }

    #[test]
    fn item_status_display() {
        let _colors = no_colors();
        let config = GitStatusCommandConfig::default();
        let root = Path::new("/repo");

        let modified =
            item(&format!("1 .M N... 100644 100644 100644 {H} {H} src/a.rs"));
        assert_eq!(
            modified.display(&config, root, root, None).to_string(),
            " M      src/a.rs"
        );
        assert_eq!(
            modified
                .display(&config, Path::new("/repo/src"), root, None)
                .to_string(),
            " M      a.rs"
        );

        let renamed = item(&format!(
            "2 R. N... 100644 100644 100644 {H} {H} R100 new\told"
        ));
        assert_eq!(
            renamed
                .display(&config, Path::new("/repo/doc"), root, Some("sub"))
                .to_string(),
            "R       ../sub/old -> ../sub/new"
        );

        let submodule =
            item(&format!("1 .M S.MU 160000 160000 160000 {H} {H} sub"));
        assert_eq!(
            submodule.display(&config, root, root, None).to_string(),
            " M S M? sub"
        );

        assert_eq!(
            item("? new").display(&config, root, root, None).to_string(),
            "?? ???? new"
        );
    }

    #[test]
    fn item_status_display_with_relative_cwd_keeps_path() {
        let _colors = no_colors();
        let config = GitStatusCommandConfig::default();
        let entry =
            item(&format!("1 .M N... 100644 100644 100644 {H} {H} a.rs"));

        assert_eq!(
            entry
                .display(
                    &config,
                    Path::new("relative"),
                    Path::new("/repo"),
                    None
                )
                .to_string(),
            " M      /repo/a.rs"
        );
    }

    #[test]
    fn upstream_display() {
        let _colors = no_colors();
        let mut config = GitStatusCommandConfig::default();
        config.upstream.ahead = ColoredText::new("^", Color::none());
        config.upstream.behind = ColoredText::new("v", Color::none());
        let upstream = UpstreamInfo {
            name: "origin/main".into(),
            ahead: 2,
            behind: 5,
            gone: false,
        };

        assert_eq!(
            upstream.display(&config).to_string(),
            "2^  5v  origin/main"
        );
    }

    /// Ongoing operations detected in a git directory containing `files`
    /// and `dirs`.
    fn ongoing_operations(files: &[&str], dirs: &[&str]) -> Vec<GitOperation> {
        let git_dir = tempfile::tempdir().expect("cannot create directory");
        for dir in dirs {
            std::fs::create_dir_all(git_dir.path().join(dir))
                .expect("cannot create directory");
        }
        for file in files {
            std::fs::write(git_dir.path().join(file), "")
                .expect("cannot create file");
        }
        get_ongoing_operations(git_dir.path())
    }

    #[test]
    fn detect_ongoing_operations() {
        use GitOperation::*;

        assert_eq!(ongoing_operations(&[], &[]), vec![]);
        assert_eq!(ongoing_operations(&[], &["rebase-merge"]), vec![Rebase]);
        assert_eq!(
            ongoing_operations(&["rebase-apply/rebasing"], &["rebase-apply"]),
            vec![Rebase]
        );
        assert_eq!(ongoing_operations(&[], &["rebase-apply"]), vec![AM]);
        assert_eq!(
            ongoing_operations(&["CHERRY_PICK_HEAD"], &["sequencer"]),
            vec![CherryPick]
        );
        assert_eq!(
            ongoing_operations(&["CHERRY_PICK_HEAD"], &[]),
            vec![CherryPick]
        );
        assert_eq!(ongoing_operations(&["BISECT_START"], &[]), vec![Bisect]);
        assert_eq!(ongoing_operations(&["MERGE_HEAD"], &[]), vec![Merge]);
        assert_eq!(ongoing_operations(&["REVERT_HEAD"], &[]), vec![Revert]);
        // Directories named as the marker files are not markers.
        assert_eq!(ongoing_operations(&[], &["MERGE_HEAD"]), vec![]);
    }

    #[test]
    fn git_operation_display() {
        assert_eq!(
            [
                GitOperation::Rebase,
                GitOperation::AM,
                GitOperation::CherryPick,
                GitOperation::Bisect,
                GitOperation::Merge,
                GitOperation::Revert,
            ]
            .map(|operation| operation.to_string()),
            ["rebase", "am", "cherry-pick", "bisect", "merge", "revert"]
        );
    }

    #[test]
    fn last_fetched_from_fetch_head() {
        let git_dir = tempfile::tempdir().expect("cannot create directory");
        assert_eq!(get_last_fetched(git_dir.path()), None);

        std::fs::write(git_dir.path().join("FETCH_HEAD"), "")
            .expect("cannot create file");
        assert!(get_last_fetched(git_dir.path()).is_some());
    }
}
