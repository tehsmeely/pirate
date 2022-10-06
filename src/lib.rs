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
        IncrI,
    }
    impl Display for HelloWorldRpcName {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self)
        }
    }
    impl RpcName for HelloWorldRpcName {}

    #[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
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

    pub struct IncrIRpc {}
    impl IncrIRpc {
        pub fn rpc() -> Rpc<HelloWorldRpcName, (), ()> {
            Rpc::new(HelloWorldRpcName::IncrI)
        }
        pub fn rpc_impl() -> RpcImpl<HelloWorldRpcName, HelloWorldState, (), ()> {
            RpcImpl::new(
                HelloWorldRpcName::IncrI,
                Box::new(|state, ()| {
                    state.i += 1;
                    Ok(())
                }),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::server::RPCServer;

    use super::example::*;
    use crate::client::RpcClient;
    use crate::core::{Rpc, RpcType};
    use crate::example::HelloWorldRpcName::IncrI;
    use crate::transport::{TcpTransport, Transport};

    #[test]
    fn just_server_test() {
        let state = HelloWorldState { i: 3 };
        let mut server = RPCServer::new(Arc::new(Mutex::new(state)));
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
        let state_ref = Arc::new(Mutex::new(state));
        let mut server = RPCServer::new(state_ref);
        server.add_rpc(
            // TODO: Having to supply the name here sucks, it's already baked into the RpcImpl
            HelloWorldRpcName::HelloWorld,
            Box::new(make_hello_world_rpc_impl()),
        );
        server.add_rpc(HelloWorldRpcName::GetI, Box::new(make_get_i_rpc_impl()));
        server.add_rpc(HelloWorldRpcName::IncrI, Box::new(IncrIRpc::rpc_impl()));
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
        let incr_i_rpc = IncrIRpc::rpc();

        let mut rpc_results = None;
        let mut client_call_task = tokio::spawn(async move {
            let r1 = call_client(addr, QR("foo".into()), hello_world_rpc.clone()).await;
            let r2 = call_client(addr, (), get_i_rpc.clone()).await;
            let () = call_client(addr, (), incr_i_rpc.clone()).await;
            let r3 = call_client(addr, (), get_i_rpc).await;
            let () = call_client(addr, (), incr_i_rpc).await;
            let r4 = call_client(addr, QR("bar".into()), hello_world_rpc).await;
            (r1, r2, r3, r4)
        });

        while rpc_results.is_none() {
            println!(".");
            tokio::select! {
                _ = server.serve(addr) => {},
                client_output = &mut client_call_task => {rpc_results = Some(client_output)},
            }
        }

        let (hello_world_1, get_i_1, get_i_2, hello_world_2) = rpc_results.unwrap().unwrap();
        let expecting = QR("Hello world: 3:QR(\"foo\")".into());
        let expecting2 = QR("Hello world: 5:QR(\"bar\")".into());
        assert_eq!(expecting, hello_world_1);
        assert_eq!(3usize, get_i_1);
        assert_eq!(4usize, get_i_2);
        assert_eq!(expecting2, hello_world_2);
    }
}
