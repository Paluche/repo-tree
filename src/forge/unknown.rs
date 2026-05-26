//! Mockup interaction with an unknown / unsupported forge.
use std::error::Error;

use async_trait::async_trait;

use super::ForgeApi;
use crate::repo_id::RepoId;

/// The repository is not stored on a forge or stored on an non-supported forge.
pub struct UnknownForgeApi;

#[async_trait(?Send)]
impl ForgeApi for UnknownForgeApi {
    async fn is_archived(
        &self,
        _repo_id: &RepoId,
    ) -> Result<bool, Box<dyn Error>> {
        Ok(false)
    }

    async fn get_name(
        &self,
        repo_id: &RepoId,
    ) -> Result<String, Box<dyn Error>> {
        Ok(repo_id.name.clone())
    }
}
