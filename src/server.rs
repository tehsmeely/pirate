
use std::collections::HashMap;
use std::sync::{Arc, Mutex};



use crate::core::{RpcName, StoredRpc};
use crate::error::{RpcError, RpcResult};
use crate::transport::{TcpTransport, Transport};
use crate::OwnedBytes;

pub struct RpcServer<S, Name>
where
    Name: RpcName,
{
    state: Arc<Mutex<S>>,
    rpcs: HashMap<Name, Box<dyn StoredRpc<S, Name>>>,
}

impl<S, Name> RpcServer<S, Name>
where
    Name: RpcName,
{
    pub fn new(state: Arc<Mutex<S>>) -> Self {
        Self {
            state,
            rpcs: HashMap::new(),
        }
    }

    pub fn add_rpc(&mut self, stored_rpc: Box<dyn StoredRpc<S, Name>>) {
        let name = stored_rpc.rpc_name();
        self.rpcs.insert(name, stored_rpc);
    }

    pub(crate) fn call(
        &self,
        incoming_bytes: &[u8],
        incoming_name: &Name,
    ) -> RpcResult<OwnedBytes> {
        println!(". Server.call");
        match self.rpcs.get(incoming_name) {
            Some(rpc_impl) => {
                println!("Rpc found: {}", incoming_name);
                let result_bytes = {
                    let mut state = self.state.lock().unwrap();
                    rpc_impl.call_of_bytes(incoming_bytes, &mut state)?
                };
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
        let received_query = transport.receive_query().await?;
        println!("received query from connection");
        let result_bytes = self
            .call(&received_query.query_bytes, &received_query.name)
            .unwrap();
        println!(
            "Handle connection: got {} result bytes to respond with",
            result_bytes.len()
        );
        transport.respond(&result_bytes).await
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
