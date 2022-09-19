use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum RpcError {
    ParseError(serde_pickle::error::Error),
    Custom(String),
}

impl Display for RpcError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseError(pickle) => write!(f, "{:}", pickle),
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

pub type RpcResult<A> = Result<A, RpcError>;
