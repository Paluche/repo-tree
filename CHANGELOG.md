# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Release highlights

Introduction of the `tree-spaces`, you have two available ones being:

- `dev`: Repositories with a remote.
- `local`: Repositories without a remote.

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

### Deprecations

None

### New features

None

### Fixed bugs

- Fix computation of the expected path of a local repository, to properly use
  the configuration values.

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
