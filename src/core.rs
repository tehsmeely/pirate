use std::any::Any;

use crate::error::RpcResult;
use crate::{Bytes, OwnedBytes};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Display;
use std::hash::Hash;
use std::marker::PhantomData;

pub trait ToFromBytes {
    fn to_bytes(&self) -> RpcResult<OwnedBytes>;
    fn of_bytes(b: Bytes) -> RpcResult<Self>
    where
        Self: Sized;
}

pub trait RpcType: Any + ToFromBytes + Clone {}

pub trait RpcName: PartialEq + Eq + Hash + Serialize + DeserializeOwned + Display + Clone {}

#[derive(Clone)]
pub struct Rpc<Name, Q: RpcType, R: RpcType> {
    pub name: Name,
    _query_phantom: PhantomData<Q>,
    _response_phantom: PhantomData<R>,
}

impl<Name: RpcName, Q: RpcType, R: RpcType> Rpc<Name, Q, R> {
    pub fn new(name: Name) -> Self {
        Self {
            name,
            _query_phantom: PhantomData,
            _response_phantom: PhantomData,
        }
    }
}

pub struct RpcImpl<Name: RpcName, State, Q: RpcType, R: RpcType> {
    pub rpc: Rpc<Name, Q, R>,
    call: Box<dyn Fn(&mut State, Q) -> RpcResult<R>>,
}

impl<Name: RpcName, State, Q: RpcType, R: RpcType> RpcImpl<Name, State, Q, R> {
    pub fn new(name: Name, call: Box<dyn Fn(&mut State, Q) -> RpcResult<R>>) -> Self {
        Self {
            rpc: Rpc::new(name),
            call,
        }
    }

    fn query_of_bytes(&self, b: Bytes) -> RpcResult<Q> {
        Q::of_bytes(b)
    }
    fn call(&self, state: &mut State, q: Q) -> RpcResult<R> {
        (self.call)(state, q)
    }
    fn result_to_bytes(&self, r: R) -> RpcResult<OwnedBytes> {
        R::to_bytes(&r)
    }
}

pub trait StoredRpc<State, Name: RpcName> {
    fn call_of_bytes(&self, bytes: Bytes, state: &mut State) -> RpcResult<OwnedBytes>;
    fn rpc_name(&self) -> Name;
}

impl<Name: RpcName, State, Q: RpcType, R: RpcType> StoredRpc<State, Name>
    for RpcImpl<Name, State, Q, R>
{
    fn call_of_bytes(&self, input_bytes: Bytes, state: &mut State) -> RpcResult<OwnedBytes> {
        let query = self.query_of_bytes(input_bytes)?;
        let result = self.call(state, query)?;
        let result_bytes = self.result_to_bytes(result)?;
        Ok(result_bytes)
    }

    fn rpc_name(&self) -> Name {
        self.rpc.name.clone()
    }
}
