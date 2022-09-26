use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

use serde_pickle::SerOptions;

use crate::core::{RpcName, StoredRpc};
use crate::error::{RpcError, RpcResult};
use crate::example::QR;
use crate::transport::{AsyncInternalTransport, TcpTransport, Transport};
use crate::OwnedBytes;

pub struct RPCServer<S, Name, R>
where
    R: StoredRpc<S>,
    Name: RpcName,
{
    state: Arc<S>,
    rpcs: HashMap<Name, Box<R>>,
}

impl<S, Name, R> RPCServer<S, Name, R>
where
    R: StoredRpc<S>,
    Name: RpcName,
{
    pub fn new(state: Arc<S>) -> Self {
        Self {
            state,
            rpcs: HashMap::new(),
        }
    }

    pub fn add_rpc(&mut self, name: Name, rpc_impl: Box<R>) {
        self.rpcs.insert(name, rpc_impl);
    }

    pub fn call(&self, incoming_bytes: &[u8], incoming_name: &Name) -> RpcResult<OwnedBytes> {
        println!(". Server.call");
        match self.rpcs.get(incoming_name) {
            Some(rpc_impl) => {
                println!("Rpc found: {}", incoming_name);
                let result_bytes = rpc_impl.call_of_bytes(incoming_bytes, &self.state)?;
                println!(
                    "RPC Result -> {} Bytes: {:?}",
                    result_bytes.len(),
                    result_bytes
                );
                Ok(result_bytes)
            }
            None => {
                println!("Rpc not found: {}", incoming_name);
                Err(RpcError::Custom(format!(
                    "Rpc not found: {}",
                    incoming_name
                )))
            }
        }
    }

    async fn handle_connection(&self, tcp_stream: tokio::net::TcpStream) -> RpcResult<()> {
        println!(". Handle connection");
        let mut transport = {
            let async_trans = TcpTransport::new(tcp_stream);
            Transport::new(async_trans)
        };
        let received_query = transport.receive_query_a().await?;
        println!("received query from connection");
        let result_bytes = self
            .call(&received_query.query_bytes, &received_query.name)
            .unwrap();
        println!(
            "Handle connection: got {} result bytes to respond with",
            result_bytes.len()
        );
        transport.respond_a(&result_bytes).await
    }

    pub async fn serve(&self, listen_on: impl tokio::net::ToSocketAddrs + std::fmt::Display) {
        println!("Starting server on {}", listen_on);
        let listener = tokio::net::TcpListener::bind(listen_on).await.unwrap();

        println!("Listener started");
        loop {
            match listener.accept().await {
                Ok((tcp_stream, _from)) => {
                    let connection_result = self.handle_connection(tcp_stream).await;
                    if let Err(e) = connection_result {
                        println!("Error handling connection: {}", e);
                    }
                }
                Err(e) => println!("TCP Listener error: {}", e),
            }
        }
    }
}

/*
mod async_rpc {
    pub struct RPCServer<S, Name, R>
    where
        R: StoredRpc<S>,
        Name: RpcName,
    {
        state: Arc<S>,
        rpcs: HashMap<Name, Box<R>>,
    }

    impl<S, Name, R> RPCServer<S, Name, R>
    where
        R: StoredRpc<S>,
        Name: RpcName,
    {
        pub fn new(state: Arc<S>) -> Self {
            Self {
                state,
                rpcs: HashMap::new(),
            }
        }

        pub fn add_rpc(&mut self, name: Name, rpc_impl: Box<R>) {
            self.rpcs.insert(name, rpc_impl);
        }

        pub fn call(&self, incoming_bytes: &[u8], incoming_name: &Name, incoming_type_id: TypeId) {
            match self.rpcs.get(incoming_name) {
                Some(rpc_impl) => {
                    let mut print_writer = PrintWriter;
                    rpc_impl.call_of_bytes(incoming_bytes, &self.state, &mut print_writer);
                }
                None => {
                    println!("Rpc not found: {}", incoming_name)
                }
            }
        }
    }
}
*/
