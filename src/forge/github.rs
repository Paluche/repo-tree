//! Interaction with GitHub forge through their REST API.
use std::error::Error;
use std::sync::Arc;

use async_trait::async_trait;
use octocrab::Octocrab;
use octocrab::models::Repository;

use super::ForgeApi;
use crate::error::UnimplementedForgeApi;
use crate::repo_id::RepoId;

/// Interaction with the GitHub API.
pub struct GitHubApi {
    /// Octocrab instance to use to interact with the GitHub API.
    instance: Arc<Octocrab>,
}

/// Get the Octocrab instance we will use to communicate with GitHub.
pub fn api() -> GitHubApi {
    GitHubApi {
        instance: octocrab::instance(),
    }
}

impl GitHubApi {
    /// Simple tool to split the GitHub owner and repository name from a full
    /// GitHub repository name.
    fn split_repo_id(
        repo_id: &RepoId,
    ) -> Result<(&str, &str), UnimplementedForgeApi> {
        let mut parts = repo_id.name.as_str().split("/");
        let owner = parts.next().unwrap();
        let name = parts.next().unwrap();
        assert!(parts.next().is_none());

        // XXX This does not that into account that your repository might be
        // hosted of a self-hosted github.
        let host = &repo_id
            .remote
            .as_ref()
            .expect("There should be a remote.")
            .host_url;
        if host != "https://github.com" {
            Err(UnimplementedForgeApi(host.to_string()))
        } else {
            Ok((owner, name))
        }
    }

    /// Get the information for a repository.
    // XXX Cache this function
    async fn get_repo(
        &self,
        repo_id: &RepoId,
    ) -> Result<Repository, Box<dyn Error>> {
        let (owner, repo) = Self::split_repo_id(repo_id)?;
        Ok(self.instance.repos(owner, repo).get().await?)
    }
}

#[async_trait(?Send)]
impl ForgeApi for GitHubApi {
    async fn is_archived(
        &self,
        repo_id: &RepoId,
    ) -> Result<bool, Box<dyn Error>> {
        Ok(self.get_repo(repo_id).await?.archived.unwrap_or(false))
    }

    /// Get the name of the repository as it is on the forge.
    async fn get_name(
        &self,
        repo_id: &RepoId,
    ) -> Result<String, Box<dyn Error>> {
        let (owner, repo) = Self::split_repo_id(repo_id)?;

        Ok(self
            .instance
            .repos(owner, repo)
            .get()
            .await?
            .full_name
            .unwrap_or(repo_id.name.clone()))
    }
}
