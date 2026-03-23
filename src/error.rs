use thiserror::Error;

#[derive(Debug, Error)]
#[error("{0} not implemented yet")]
pub struct NotImplementedError(pub String);
