use crate::core::{RpcType, ToFromBytes};
use crate::error::RpcResult;
use crate::{Bytes, OwnedBytes};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// Premade RPC Type implementations for core types to avoid habing to new-type all the time

/*
impl<T> ToFromBytes for T
where
    T: Serialize + DeserializeOwned,
{
    fn to_bytes(&self) -> RpcResult<OwnedBytes> {
        serde_pickle::to_vec(self, serde_pickle::SerOptions::new()).into()
    }

    fn of_bytes(b: Bytes) -> RpcResult<Self>
    where
        Self: Sized,
    {
        serde_pickle::from_slice(b, serde_pickle::DeOptions::new()).into()
    }
}
*/

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

/*
impl<T> ToFromBytes for Vec<T>
where
    T: ToFromBytes,
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

 */

impl<T> RpcType for T where T: ToFromBytes + Clone + Serialize + for<'de> Deserialize<'de> + 'static {}

//impl RpcType for String {}
/*
impl<T> RpcType for Vec<T> where
    T: ToFromBytes + Clone + Serialize + for<'de> Deserialize<'de> + 'static
{
}

 */
