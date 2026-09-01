//! Interaction with the different forges.
use std::error::Error;

use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;

use crate::error::UnimplementedForgeApi;
use crate::repo_id::RepoId;

/// The different supported forges.
#[derive(Serialize, Deserialize, Clone, PartialEq, Hash, Debug)]
pub enum Forge {
    /// GitHub forge,
    GitHub,
    /// GitLab forge,
    GitLab,
    /// Forgejo forge,
    Forgejo,
    /// Bitbucket forge.
    Bitbucket,
}

/// All possible interactions we want to have with a forge API.
#[async_trait(?Send)]
#[allow(dead_code)]
pub trait ForgeApi {
    /// Find out if the repository is archived.
    async fn is_archived(
        &self,
        repo_id: &RepoId,
    ) -> Result<bool, Box<dyn Error>>;

    /// Get the name of the repository as it is on the forge.
    async fn get_name(
        &self,
        repo_id: &RepoId,
    ) -> Result<String, Box<dyn Error>>;
}

impl Forge {
    /// Get the struct implementing the ForgeApi for the respective forge.
    #[allow(dead_code)]
    pub fn api(&self) -> Result<Box<dyn ForgeApi>, Box<dyn Error>> {
        match self {
            Self::GitHub | Self::GitLab | Self::Forgejo | Self::Bitbucket => {
                Err(Box::new(UnimplementedForgeApi(self.clone())))
            }
        }
    }
}
