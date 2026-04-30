//! Utilities for working with any HTTP-based APIs.

/// Error type for HTTP-based APIs
#[derive(Debug, thiserror::Error)]
pub enum Error<E> {
    /// Incorrect API usage while building a request
    #[error(transparent)]
    Request(anyhow::Error),
    /// Transport failure
    #[error(transparent)]
    Transport(anyhow::Error),
    /// Failed to decode response
    #[error(transparent)]
    Decode(anyhow::Error),
    /// Error returned by the remote service
    #[error(transparent)]
    Service(E),
}

impl<E> Error<E> {
    pub(crate) fn flat_result<T>(r: anyhow::Result<Result<T, E>>) -> Result<T, Self> {
        match r {
            Ok(Ok(data)) => Ok(data),
            Ok(Err(e)) => Err(Self::Service(e)),
            Err(e) => Err(Self::Decode(e)),
        }
    }
}
