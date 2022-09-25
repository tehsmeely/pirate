use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::Write;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_pickle::{DeOptions, SerOptions};

mod client;
mod core;
mod error;
mod server;
mod transport;

pub type Bytes<'a> = &'a [u8];
pub type OwnedBytes = Vec<u8>;

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
    use crate::core::{Rpc, RpcImpl, RpcName, RpcType, ToFromBytes};
    use crate::error::RpcResult;
    use crate::{Bytes, OwnedBytes};
    use serde::{Deserialize, Serialize};
    use std::fmt::{Display, Formatter};

    pub struct HelloWorldState {
        pub i: usize,
    }

    #[derive(Clone, Hash, Eq, PartialEq, Debug, Serialize, Deserialize)]
    pub enum HelloWorldRpcName {
        HelloWorld,
    }
    impl Display for HelloWorldRpcName {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self)
        }
    }
    impl RpcName for HelloWorldRpcName {}

    #[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
    pub struct QR(pub String);
    impl ToFromBytes for QR {
        fn to_bytes(&self) -> RpcResult<OwnedBytes> {
            serde_pickle::ser::to_vec(&self, serde_pickle::SerOptions::new()).map_err(Into::into)
        }

        fn of_bytes(b: Bytes) -> RpcResult<Self> {
            serde_pickle::de::from_slice(b, serde_pickle::DeOptions::new()).map_err(Into::into)
        }
    }
    impl RpcType for QR {}

    pub fn make_hello_world_rpc() -> Rpc<HelloWorldRpcName, QR, QR> {
        Rpc::new(HelloWorldRpcName::HelloWorld)
    }
    pub fn make_hello_world_rpc_impl() -> RpcImpl<HelloWorldRpcName, HelloWorldState, QR, QR> {
        RpcImpl::new(
            HelloWorldRpcName::HelloWorld,
            Box::new(|state, q| {
                println!("Got Called! {:?}", q);
                Ok(QR(format!("Hello world: {}:{:?}", state.i, q)))
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::server::RPCServer;

    use super::example::*;
    use super::*;

    #[test]
    fn just_server_test() {
        let state = HelloWorldState { i: 3 };
        let mut server = RPCServer::new(Arc::new(state));
        server.add_rpc(
            HelloWorldRpcName::HelloWorld,
            Box::new(make_hello_world_rpc_impl()),
        );
        println!("Full Test");
        let incoming_bytes =
            serde_pickle::ser::to_vec(&"Foo", serde_pickle::SerOptions::new()).unwrap();
        let incoming_type_id = TypeId::of::<(QR, QR)>();
        let wrong_incoming_type_id = TypeId::of::<(QR, u8)>();
        server.call(
            &incoming_bytes,
            &HelloWorldRpcName::HelloWorld,
            incoming_type_id,
        );
        server.call(
            &incoming_bytes,
            &HelloWorldRpcName::HelloWorld,
            wrong_incoming_type_id,
        );
    }

    #[test]
    fn sync_round_trip() {}
}
