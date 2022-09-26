use crate::core::RpcName;
use crate::error::{RpcError, RpcResult};
use crate::example::HelloWorldRpcName;
use crate::{Bytes, OwnedBytes};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::marker::PhantomData;

pub trait InternalTransport {
    fn send(&mut self, b: Bytes);
    fn send_and_wait_for_response(&mut self, b: Bytes) -> OwnedBytes;
    fn receive(&mut self) -> Option<OwnedBytes>;
}

#[async_trait]
pub trait AsyncInternalTransport {
    async fn send(&mut self, b: Bytes<'_>);
    async fn send_and_wait_for_response(&mut self, b: Bytes<'_>) -> OwnedBytes;
    async fn receive(&mut self) -> Option<OwnedBytes>;
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

        let package_bytes = serde_pickle::to_vec(&package, sero.clone()).unwrap();

        let package2: TransportPackageOwned =
            serde_pickle::from_slice(&package_bytes, deo.clone()).unwrap();

        let name2: HelloWorldRpcName =
            serde_pickle::from_slice(&package2.name_bytes, deo.clone()).unwrap();
        let query2: String = serde_pickle::from_slice(&package2.query_bytes, deo.clone()).unwrap();

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
    _name: PhantomData<Name>,
}

impl<I: AsyncInternalTransport, Name: RpcName> Transport<I, Name> {
    pub fn new(internal_transport: I) -> Self {
        Self {
            internal_transport,
            _name: PhantomData::default(),
        }
    }
    pub async fn send_query_a(
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
        let response_bytes = self
            .internal_transport
            .send_and_wait_for_response(&package_bytes)
            .await;
        Ok(response_bytes)
    }

    pub async fn receive_query_a(&mut self) -> RpcResult<ReceivedQuery<Name>> {
        if let Some(bytes) = self.internal_transport.receive().await {
            let package: TransportPackageOwned =
                serde_pickle::de::from_slice(&bytes, serde_pickle::DeOptions::new()).unwrap();
            let name =
                serde_pickle::de::from_slice(&package.name_bytes, serde_pickle::DeOptions::new())
                    .unwrap();
            Ok(ReceivedQuery {
                name,
                query_bytes: package.query_bytes,
            })
        } else {
            //TODO Rework Custom Error
            Err(RpcError::Custom("Got no bytes".into()))
        }
    }

    pub async fn respond_a(&mut self, bytes: Bytes<'_>) -> RpcResult<()> {
        self.internal_transport.send(bytes);
        Ok(())
    }
}

/*
pub struct SyncTransport<I: InternalTransport> {
    internal_transport: I,
}
*/

impl<I: InternalTransport, Name: RpcName> Transport<I, Name> {
    pub fn new_sync(internal_transport: I) -> Self {
        Self {
            internal_transport,
            _name: PhantomData::default(),
        }
    }
    pub fn send_query(&mut self, query_bytes: Bytes, rpc_name: &Name) -> RpcResult<OwnedBytes> {
        let name_bytes =
            serde_pickle::ser::to_vec(&rpc_name, serde_pickle::SerOptions::new()).unwrap();
        let package = TransportPackage {
            name_bytes: &name_bytes,
            query_bytes,
        };
        let package_bytes =
            serde_pickle::ser::to_vec(&package, serde_pickle::SerOptions::new()).unwrap();
        let response_bytes = self
            .internal_transport
            .send_and_wait_for_response(&package_bytes);
        Ok(response_bytes)
    }

    pub fn receive_query(&mut self) -> RpcResult<ReceivedQuery<Name>> {
        if let Some(bytes) = self.internal_transport.receive() {
            let package: TransportPackage =
                serde_pickle::de::from_slice(&bytes, serde_pickle::DeOptions::new()).unwrap();
            let name =
                serde_pickle::de::from_slice(package.name_bytes, serde_pickle::DeOptions::new())
                    .unwrap();
            Ok(ReceivedQuery {
                name,
                query_bytes: package.query_bytes.to_vec(),
            })
        } else {
            //TODO Rework Custom Error
            Err(RpcError::Custom("Got no bytes".into()))
        }
    }
}

pub struct CannedTestingTransport {
    pub always_respond_with: String,
    pub receive_times: usize,
}

impl InternalTransport for CannedTestingTransport {
    fn send(&mut self, b: Bytes) {}

    fn send_and_wait_for_response(&mut self, b: Bytes) -> OwnedBytes {
        serde_pickle::to_vec(&self.always_respond_with, serde_pickle::SerOptions::new()).unwrap()
    }

    fn receive(&mut self) -> Option<OwnedBytes> {
        if self.receive_times > 0 {
            self.receive_times -= 1;
            Some(
                serde_pickle::to_vec(&self.always_respond_with, serde_pickle::SerOptions::new())
                    .unwrap(),
            )
        } else {
            None
        }
    }
}

pub struct SyncTcpTransport {
    stream: std::net::TcpStream,
}

impl InternalTransport for SyncTcpTransport {
    fn send(&mut self, b: Bytes) {
        self.stream.write_all(b);
    }

    fn send_and_wait_for_response(&mut self, b: Bytes) -> OwnedBytes {
        self.send(b);
        self.receive().unwrap()
    }

    fn receive(&mut self) -> Option<OwnedBytes> {
        let mut buf = [0u8; 1024];
        match self.stream.read(&mut buf) {
            Ok(bytes_received) => println!("Received {} bytes", bytes_received),
            Err(e) => println!("Error reading: {:?}", e),
        }
        Some(buf.to_vec())
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
impl AsyncInternalTransport for TcpTransport {
    async fn send(&mut self, b: Bytes<'_>) {
        use tokio::io::AsyncWriteExt;
        self.stream.write_all(b).await;
    }

    async fn send_and_wait_for_response(&mut self, b: Bytes<'_>) -> OwnedBytes {
        self.send(b).await;
        self.receive().await.unwrap()
    }

    async fn receive(&mut self) -> Option<OwnedBytes> {
        use tokio::io::AsyncReadExt;
        let mut buf = [0u8; 1024];
        match self.stream.read(&mut buf).await {
            Ok(bytes_received) => println!("Received {} bytes", bytes_received),
            Err(e) => println!("Error reading: {:?}", e),
        }
        Some(buf.to_vec())
    }
}
