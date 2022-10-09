//! Pirates - a straightforward ArrrrPC library
//!
//! The core of things in the RPC definition itself.
//! you achieve this by implementing `RpcDefinition` on a struct of your choice.
//! the `#[pirates::rpc_definition]` macro can do this for you on an impl that
//! contains a run and implement function (Enable the "macros" feature)
//!
//! ```rust,no_run
//! pub struct AddName {}
//! #[pirates::rpc_definition]
//! impl AddName {
//!     fn name() -> RpcId {
//!         RpcId::AddName
//!     }
//!     fn implement(state: &mut ServerState, query: String) -> RpcResult<()> {
//!         state.names.push(query);
//!         Ok(())
//!     }
//! }
//! ```
//!
//! There are two core types these are generic over which you need to define:
//! 1) Rpc Identifier. Create a type which implements RpcName
//! ```rust,no_run
//! #[derive(PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
//! enum RpcId {
//!     AddName,
//!     GetNames,
//! }
//! impl std::fmt::Display for RpcId {
//!     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//!         match self {
//!             Self::AddName => write!(f, "AddName"),
//!             Self::GetNames => write!(f, "GetNames"),
//!         }
//!     }
//! }
//! ```
//! 2) Server state. Any type inside an Arc<Mutex<T> that the server can hand to RPCs
//! ```rust,no_run
//! struct ServerState {
//!     names: Vec<String>,
//! }
//! ```
//!
//!
//! When you have an rpc definition, you can now serve it.
//! Serving is done by creating an `RpcServer` and awaiting its `serve` method
//!
//! ```rust,no_run
//! let mut server = RpcServer::new(state.clone());
//! server.add_rpc(Box::new(rpcs::AddName::server()));
//! server.serve("127.0.0.1:5959").await;
//! ```
//!
//!
//! Elsewhere, to call it, use the `call_client` function with access to the RPC
//! ```rust,no_run
//! let addr = "127.0.0.1:5959";
//! let name = String::from("Gaspode the wonder dog");
//! pirates::call_client(addr, name, rpcs::AddName::client()).await;
//! ```

mod client;
mod core;
pub mod error;
mod rpc_types;
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

#[cfg(test)]
mod tests {
    use crate::client::call_client;
    use crate::core::{Rpc, RpcImpl, RpcName};
    use crate::error::RpcResult;
    use crate::server::RpcServer;
    use crate::RpcDefinition;
    use serde::{Deserialize, Serialize};
    use std::fmt::{Display, Formatter};
    use std::sync::{Arc, Mutex};

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

        let hello_world_rpc = make_hello_world_rpc();
        let get_i_rpc = make_get_i_rpc();
        let incr_i_rpc = IncrIRpc::client();

        let mut rpc_results = None;
        let mut client_call_task = tokio::spawn(async move {
            let r1 = call_client(addr, "foo".into(), hello_world_rpc.clone())
                .await
                .unwrap();
            let r2 = call_client(addr, (), get_i_rpc.clone()).await.unwrap();
            call_client(addr, (), incr_i_rpc.clone()).await.unwrap();
            let r3 = call_client(addr, (), get_i_rpc).await.unwrap();
            call_client(addr, (), incr_i_rpc).await.unwrap();
            let r4 = call_client(addr, "bar".into(), hello_world_rpc)
                .await
                .unwrap();
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
        let expecting: String = "Hello world: 3:\"foo\"".into();
        assert_eq!(expecting, hello_world_1);
        assert_eq!(3usize, get_i_1);
        assert_eq!(4usize, get_i_2);
        let expecting2: String = "Hello world: 5:\"bar\"".into();
        assert_eq!(expecting2, hello_world_2);
    }
}
