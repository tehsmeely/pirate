use crate::transport::TransportError;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum RpcError {
    ParseError(serde_pickle::error::Error),
    TransportError(TransportError),
    Custom(String),
}

impl Display for RpcError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseError(pickle) => write!(f, "{}", pickle),
            Self::TransportError(transport_error) => write!(f, "{}", transport_error),
            Self::Custom(s) => write!(f, "{}", s),
        }
    }
}

impl Error for RpcError {}

impl From<serde_pickle::Error> for RpcError {
    fn from(e: serde_pickle::Error) -> Self {
        Self::ParseError(e)
    }
}
impl From<TransportError> for RpcError {
    fn from(e: TransportError) -> Self {
        Self::TransportError(e)
    }
}

pub type RpcResult<A> = Result<A, RpcError>;
