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
    use crate::client::RpcClient;
    use crate::transport::{TcpTransport, Transport};

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
        server.call(&incoming_bytes, &HelloWorldRpcName::HelloWorld);
        server.call(&incoming_bytes, &HelloWorldRpcName::HelloWorld);
    }

    #[tokio::test]
    async fn server_a() {
        // Server setup
        println!("Server Setup");
        let state = HelloWorldState { i: 3 };
        let mut server = RPCServer::new(Arc::new(state));
        server.add_rpc(
            HelloWorldRpcName::HelloWorld,
            Box::new(make_hello_world_rpc_impl()),
        );
        let addr = "127.0.0.1:5555";
        //let server_future = server.serve(addr);

        // Client
        println!("Client Setup");
        async fn client(addr: &str) -> QR {
            let mut transport = {
                let client_stream = tokio::net::TcpStream::connect(addr).await.unwrap();
                let async_transport = TcpTransport::new(client_stream);
                Transport::new(async_transport)
            };

            let rpc_client = RpcClient::new(make_hello_world_rpc());

            let result = rpc_client
                .call_a(QR("Foo".into()), &mut transport)
                .await
                .unwrap();
            result
        }
        //let client_fut = client(addr);

        let mut client_result = None;

        while client_result.is_none() {
            tokio::select! {
                _ = server.serve(addr) => {},
                client_output = client(addr) => {client_result = Some(client_output)},
            }
        }

        let expecting = QR("".into());
        assert_eq!(Some(expecting), client_result);
    }

    #[test]
    fn sync_round_trip() {}
}
