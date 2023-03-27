use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::core::{RpcName, StoredRpc};
use crate::error::{RpcError, RpcResult};
use crate::transport::{TcpTransport, Transport, TransportConfig};
use crate::OwnedBytes;
use log::{debug, error, info, warn};

pub struct RpcServer<S, Name>
where
    Name: RpcName,
{
    state: Arc<Mutex<S>>,
    rpcs: HashMap<Name, Box<dyn StoredRpc<S, Name>>>,
    transport_config: TransportConfig,
}

impl<S, Name> RpcServer<S, Name>
where
    Name: RpcName,
{
    pub fn new(state: Arc<Mutex<S>>, transport_config: TransportConfig) -> Self {
        Self {
            state,
            rpcs: HashMap::new(),
            transport_config,
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
        debug!("Server called by rpc {}", incoming_name);
        match self.rpcs.get(incoming_name) {
            Some(rpc_impl) => {
                let result_bytes = {
                    let mut state = self.state.lock().unwrap();
                    rpc_impl.call_of_bytes(
                        incoming_bytes,
                        &self.transport_config.wire_config,
                        &mut state,
                    )?
                };
                Ok(result_bytes)
            }
            None => Err(RpcError::Custom(format!(
                "Rpc not found: {}",
                incoming_name
            ))),
        }
    }

    async fn handle_connection(&self, tcp_stream: tokio::net::TcpStream) -> RpcResult<()> {
        debug!("Handling connection: {:?}", tcp_stream);
        let mut transport = {
            let async_trans = TcpTransport::new(tcp_stream);
            Transport::new(async_trans, self.transport_config.clone())
        };
        let received_query = transport.receive_query().await?;
        let result_bytes = self
            .call(&received_query.query_bytes, &received_query.name)
            .unwrap();
        transport.respond(&result_bytes).await
    }

    pub async fn serve(&self, listen_on: impl tokio::net::ToSocketAddrs + std::fmt::Display) {
        info!("Starting server on {}", listen_on);
        let listener = tokio::net::TcpListener::bind(listen_on).await.unwrap();
        loop {
            match listener.accept().await {
                Ok((tcp_stream, _from)) => {
                    let connection_result = self.handle_connection(tcp_stream).await;
                    if let Err(e) = connection_result {
                        warn!("Error handling connection: {}", e);
                    }
                }
                Err(e) => error!("TCP Listener error: {}", e),
            }
        }
    }
}
