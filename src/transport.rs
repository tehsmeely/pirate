use crate::core::RpcName;
use crate::error::{RpcError, RpcResult};
use crate::example::HelloWorldRpcName;
use crate::{Bytes, OwnedBytes};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::marker::PhantomData;

#[async_trait]
pub trait InternalTransport {
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
        println!(
            "Sending {} Bytes:  {:?}",
            package_bytes.len(),
            package_bytes
        );
        let response_bytes = self
            .internal_transport
            .send_and_wait_for_response(&package_bytes)
            .await;
        Ok(response_bytes)
    }

    pub async fn receive_query(&mut self) -> RpcResult<ReceivedQuery<Name>> {
        if let Some(bytes) = self.internal_transport.receive().await {
            println!("Received {} Bytes:  {:?}", bytes.len(), bytes);
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

    pub async fn respond(&mut self, bytes: Bytes<'_>) -> RpcResult<()> {
        println!("Responding with {} Bytes: {:?}", bytes.len(), bytes);
        self.internal_transport.send(bytes).await;
        Ok(())
    }
}

pub struct CannedTestingTransport {
    pub always_respond_with: String,
    pub receive_times: usize,
}

#[async_trait]
impl InternalTransport for CannedTestingTransport {
    async fn send(&mut self, _b: Bytes<'_>) {}

    async fn send_and_wait_for_response(&mut self, _b: Bytes<'_>) -> OwnedBytes {
        serde_pickle::to_vec(&self.always_respond_with, serde_pickle::SerOptions::new()).unwrap()
    }

    async fn receive(&mut self) -> Option<OwnedBytes> {
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
    async fn send(&mut self, b: Bytes<'_>) {
        use tokio::io::AsyncWriteExt;
        println!(">> Just sending {} bytes", b.len());
        self.stream.write_all(b).await;
    }

    async fn send_and_wait_for_response(&mut self, b: Bytes<'_>) -> OwnedBytes {
        println!(">> Sending, before waiting for response");
        self.send(b).await;
        println!(">> Sent, waiting for response...");
        let r = self.receive().await.unwrap();
        println!(">> Got response.");
        r
    }

    async fn receive(&mut self) -> Option<OwnedBytes> {
        use tokio::io::AsyncReadExt;
        let mut buf = [0u8; 1024];
        println!(">> Receiving");
        let len = match self.stream.read(&mut buf).await {
            Ok(bytes_received) => {
                println!(">> Received {} bytes", bytes_received);
                bytes_received
            }
            Err(e) => {
                println!(">> Error reading: {:?}", e);
                0
            }
        };
        Some(buf[0..len].to_vec())
    }
}
