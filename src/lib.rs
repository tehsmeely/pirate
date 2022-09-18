use serde::{Deserialize, Serialize};
use serde_pickle::{DeOptions, SerOptions};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::Write;
use std::sync::Arc;

pub type Bytes<'a> = &'a [u8];
pub type OwnedBytes = Vec<u8>;

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

pub struct Rpc<State, Q: Any, R: Any> {
    query_of_bytes: Box<dyn Fn(Bytes) -> RpcResult<Q>>,
    call: Box<dyn Fn(&State, Q) -> RpcResult<R>>,
    result_to_bytes: Box<dyn Fn(R) -> RpcResult<OwnedBytes>>,
}
impl<State, Q: Any, R: Any> Rpc<State, Q, R> {
    fn query_of_bytes(&self, b: Bytes) -> RpcResult<Q> {
        (self.query_of_bytes)(b)
    }
    fn call(&self, state: &State, q: Q) -> RpcResult<R> {
        (self.call)(state, q)
    }
    fn result_to_bytes(&self, r: R) -> RpcResult<OwnedBytes> {
        (self.result_to_bytes)(r)
    }
}

pub trait StoredRpc<State> {
    fn call_of_bytes(
        &self,
        bytes: Bytes,
        state: &State,
        response_writer: impl Write,
    ) -> RpcResult<()>;
    fn internal_type_id(&self) -> TypeId;
}

impl<State, Q: 'static, R: 'static> StoredRpc<State> for Rpc<State, Q, R> {
    fn call_of_bytes(
        &self,
        input_bytes: Bytes,
        state: &State,
        mut response_writer: impl Write,
    ) -> RpcResult<()> {
        let query = self.query_of_bytes(input_bytes)?;
        let result = self.call(state, query)?;
        let result_bytes = self.result_to_bytes(result)?;
        //TODO: Support [into] for WriteError
        response_writer.write(&result_bytes).unwrap();
        Ok(())
    }

    fn internal_type_id(&self) -> TypeId {
        TypeId::of::<(Q, R)>()
    }
}

pub struct RPCServer<S, R>
where
    R: StoredRpc<S>,
{
    state: Arc<S>,
    rpcs: HashMap<TypeId, Box<R>>,
}
impl<S, R> RPCServer<S, R>
where
    R: StoredRpc<S>,
{
    fn new(state: Arc<S>) -> Self {
        Self {
            state,
            rpcs: HashMap::new(),
        }
    }

    fn add_rpc(&mut self, rpc_impl: Box<R>) {
        let type_id = rpc_impl.internal_type_id();
        self.rpcs.insert(type_id, rpc_impl);
    }

    fn start(&self) {
        //Handle incoming connection
        let query_type_id = TypeId::of::<(String, String)>();
        let query_bytes = serde_pickle::ser::to_vec(&"foo", SerOptions::new()).unwrap();
        //Identify [RPC] by type id (or uuid?)
        let mut print_writer = PrintWriter;
        if let Some(rpc_impl) = self.rpcs.get(&query_type_id) {
            rpc_impl.call_of_bytes(&query_bytes, &self.state, &mut print_writer);
        }
        //Exec [RPC.implementation] with [self.state] and [query]
        //return result or error
    }
}

pub struct PrintWriter;
impl Write for PrintWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        println!("PrintWriter. Writing: {}", String::from_utf8_lossy(buf));
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

mod example {
    use crate::Rpc;

    pub struct HelloWorldState {
        pub i: usize,
    }

    pub fn make_hello_world_rpc() -> Rpc<HelloWorldState, String, String> {
        Rpc {
            query_of_bytes: Box::new(|bytes| {
                serde_pickle::de::from_slice(bytes, serde_pickle::de::DeOptions::new())
                    .map_err(Into::into)
            }),
            call: Box::new(|state, q| Ok(format!("Hello world: {}:{}", state.i, q))),
            result_to_bytes: Box::new(|s| {
                serde_pickle::ser::to_vec(&s, serde_pickle::ser::SerOptions::new())
                    .map_err(Into::into)
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::example::*;
    use super::*;
    use std::sync::Arc;

    #[test]
    fn full_test() {
        let state = HelloWorldState { i: 3 };
        let mut server = RPCServer::new(Arc::new(state));
        server.add_rpc(Box::new(make_hello_world_rpc()));
        server.start();
    }
}
