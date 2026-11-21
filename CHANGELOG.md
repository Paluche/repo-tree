# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Release highlights

- Introduction of the `tree-spaces`, you have three available ones being:

  - `dev`: Active (non-archived) repositories with a remote.
  - `local`: Repositories without a remote.
  - `archive`: Archive repositories with a remote.
  - `agent`: Workspace tree-space, dedicated for agent workspaces.

- Introduction of the "workspace tree-spaces" allowing to have several copies of
  a same repository on different trees.

### Breaking changes

- The introduction of tree-spaces re-organize the repo-tree. To migrate from a
  previous version you must do the following commands:
  ```bash
  mkdir ${REPO_TREE_DIR}/dev
  mv ${REPO_TREE_DIR}/* ${REPO_TREE_DIR}/dev
  mv ${REPO_TREE_DIR}/dev/local ${REPO_TREE_DIR}
  rt refresh-cacge
  ```
- The host and name filters arguments used in `rt list` or `rt todo *` commands
  are now glob. Previously we were filtering on exact match for the _host_,
  which is still compatible with the glob syntax, but the _name_ filtering was
  matching on names starting with the provided pattern. The behavior for the
  _name_ filter is now fully changed you need to edit your script to add a `*`
  at the end of your name filter to obtain the previous behavior.
- Command `rt repo remote` changed behavior. This command now is a user
  interface to get a remote of the repository, if there is several choices, then
  we ask the user for which one they want.
- **jj prompt**: The `prompt.jj.bookmark.remote` color is replaced by the
  `prompt.jj.bookmark.untracked` table, with a `name` and a `remote` color to
  color the bookmark name and its remote separately.
- **jj prompt**: The `prompt.jj.tags` configuration is replaced by
  `prompt.jj.tag`, with the same structure as `prompt.jj.bookmark`. The
  `prompt.jj.bookmark.none` configuration is removed: nothing is displayed
  anymore when there is no bookmark nor tag to show.

### Deprecations

None

### New features

- We are no more depending on the `jj-lib` crate. All `jj` interaction are done
  by calling the CLI.
- **jj prompt**: Display the list of locally deleted bookmarks pending to be either
  pushed and deleted from the remote too or to be forgotten locally about.
- **jj prompt**: Make the bookmark and tags prompt even more configurable. Might
  break local configuration.
- **jj prompt**: Display the tags with the same logic as the bookmarks: tags
  on the parent, current and descendant commits, local, tracked and remote only
  tags, and locally deleted tags pending some action.
- Add new command `rt workspace add` and `rt workspace rm` to manage our
  repositories workspace in the different workspace tree-space.
- **identity**: Configure the identities to use in the repositories: a
  `default` identity and identities overriding part of it for a host, a
  directory of a host or a single repository.
- **identity**: `rt identity show` displays the identity of a repository and
  the identities it is made of.
- **identity**: `rt identity apply` writes the identity in the Jujutsu and Git
  configuration of a repository, or of all of them with `--all`, and
  `rt identity show` lists the differences.
- **clone**: `rt clone` sets the identity of the cloned repository. Jujutsu
  repositories get it for the clone itself, so that the first working-copy
  commit already has the right author.
- **identity**: Refreshing the repositories cache, which `rt insert` also does,
  sets the identity of the repositories whose configuration differs from it,
  with a hint listing the changes.
- **identity**: After a change of the configuration file, the next `rt`
  command using the repositories cache sets the new identities in the
  repositories.

### Fixed bugs

- Fix computation of the expected path of a local repository, to properly use
  the configuration values.
- **rcd**: Fix bug when arguments contain -h anywhere within. The function was
  then printing the help text instead of doing the resolution.
- **fetch**: Fix the `jj` command line that was fetching the repository to use
  the default fetch configuration. Using `rt fetch` we did not fetch the tags.
- **resolve**: Fix computation of the short repository identifiers when there
  was a conflict.
- **jj prompt**: Display the untracked remote bookmarks of a local bookmark
  apart from it, instead of merging them in the local bookmark as if they were
  tracked.
- **workspace**: Fix the repositories cache sometimes still listing the
  previous workspaces after `rt workspace add` or `rt workspace rm`.
- **workspace**: Refresh the repositories cache automatically when a workspace
  is added or removed outside of `rt`.
- **rm**: Fix the confirmation messages of `rt rm` and `rt workspace rm`, which
  were broken over two lines, and name the workspace `rt workspace rm` removes.
- **rm**: Fix `rt rm` and `rt workspace rm` crashing after removing the
  repository when its parent directory was left empty. Empty parent directories
  are now removed up to the tree-space directory, and errors are reported
  instead of crashing.
- Fix the repositories cache losing the Jujutsu repositories when refreshed
  from a directory which does not exist anymore, as after `rt workspace rm`.
- **workspace**: `rt workspace rm` now forgets the workspace in Jujutsu before
  deleting its directory, and fails when asked to remove the default or the
  only workspace of a repository.
- **workspace**: When refreshing the repositories cache, forget, with a
  warning, the Jujutsu workspaces which directory has been deleted, so that
  `rt` and `jj` agree on the list of workspaces.
- **completion**: The completion of the `todo` shell function completes the
  `rt todo` arguments instead of the `rt git status` ones.
- **rm**, **workspace**: The help of `--force` tells that it skips the
  confirmation. Failing to read the confirmation aborts instead of crashing.
- **git prompt**: Display the current branch, only the other branches pointing
  at the same commit were displayed.
- **prompt**: No longer display empty fields, for an empty status summary or a
  host without representation.
- **repo**: The errors of `rt repo remote` are printed on the standard error
  instead of the standard output, which is meant for the remote URL.
- **fetch**: Report why a repository could not be fetched. The error of a
  Jujutsu fetch was silently ignored.
- **tree**: The short option to refresh the cache is `-R`, as for the other
  commands, instead of `-r`. A repository whose submodules cannot be read is
  displayed without them instead of making `rt tree` crash.
- **list**, **todo**: The `--host local` filter, suggested by the help, now
  selects the repositories without remote. It matched nothing.
- **resolve**: Resolving a repository with several workspaces no longer
  crashes when the tree-space selected with fzf is unknown.
- **resolve-url**: Return the main workspace of the repository, or the one of
  the tree-space given with `--tree`, instead of the last workspace found. Exit
  with code 2 when nothing matches, as `rt resolve` does.
- **clone**: Fail when the clone location exists and is not a repository,
  instead of succeeding without printing the location. The error about an
  invalid URL names the URL.
- **insert**: Removing the directories left empty by the move of a repository
  no longer goes up outside of the repo-tree, it could remove empty directories
  of the user up to the root of the filesystem. `rt insert .` and relative paths
  no longer make `rt` crash.
- **clean**: `rt clean --dry-run` no longer loops forever when there are empty
  directories, it lists them as to be removed. `rt clean` stops with an error
  when an empty directory cannot be removed, instead of looping forever, and
  names the paths when a repository cannot be moved. Agent workspaces are no
  longer reported as misplaced, `rt clean` was trying to move them onto their
  main repository.
- **cache**: The cache is now located in `XDG_CACHE_HOME` when it is set, and
  is written atomically so that a concurrent `rt` never reads a partial cache.
  The first run no longer reports an error about the missing cache.
- **cache**: A missing or unreadable repo-tree root, or an unreadable directory
  in it, is reported as an error instead of making `rt` crash. A repository
  which cannot be loaded is reported, instead of being silently explored as a
  directory of the tree, which reported and let `rt clean` remove the empty
  directories of its `.git` directory.
- **refresh-cache**: Exit with an error when the cache cannot be written.
- **resolve**: Every repository can now be resolved by the shortest end of its
  name which identifies it. A local repository named as the last component of
  another repository could not be resolved at all, a third repository sharing a
  short name with two others took over that short name, and two repositories
  with the same full name were both dropped. A name shared by repositories of
  different hosts now needs the host prefix. The warning about an alias to an
  unknown repository now names the alias.
- **tree**: The URL of a submodule configured relatively to the URL of its main
  repository (`../other.git`) is resolved as git does. One more path component
  was removed, and the URL of a main repository with a scp-like remote was
  converted into a `ssh://` URL.
- **jj prompt**: In a repository colocated with Git, a bookmark existing only
  locally is displayed as local, instead of as tracked by the `git` pseudo
  remote.
- **jj**: An unexpected output of `jj git remote list` or `jj workspace list`
  is reported as an error instead of making `rt` crash. `rt workspace add` to a
  destination which already exists, or whose parent is a file, reports an
  error instead of crashing.
- **repository**: Looking for the repository of a relative path, as done by
  `rt insert`, no longer ends up with the repository of the current directory
  when the path is not inside a repository.
- **git status**: The path of a conflicted file containing a space is no longer
  truncated at the first space. An unexpected output of `git status` is
  reported as an error naming the line, instead of making `rt` crash, and
  unknown headers of future git versions are ignored.
- **git prompt**: Untracked files are displayed once, instead of once in the
  staged and once in the unstaged colors. Conflicted files are now shown in the
  status summary, with the new `prompt.git.status.conflicted` symbol.
- **repo id**: A repository whose remote is a path on the local filesystem,
  or a `file://` URL, is now handled as a local repository. Such repositories
  could not be parsed and were ignored by the repositories search, which
  explored their `.git` directory instead. Remote URLs whose path contains `.`,
  `..` or empty components are rejected, so they cannot designate a location
  outside the repo-tree, and a scp-like URL with an absolute path no longer
  leads to an absolute location.
- **completion**: The completion of the `--host` options now proposes the host
  names starting with the typed text. It used to compare the text with the host
  URL, proposing for instance nothing for `kern` and `kernel` for `git`.
- **git prompt**: Display the "ahead" symbol, instead of the "up-to-date" one,
  when the current branch is ahead of its upstream.
- **config**: A missing `root` without `REPO_TREE_DIR`, or a relative
  `REPO_TREE_DIR`, is now reported as an error instead of making `rt` crash.
  An empty or relative `XDG_CONFIG_HOME` is ignored, as required by the XDG
  Base Directory specification.
- **config**: Every configuration table can now be partially written, the
  missing values take their default value. For instance a `[repository]`
  table with only `ignore`, or a `[tree.archive]` table with only `dir_name`
  were rejected.
- **config**: A color given as an array with other than 3 values is now
  reported as a configuration error instead of making `rt` crash. The `text`
  and `color` of a colored text can now each be omitted.
- **todo**: `rt todo next` now goes forward in the list of repositories and
  `rt todo prev` backward, from inside a repository as from outside of it. Both
  no longer hang when the current repository is not in the filtered list, and
  never propose the current repository, including from one of its workspaces.
- **errors**: The errors about a repository in an unexpected tree-space and
  about a directory which is not a repository now name the tree-space and the
  directory instead of repeating the repository or the version control system.
- **tree**: `rt tree` shows, under each git submodule, how many commits its
  checkout is ahead of and behind the commit recorded in the main repository,
  or that it is not checked out. These counts were computed from the main
  repository objects, which do not hold the submodule commits, and were never
  shown.

## [0.2.0] - 2026-06-21

### Fixed bugs

- Fix bad prompt for jujutsu repository when the repository is not the default
  workspace.

- Support the presence of username in SSH remote URLS.

- Fix `rt todo prev` and `rt todo next` commands due to a bad iteration.

- Fixes for auto-completion:
  - remove multiple suggestion for a same repository with multiple identifiers
    for a single suggestion.
  - Be deterministic with the order of the suggestions.

## [0.1.0] - 2026-04-17

Initial release.
