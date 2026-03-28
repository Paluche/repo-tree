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
#[error("Missing host configuration for {0}")]
/// Error during the parsing of the remote URL.
pub struct UnknownRemoteHostError(pub String);

#[derive(Debug, Error)]
#[error("No cache file to load")]
/// Error during the parsing of the remote URL.
pub struct NoCacheError();
