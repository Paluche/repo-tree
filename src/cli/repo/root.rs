//! Compute the root to the repository.
use clap::Args;

use super::super::get_cwd;
use crate::VersionControlSystem;

/// Get the root and type of the repository the working directory or its
/// parent is into.
#[derive(Args, Debug, PartialEq)]
pub struct RootArgs {
    /// Get the root of the repository the parent directory of the current
    /// working directory is in.
    #[arg(long, short)]
    parent: bool,

    /// Also display repository types. The output will then have 4 words:
    /// <Root of the repository> <is_git> <is_jj>
    #[arg(long)]
    print_type: bool,
}

pub fn run(args: RootArgs) -> i32 {
    let mut cwd = get_cwd();

    if args.parent
        && let Some(parent) = cwd.parent()
    {
        cwd = parent.to_path_buf()
    }

    if let Some((root, vcs, _, _, _)) =
        VersionControlSystem::discover_root(cwd.clone())
    {
        print!("{}", root.display());
        if args.print_type {
            println!("\n{}\n{}", vcs.is_git(), vcs.is_jujutsu(),);
        } else {
            println!();
        }
        0
    } else {
        1
    }
}
