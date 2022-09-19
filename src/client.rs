use crate::core::RpcType;
use crate::error::{RpcError, RpcResult};
use std::any::Any;
use std::marker::PhantomData;

pub struct RpcClient<Q: RpcType, R: RpcType> {
    _q: PhantomData<Q>,
    _r: PhantomData<R>,
}
