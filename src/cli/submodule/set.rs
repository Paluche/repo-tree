use std::{error::Error, path::PathBuf};

use git2::Oid;

use crate::git::submodules;

pub fn set(
    main_repository_path: PathBuf,
    submodule_relpath: String,
    reference: String,
) -> Result<(), Box<dyn Error>> {
    let main_repo = git2::Repository::discover(main_repository_path)?;

    // TODO: Greater range of references
    let commit_oid = Oid::from_str(&reference)?;

    Ok(submodules::set(&main_repo, &submodule_relpath, commit_oid)?)
}
