use crate::core::RpcName;
use crate::error::RpcResult;
use crate::{Bytes, OwnedBytes};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::TcpStream;

pub trait InternalTransport {
    fn send(&mut self, b: Bytes);
    fn send_and_wait_for_response(&mut self, b: Bytes) -> OwnedBytes;
    fn receive(&mut self) -> Option<OwnedBytes>;
}

#[derive(Serialize, Deserialize)]
struct TransportPackage<'a> {
    #[serde(borrow)]
    name_bytes: Bytes<'a>,
    #[serde(borrow)]
    query_bytes: Bytes<'a>,
}

pub struct Transport<I: InternalTransport> {
    internal_transport: I,
}

impl<I: InternalTransport> Transport<I> {
    pub fn new(internal_transport: I) -> Self {
        Self { internal_transport }
    }
    pub fn send_query(
        &mut self,
        query_bytes: Bytes,
        rpc_name: &impl RpcName,
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
            .send_and_wait_for_response(&package_bytes);
        Ok(response_bytes)
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

pub struct TcpTransport {
    stream: TcpStream,
}

impl InternalTransport for TcpTransport {
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
