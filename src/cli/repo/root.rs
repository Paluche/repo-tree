//! Compute the root to the repository.
use super::super::get_cwd;
use crate::VersionControlSystem;

pub fn root(parent: bool, print_type: bool) -> i32 {
    let mut cwd = get_cwd();

    if parent && let Some(parent) = cwd.parent() {
        cwd = parent.to_path_buf()
    }

    if let Some((root, vcs, _)) =
        VersionControlSystem::discover_root(cwd.clone())
    {
        print!("{}", root.display());
        if print_type {
            println!(" {} {}", vcs.is_git(), vcs.is_jujutsu(),);
        } else {
            println!();
        }
        0
    } else {
        1
    }
}
