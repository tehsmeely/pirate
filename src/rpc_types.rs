use crate::core::{RpcType, ToFromBytes};
use crate::error::RpcResult;
use crate::{Bytes, OwnedBytes};

use serde::{Deserialize, Serialize};

/// Pre-made RPC Type implementations for core types to avoid having to new-type all the time
impl<T> ToFromBytes for T
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    fn to_bytes(&self) -> RpcResult<OwnedBytes> {
        serde_pickle::to_vec(self, serde_pickle::SerOptions::new()).map_err(Into::into)
    }

    fn of_bytes(b: Bytes) -> RpcResult<Self>
    where
        Self: Sized,
    {
        serde_pickle::from_slice(b, serde_pickle::DeOptions::new()).map_err(Into::into)
    }
}

impl<T> RpcType for T where T: ToFromBytes + Clone + Serialize + for<'de> Deserialize<'de> + 'static {}
