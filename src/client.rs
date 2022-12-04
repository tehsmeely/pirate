use crate::core::{Rpc, RpcName, RpcType};
use crate::error::{RpcError, RpcResult};
use crate::transport::{
    InternalTransport, TcpTransport, Transport, TransportConfig, TransportError,
};

/// An [RpcClient] encapsulates an Rpc and allows it to be called, providing a [Transport]
/// a convenience function, [call_client] is provided which wraps this type and uses the
/// [TcpTransport] transport
pub struct RpcClient<Name: RpcName, Q: RpcType, R: RpcType> {
    rpc: Rpc<Name, Q, R>,
}

impl<'de, Name: RpcName, Q: RpcType, R: RpcType> RpcClient<Name, Q, R> {
    pub fn new(rpc: Rpc<Name, Q, R>) -> Self {
        Self { rpc }
    }

    /// Call the rpc, using the specified [Transport] to connect to the server
    pub async fn call(
        &self,
        query: Q,
        transport: &mut Transport<impl InternalTransport, Name>,
    ) -> RpcResult<R> {
        let query_bytes = transport.config.serialize(&query);
        let result_bytes = transport.send_query(&query_bytes, &self.rpc.name).await?;
        Ok(transport.config.deserialize(&result_bytes))
    }
}

/// Basic client call function using the [TpcTransport] internal transport with [TransportConfig::Pickle]
pub async fn call_client<Name: RpcName, Q: RpcType, R: RpcType>(
    addr: &str,
    q: Q,
    rpc: Rpc<Name, Q, R>,
) -> RpcResult<R> {
    let mut transport = {
        let l = match tokio::net::TcpStream::connect(addr).await {
            Ok(client_stream) => {
                let tcp_transport = TcpTransport::new(client_stream);
                Ok(Transport::new(tcp_transport, TransportConfig::default()))
            }
            Err(e) => Err(e),
        };
        l
    }
    .map_err(|e| RpcError::TransportError(TransportError::ConnectError(format!("{}", e))))?;

    let rpc_client = RpcClient::new(rpc);

    rpc_client.call(q, &mut transport).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::make_hello_world_rpc;
    use crate::transport::CannedTestingTransport;

    #[tokio::test]
    async fn client_test() {
        let internal_transport = CannedTestingTransport {
            always_respond_with: "Foo-Bar".to_string(),
            receive_times: 0,
        };
        let mut transport = Transport::new(internal_transport, Default::default());

        let rpc_client = RpcClient {
            rpc: make_hello_world_rpc(),
        };

        let result = rpc_client.call("Foo".into(), &mut transport).await.unwrap();

        assert_eq!(String::from("Foo-Bar"), result);
    }
}
