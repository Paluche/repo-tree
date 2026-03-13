//! Definition of common revsets into functions.

use std::{error::Error, path::Path, process::Command, sync::Arc};

use jj_lib::{
    backend::{BackendResult, CommitId},
    repo::Repo,
    revset::{
        RemoteRefSymbolExpression, ResolvedRevsetExpression,
        RevsetFilterPredicate, SymbolResolver, SymbolResolverExtension,
        UserRevsetExpression,
    },
    str_util::{StringExpression, StringPattern},
};

fn default_symbol_resolver(repo: &dyn Repo) -> SymbolResolver<'_> {
    SymbolResolver::new(repo, &([] as [&Box<dyn SymbolResolverExtension>; 0]))
}

/// Revset to match all the remote bookmarks.
pub fn remote_bookmarks(repo: &dyn Repo) -> Arc<ResolvedRevsetExpression> {
    UserRevsetExpression::remote_bookmarks(
        RemoteRefSymbolExpression {
            name: StringExpression::Pattern(Box::new(StringPattern::all())),
            remote: StringExpression::NotIn(Box::new(StringExpression::exact(
                "ignored",
            ))),
        },
        None,
    )
    .resolve_user_expression(repo, &default_symbol_resolver(repo))
    .expect("remote_bookmarks resolution failed.")
}

/// Revset to match empty commits with empty descriptions.
pub fn bare_commit() -> Arc<ResolvedRevsetExpression> {
    ResolvedRevsetExpression::is_empty().union(
        &ResolvedRevsetExpression::filter(RevsetFilterPredicate::Description(
            StringExpression::exact(""),
        )),
    )
}

pub fn conflicts() -> Arc<ResolvedRevsetExpression> {
    ResolvedRevsetExpression::filter(RevsetFilterPredicate::HasConflict)
}

// Revset to match any mutable commits, according to the configuration.
pub fn mutable(
    repo_path: &Path,
) -> Result<Arc<ResolvedRevsetExpression>, Box<dyn Error>> {
    Ok(immutable_heads(repo_path)?.negated())
}

pub fn immutable_heads(
    repo_path: &Path,
) -> Result<Arc<ResolvedRevsetExpression>, Box<dyn Error>> {
    let output = String::from_utf8(
        Command::new("jj")
            .arg("--ignore-working-copy")
            .arg("--repository")
            .arg(repo_path)
            .arg("log")
            .arg("-r")
            .arg("immutable_heads()")
            .arg("--no-graph")
            .arg("--template")
            .arg("commit_id ++ \"\n\"")
            .output()?
            .stdout,
    )?;

    Ok(ResolvedRevsetExpression::commits(
        output
            .split("\n")
            .filter_map(|commit_id| {
                (!commit_id.is_empty())
                    .then_some(CommitId::try_from_hex(commit_id).unwrap())
            })
            .collect(),
    ))
}

pub fn has_match(
    repo: &dyn Repo,
    revset: Arc<ResolvedRevsetExpression>,
) -> BackendResult<bool> {
    revset
        .evaluate(repo)
        .map(|l| !l.is_empty())
        .map_err(|e| e.into_backend_error())
}
