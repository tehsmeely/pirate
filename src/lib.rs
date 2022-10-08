mod client;
mod core;
pub mod error;
pub mod rpc_types;
mod server;
mod transport;

pub type Bytes<'a> = &'a [u8];
pub type OwnedBytes = Vec<u8>;

pub use crate::client::call_client;
pub use crate::client::RpcClient;
pub use crate::core::Rpc;
pub use crate::core::RpcImpl;
pub use crate::core::RpcName;
pub use crate::core::RpcType;
pub use crate::core::StoredRpc;
pub use crate::server::RpcServer;

#[cfg(feature = "macros")]
pub use pirates_macro_lib::rpc_definition;

pub trait RpcDefinition<Name: RpcName, State, Q: RpcType, R: RpcType> {
    fn client() -> Rpc<Name, Q, R>;
    fn server() -> RpcImpl<Name, State, Q, R>;
}

mod example {
    use crate::core::{Rpc, RpcImpl, RpcName, RpcType, ToFromBytes};
    use crate::error::RpcResult;
    use crate::{Bytes, OwnedBytes, RpcDefinition};
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

    pub fn make_hello_world_rpc() -> Rpc<HelloWorldRpcName, String, String> {
        Rpc::new(HelloWorldRpcName::HelloWorld)
    }
    pub fn make_hello_world_rpc_impl() -> RpcImpl<HelloWorldRpcName, HelloWorldState, String, String>
    {
        RpcImpl::new(
            HelloWorldRpcName::HelloWorld,
            Box::new(|state, q| {
                println!("Hello World RPC Got Called! Query: {:?}", q);
                Ok(format!("Hello world: {}:{:?}", state.i, q))
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
        fn implement(state: &mut HelloWorldState, _query: ()) -> RpcResult<()> {
            state.i += 1;
            Ok(())
        }
    }
    impl RpcDefinition<HelloWorldRpcName, HelloWorldState, (), ()> for IncrIRpc {
        fn client() -> Rpc<HelloWorldRpcName, (), ()> {
            Rpc::new(HelloWorldRpcName::IncrI)
        }

        fn server() -> RpcImpl<HelloWorldRpcName, HelloWorldState, (), ()> {
            RpcImpl::new(HelloWorldRpcName::IncrI, Box::new(Self::implement))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::server::RpcServer;

    use super::example::*;
    use crate::client::RpcClient;
    use crate::core::{Rpc, RpcType};
    use crate::example::HelloWorldRpcName::IncrI;
    use crate::transport::{TcpTransport, Transport};
    use crate::RpcDefinition;

    #[test]
    fn just_server_test() {
        let state = HelloWorldState { i: 3 };
        let mut server = RpcServer::new(Arc::new(Mutex::new(state)));
        server.add_rpc(Box::new(make_hello_world_rpc_impl()));
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
        let mut server = RpcServer::new(state_ref);
        server.add_rpc(Box::new(make_hello_world_rpc_impl()));
        server.add_rpc(Box::new(make_get_i_rpc_impl()));
        server.add_rpc(Box::new(IncrIRpc::server()));
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
        let incr_i_rpc = IncrIRpc::client();

        let mut rpc_results = None;
        let mut client_call_task = tokio::spawn(async move {
            let r1 = call_client(addr, "foo".into(), hello_world_rpc.clone()).await;
            let r2 = call_client(addr, (), get_i_rpc.clone()).await;
            let () = call_client(addr, (), incr_i_rpc.clone()).await;
            let r3 = call_client(addr, (), get_i_rpc).await;
            let () = call_client(addr, (), incr_i_rpc).await;
            let r4 = call_client(addr, "bar".into(), hello_world_rpc).await;
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
        let expecting: String = "Hello world: 3:String(\"foo\")".into();
        assert_eq!(expecting, hello_world_1);
        assert_eq!(3usize, get_i_1);
        assert_eq!(4usize, get_i_2);
        let expecting2: String = "Hello world: 5:String(\"bar\")".into();
        assert_eq!(expecting2, hello_world_2);
    }
}

mod example_macro_expand {
    use serde::{Deserialize, Serialize};
    #[derive(PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
    enum MyName {
        One,
    }
    impl std::fmt::Display for MyName {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                MyName::One => write!(f, "One"),
            }
        }
    }
    impl RpcName for MyName {}
    struct MyState {}
    struct Foo {}
    use crate::error::RpcResult;
    use crate::RpcName;
    use std::fmt::Formatter;

    mod pirates {
        pub use crate::Rpc;
        pub use crate::RpcDefinition;
        pub use crate::RpcImpl;
    }

    #[pirates_macro_lib::rpc_definition]
    impl Foo {
        fn name() -> MyName {
            MyName::One
        }
        fn implement(_state: &mut MyState, query: usize) -> RpcResult<String> {
            Ok(format!("You sent me {}", query))
        }
    }
}
