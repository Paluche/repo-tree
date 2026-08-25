//! Definition of errors struct used in the crate.
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
#[error("{0} not implemented yet")]
/// A functionality is not implemented yet.
pub struct NotImplementedError(pub String);

#[derive(Debug, Error)]
#[error("Error parsing {0}")]
/// Error during the parsing of the remote URL.
pub struct ParseUrlError(pub String);

#[derive(Debug, Error)]
#[error("No repository found in {0}")]
/// No repository found.
pub struct NoRepositoryError(pub PathBuf);

#[derive(Debug, Error)]
#[error("Repository {0}, should not be in {0} tree-space")]
/// No repository found.
pub struct UnexpectedTreeSpaceError(pub String, pub String);

#[derive(Debug, Error)]
#[error("Missing host configuration for {0}")]
/// Error when trying to obtain configuration information about a host, which
/// has no configuration associated.
pub struct UnknownRemoteHostError(pub String);

#[derive(Debug, Error)]
#[error("No cache file to load")]
/// Error during the parsing of the remote URL.
pub struct NoCacheError();

#[derive(Debug, Error)]
#[error("Bad configuration: {0}")]
/// Error during the parsing of the configuration.
pub struct ConfigError(pub String);

#[derive(Debug, Error)]
#[error(
    "Unable to convert timestamp to obtain the last time {0} has been modified"
)]
/// Error during the retrieving of the last time a file has been modified.
pub struct GetLastModifiedError(pub String);

#[derive(Debug, Error)]
#[error("The API interaction with the forge {0} is not yet implemented.")]
/// Error during the retrieving of the last time a file has been modified.
pub struct UnimplementedForgeApi(pub String);
