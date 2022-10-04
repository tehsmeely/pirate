mod client;
mod core;
mod error;
mod server;
mod transport;

pub type Bytes<'a> = &'a [u8];
pub type OwnedBytes = Vec<u8>;

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
        GetI,
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

    impl ToFromBytes for () {
        fn to_bytes(&self) -> RpcResult<OwnedBytes> {
            serde_pickle::ser::to_vec(&self, serde_pickle::SerOptions::new()).map_err(Into::into)
        }
        fn of_bytes(b: Bytes) -> RpcResult<Self> {
            serde_pickle::de::from_slice(b, serde_pickle::DeOptions::new()).map_err(Into::into)
        }
    }
    impl RpcType for () {}
    impl ToFromBytes for usize {
        fn to_bytes(&self) -> RpcResult<OwnedBytes> {
            serde_pickle::ser::to_vec(&self, serde_pickle::SerOptions::new()).map_err(Into::into)
        }
        fn of_bytes(b: Bytes) -> RpcResult<Self> {
            serde_pickle::de::from_slice(b, serde_pickle::DeOptions::new()).map_err(Into::into)
        }
    }
    impl RpcType for usize {}

    pub fn make_hello_world_rpc() -> Rpc<HelloWorldRpcName, QR, QR> {
        Rpc::new(HelloWorldRpcName::HelloWorld)
    }
    pub fn make_hello_world_rpc_impl() -> RpcImpl<HelloWorldRpcName, HelloWorldState, QR, QR> {
        RpcImpl::new(
            HelloWorldRpcName::HelloWorld,
            Box::new(|state, q| {
                println!("Hello World RPC Got Called! Query: {:?}", q);
                Ok(QR(format!("Hello world: {}:{:?}", state.i, q)))
            }),
        )
    }

    pub fn make_get_i_rpc() -> Rpc<HelloWorldRpcName, (), usize> {
        Rpc::new(HelloWorldRpcName::GetI)
    }
    pub fn make_get_i_rpc_impl() -> RpcImpl<HelloWorldRpcName, HelloWorldState, (), usize> {
        RpcImpl::new(
            HelloWorldRpcName::GetI,
            Box::new(|state, q| {
                println!("GetI RPC Got Called! Query: {:?}", q);
                Ok(state.i)
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::server::RPCServer;

    use super::example::*;
    use crate::client::RpcClient;
    use crate::core::{Rpc, RpcType};
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
        server
            .call(&incoming_bytes, &HelloWorldRpcName::HelloWorld)
            .unwrap();
        server
            .call(&incoming_bytes, &HelloWorldRpcName::HelloWorld)
            .unwrap();
    }

    #[tokio::test]
    async fn server_a() {
        // Server setup
        println!("Server Setup");
        let state = HelloWorldState { i: 3 };
        let mut server = RPCServer::new(Arc::new(state));
        server.add_rpc(
            // TODO: Having to supply the name here sucks, it's already baked into the RpcImpl
            HelloWorldRpcName::HelloWorld,
            Box::new(make_hello_world_rpc_impl()),
        );
        server.add_rpc(HelloWorldRpcName::GetI, Box::new(make_get_i_rpc_impl()));
        let addr = "127.0.0.1:5555";

        async fn call_client<Q: RpcType, R: RpcType>(
            addr: &str,
            q: Q,
            rpc: Rpc<HelloWorldRpcName, Q, R>,
        ) -> R {
            let mut transport = {
                let client_stream = tokio::net::TcpStream::connect(addr).await.unwrap();
                let async_transport = TcpTransport::new(client_stream);
                Transport::new(async_transport)
            };

            let rpc_client = RpcClient::new(rpc);

            let result = rpc_client.call(q, &mut transport).await.unwrap();
            result
        }
        let hello_world_rpc = make_hello_world_rpc();
        let get_i_rpc = make_get_i_rpc();

        async fn client(addr: &str) -> QR {
            let mut transport = {
                let client_stream = tokio::net::TcpStream::connect(addr).await.unwrap();
                let async_transport = TcpTransport::new(client_stream);
                Transport::new(async_transport)
            };

            let rpc_client = RpcClient::new(make_hello_world_rpc());

            let result = rpc_client
                .call(QR("Foo".into()), &mut transport)
                .await
                .unwrap();
            result
        }
        async fn client2(addr: &str) -> usize {
            let mut transport = {
                let client_stream = tokio::net::TcpStream::connect(addr).await.unwrap();
                let async_transport = TcpTransport::new(client_stream);
                Transport::new(async_transport)
            };

            let rpc_client = RpcClient::new(make_get_i_rpc());

            let result = rpc_client.call((), &mut transport).await.unwrap();
            result
        }

        let mut rpc_results = None;
        let mut client_call_task = tokio::spawn(async move {
            let r1 = (call_client(addr, QR("foo".into()), hello_world_rpc)).await;
            let r2 = (call_client(addr, (), get_i_rpc)).await;
            (r1, r2)
        });

        while rpc_results.is_none() {
            println!(".");
            tokio::select! {
                _ = server.serve(addr) => {},
                client_output = &mut client_call_task => {rpc_results = Some(client_output)},
            }
        }

        let (a, b) = rpc_results.unwrap().unwrap();
        let expecting = QR("Hello world: 3:QR(\"foo\")".into());
        assert_eq!(expecting, a);
        assert_eq!(3usize, b);
    }

    #[test]
    fn sync_round_trip() {}
}
