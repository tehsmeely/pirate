use serde::{Deserialize, Serialize};

use crate::core::{Rpc, RpcName, RpcType};
use crate::error::{RpcError, RpcResult};
use crate::transport::{AsyncInternalTransport, InternalTransport, Transport};
use crate::{Bytes, OwnedBytes};

pub struct RpcClient<Name: RpcName, Q: RpcType, R: RpcType> {
    rpc: Rpc<Name, Q, R>,
}

impl<'de, Name: RpcName, Q: RpcType, R: RpcType> RpcClient<Name, Q, R> {
    pub fn new(rpc: Rpc<Name, Q, R>) -> Self {
        Self { rpc }
    }
    pub fn call(
        &self,
        query: Q,
        transport: &mut Transport<impl InternalTransport, Name>,
    ) -> RpcResult<R> {
        let query_bytes = query.to_bytes()?;
        let result_bytes = transport.send_query(&query_bytes, &self.rpc.name)?;
        R::of_bytes(&result_bytes)
    }
    pub async fn call_a(
        &self,
        query: Q,
        transport: &mut Transport<impl AsyncInternalTransport, Name>,
    ) -> RpcResult<R> {
        let query_bytes = query.to_bytes()?;
        let result_bytes = transport.send_query_a(&query_bytes, &self.rpc.name).await?;
        R::of_bytes(&result_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::example::{make_hello_world_rpc, QR};
    use crate::transport::CannedTestingTransport;

    #[test]
    fn client_test() {
        let internal_transport = CannedTestingTransport {
            always_respond_with: "Foo-Bar".to_string(),
            receive_times: 0,
        };
        let mut transport = Transport::new_sync(internal_transport);

        let rpc_client = RpcClient {
            rpc: make_hello_world_rpc(),
        };

        let result = rpc_client.call(QR("Foo".into()), &mut transport).unwrap();

        assert_eq!(QR("Foo-Bar".into()), result);
    }
}