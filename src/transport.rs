use crate::core::RpcName;
use crate::error::{RpcError, RpcResult};

use crate::{Bytes, OwnedBytes};
use async_trait::async_trait;
use log::debug;
use serde::{Deserialize, Serialize};
use std::fmt::Formatter;
use std::marker::PhantomData;

#[derive(Debug)]
pub enum TransportError {
    SendError(String),
    ReceiveError(String),
    ConnectError(String),
}
impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportError::SendError(s) => write!(f, "SendError({})", s),
            TransportError::ReceiveError(s) => write!(f, "ReceiveError({})", s),
            TransportError::ConnectError(s) => write!(f, "ConnectError({})", s),
        }
    }
}
impl std::error::Error for TransportError {}
impl TransportError {
    fn io_send(e: std::io::Error) -> Self {
        Self::SendError(format!("{:?}", e))
    }
    fn io_receive(e: std::io::Error) -> Self {
        Self::ReceiveError(format!("{:?}", e))
    }
}

#[async_trait]
pub trait InternalTransport {
    async fn send(&mut self, b: Bytes<'_>) -> Result<(), TransportError>;
    async fn send_and_wait_for_response(
        &mut self,
        b: Bytes<'_>,
    ) -> Result<OwnedBytes, TransportError>;
    async fn receive(&mut self) -> Result<OwnedBytes, TransportError>;
}

#[derive(Serialize, Deserialize)]
struct TransportPackage<'a> {
    #[serde(borrow)]
    name_bytes: Bytes<'a>,
    #[serde(borrow)]
    query_bytes: Bytes<'a>,
}
#[derive(Serialize, Deserialize)]
struct TransportPackageOwned {
    name_bytes: OwnedBytes,
    query_bytes: OwnedBytes,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::HelloWorldRpcName;
    #[test]
    fn transport_package_round_trip() {
        let name = HelloWorldRpcName::HelloWorld;
        let query = String::from("Foo");

        let deo = serde_pickle::DeOptions::new();
        let sero = serde_pickle::SerOptions::new();

        let name_bytes = serde_pickle::to_vec(&name, sero.clone()).unwrap();
        let query_bytes = serde_pickle::to_vec(&query, sero.clone()).unwrap();

        let package = TransportPackage {
            name_bytes: &name_bytes,
            query_bytes: &query_bytes,
        };

        let package_bytes = serde_pickle::to_vec(&package, sero).unwrap();

        let package2: TransportPackageOwned =
            serde_pickle::from_slice(&package_bytes, deo.clone()).unwrap();

        let name2: HelloWorldRpcName =
            serde_pickle::from_slice(&package2.name_bytes, deo.clone()).unwrap();
        let query2: String = serde_pickle::from_slice(&package2.query_bytes, deo).unwrap();

        assert_eq!(name, name2);
        assert_eq!(query, query2);
    }
}

pub struct ReceivedQuery<Name: RpcName> {
    pub name: Name,
    pub query_bytes: OwnedBytes,
}

pub struct Transport<I, Name> {
    internal_transport: I,
    name: PhantomData<Name>,
}

impl<I: InternalTransport, Name: RpcName> Transport<I, Name> {
    pub fn new(internal_transport: I) -> Self {
        Self {
            internal_transport,
            name: PhantomData::default(),
        }
    }
    pub async fn send_query(
        &mut self,
        query_bytes: Bytes<'_>,
        rpc_name: &Name,
    ) -> RpcResult<OwnedBytes> {
        let name_bytes =
            serde_pickle::ser::to_vec(&rpc_name, serde_pickle::SerOptions::new()).unwrap();
        let package = TransportPackage {
            name_bytes: &name_bytes,
            query_bytes,
        };
        let package_bytes =
            serde_pickle::ser::to_vec(&package, serde_pickle::SerOptions::new()).unwrap();
        debug!(
            "Transport sending {} Bytes:  {:?}",
            package_bytes.len(),
            package_bytes
        );
        self.internal_transport
            .send_and_wait_for_response(&package_bytes)
            .await
            .map_err(Into::into)
    }

    pub async fn receive_query(&mut self) -> RpcResult<ReceivedQuery<Name>> {
        match self.internal_transport.receive().await {
            Ok(bytes) => {
                debug!("Transport {} Bytes:  {:?}", bytes.len(), bytes);
                let package: TransportPackageOwned =
                    serde_pickle::de::from_slice(&bytes, serde_pickle::DeOptions::new()).unwrap();
                let name = serde_pickle::de::from_slice(
                    &package.name_bytes,
                    serde_pickle::DeOptions::new(),
                )
                .unwrap();
                Ok(ReceivedQuery {
                    name,
                    query_bytes: package.query_bytes,
                })
            }
            Err(rpc_error) => Err(RpcError::TransportError(rpc_error)),
        }
    }

    pub async fn respond(&mut self, bytes: Bytes<'_>) -> RpcResult<()> {
        self.internal_transport
            .send(bytes)
            .await
            .map_err(|e| RpcError::TransportError(e))
    }
}

pub struct CannedTestingTransport {
    pub always_respond_with: String,
    pub receive_times: usize,
}

#[async_trait]
impl InternalTransport for CannedTestingTransport {
    async fn send(&mut self, _b: Bytes<'_>) -> Result<(), TransportError> {
        Ok(())
    }

    async fn send_and_wait_for_response(
        &mut self,
        _b: Bytes<'_>,
    ) -> Result<OwnedBytes, TransportError> {
        Ok(
            serde_pickle::to_vec(&self.always_respond_with, serde_pickle::SerOptions::new())
                .unwrap(),
        )
    }

    async fn receive(&mut self) -> Result<OwnedBytes, TransportError> {
        if self.receive_times > 0 {
            self.receive_times -= 1;
            Ok(
                serde_pickle::to_vec(&self.always_respond_with, serde_pickle::SerOptions::new())
                    .unwrap(),
            )
        } else {
            Err(TransportError::ReceiveError(String::from(
                "Run out of receive count",
            )))
        }
    }
}

pub struct TcpTransport {
    stream: tokio::net::TcpStream,
}

impl TcpTransport {
    pub fn new(stream: tokio::net::TcpStream) -> Self {
        Self { stream }
    }
}

#[async_trait]
impl InternalTransport for TcpTransport {
    async fn send(&mut self, b: Bytes<'_>) -> Result<(), TransportError> {
        use tokio::io::AsyncWriteExt;
        // TODO, handle error case in writing
        self.stream
            .write_all(b)
            .await
            .map_err(TransportError::io_send)
    }

    async fn send_and_wait_for_response(
        &mut self,
        b: Bytes<'_>,
    ) -> Result<OwnedBytes, TransportError> {
        self.send(b).await?;
        self.receive().await
    }

    async fn receive(&mut self) -> Result<OwnedBytes, TransportError> {
        use tokio::io::AsyncReadExt;
        let mut buf = [0u8; 1024];
        let len = match self.stream.read(&mut buf).await {
            Ok(bytes_received) => bytes_received,
            Err(e) => {
                return Err(TransportError::io_receive(e));
            }
        };
        Ok(buf[0..len].to_vec())
    }
}
