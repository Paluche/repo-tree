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
- **config**: A color given as an array with other than 3 values is now
  reported as a configuration error instead of making `rt` crash. The `text`
  and `color` of a colored text can now each be omitted.
- **todo**: `rt todo next` now goes forward in the list of repositories and
  `rt todo prev` backward, from inside a repository as from outside of it. Both
  no longer hang when the current repository is not in the filtered list.
- **errors**: The errors about a repository in an unexpected tree-space and
  about a directory which is not a repository now name the tree-space and the
  directory instead of repeating the repository or the version control system.

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
