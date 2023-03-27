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
pub use crate::transport::InternalTransport;
pub use crate::transport::Transport;
pub use crate::transport::TransportWireConfig;

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
    use crate::transport::{TransportConfig, TransportWireConfig};
    use crate::RpcDefinition;
    use serde::{Deserialize, Serialize};
    use std::fmt::{Display, Formatter};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    pub struct HelloWorldState {
        pub i: usize,
    }

    #[derive(Clone, Hash, Eq, PartialEq, Debug, Serialize, Deserialize)]
    pub enum HelloWorldRpcName {
        HelloWorld,
        GetI,
        IncrI,
        MassiveRpc,
        PreciseRpc,
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

    pub struct MassiveRpc {}
    impl MassiveRpc {
        fn implement(_state: &mut HelloWorldState, query: usize) -> RpcResult<Vec<u32>> {
            let mut v = Vec::new();
            let mut i = 0;
            while i < query {
                v.push(1u32);
                i += 1;
            }
            Ok(v)
        }
    }
    impl RpcDefinition<HelloWorldRpcName, HelloWorldState, usize, Vec<u32>> for MassiveRpc {
        fn client() -> Rpc<HelloWorldRpcName, usize, Vec<u32>> {
            Rpc::new(HelloWorldRpcName::MassiveRpc)
        }

        fn server() -> RpcImpl<HelloWorldRpcName, HelloWorldState, usize, Vec<u32>> {
            RpcImpl::new(HelloWorldRpcName::MassiveRpc, Box::new(Self::implement))
        }
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct PrecisePayload {
        bulk_bytes: Vec<u32>,
        padding: Vec<bool>,
    }
    pub struct PreciseRpc {}
    impl PreciseRpc {
        fn implement(_state: &mut HelloWorldState, query: usize) -> RpcResult<PrecisePayload> {
            let mut v = Vec::new();
            let mut i = 0;
            while i < query {
                v.push(1u32);
                i += 1;
            }
            let mut padding = Vec::new();
            let mut i = 0;
            while i < 128 + 7 {
                padding.push(true);
                i += 1;
            }
            Ok(PrecisePayload {
                bulk_bytes: v,
                padding,
            })
        }
    }
    impl RpcDefinition<HelloWorldRpcName, HelloWorldState, usize, PrecisePayload> for PreciseRpc {
        fn client() -> Rpc<HelloWorldRpcName, usize, PrecisePayload> {
            Rpc::new(HelloWorldRpcName::PreciseRpc)
        }

        fn server() -> RpcImpl<HelloWorldRpcName, HelloWorldState, usize, PrecisePayload> {
            RpcImpl::new(HelloWorldRpcName::PreciseRpc, Box::new(Self::implement))
        }
    }

    #[test]
    fn just_server_test() {
        let state = HelloWorldState { i: 3 };
        let transport_config = TransportConfig {
            rcv_timeout: Duration::from_secs(3),
            wire_config: TransportWireConfig::Pickle(
                serde_pickle::DeOptions::new(),
                serde_pickle::SerOptions::new(),
            ),
        };
        let mut server = RpcServer::new(Arc::new(Mutex::new(state)), transport_config);
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
    async fn regular_server() {
        // Server setup
        println!("Server Setup");
        let state = HelloWorldState { i: 3 };
        let state_ref = Arc::new(Mutex::new(state));
        let transport_config = TransportConfig::default();
        let mut server = RpcServer::new(state_ref, transport_config);
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

    #[tokio::test]
    async fn big_rpc_server() {
        // Server setup
        println!("Server Setup");
        let state = HelloWorldState { i: 3 };
        let state_ref = Arc::new(Mutex::new(state));
        let mut server = RpcServer::new(state_ref, TransportConfig::default());
        server.add_rpc(Box::new(MassiveRpc::server()));
        server.add_rpc(Box::new(PreciseRpc::server()));
        let addr = "127.0.0.1:5556";

        let massive_rpc_client = MassiveRpc::client();
        let precise_rpc_client = PreciseRpc::client();

        let num_bulk = 170;
        let mut rpc_results = None;
        let mut client_call_task = tokio::spawn(async move {
            let result = call_client(addr, 2000, massive_rpc_client.clone())
                .await
                .unwrap();
            //
            let result2: PrecisePayload = call_client(addr, num_bulk, precise_rpc_client)
                .await
                .unwrap();
            (result.len(), result2.bulk_bytes.len())
        });

        while rpc_results.is_none() {
            println!(".");
            tokio::select! {
                _ = server.serve(addr) => {},
                client_output = &mut client_call_task => {rpc_results = Some(client_output)},
            }
        }

        let (massive_len, slightly_smaller_len) = rpc_results.unwrap().unwrap();
        assert_eq!(massive_len, 2000);
        // which returns 10010 bytes = 8000 bytes + 2010 overhead?

        assert_eq!(slightly_smaller_len, num_bulk);
        // which returns 1286 bytes = 1024 + 262 overhead
    }
}
