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

    /// Diagnostic helper intended for tests. Returns the inner service error if `self` is
    /// [`Error::Service`], otherwise panics with the actual variant.
    #[track_caller]
    pub fn unwrap_service(self) -> E
    where
        E: std::fmt::Debug,
    {
        match self {
            Self::Service(e) => e,
            other => panic!("Expected Service error but got {other:?}"),
        }
    }

    /// Returns the inner service error if `self` is [`Error::Decode`],
    /// or panics with the actual variant for diagnostics.
    #[track_caller]
    pub fn unwrap_decode(self) -> anyhow::Error
    where
        E: std::fmt::Debug,
    {
        match self {
            Self::Decode(e) => e,
            other => panic!("Expected Decode error but got {other:?}"),
        }
    }
}
