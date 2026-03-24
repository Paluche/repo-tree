//! Definition of errors struct used in the crate.
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{0} not implemented yet")]
/// A functionality is not implemented yet.
pub struct NotImplementedError(pub String);

#[derive(Debug, Error)]
#[error("Error parsing {0}")]
/// Error during the parsing of the remote URL.
pub struct ParseUrlError(pub String);
