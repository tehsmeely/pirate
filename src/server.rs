use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

use serde_pickle::SerOptions;

use crate::core::{RpcName, StoredRpc};
use crate::example::QR;
use crate::PrintWriter;

pub struct RPCServer<S, Name, R>
where
    R: StoredRpc<S>,
    Name: RpcName,
{
    state: Arc<S>,
    rpcs: HashMap<Name, (TypeId, Box<R>)>,
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
        let type_id = rpc_impl.internal_type_id();
        self.rpcs.insert(name, (type_id, rpc_impl));
    }

    pub fn call(&self, incoming_bytes: &[u8], incoming_name: &Name, incoming_type_id: TypeId) {
        match self.rpcs.get(incoming_name) {
            Some((server_side_type_id, rpc_impl)) => {
                if *server_side_type_id == incoming_type_id {
                    let mut print_writer = PrintWriter;
                    rpc_impl.call_of_bytes(incoming_bytes, &self.state, &mut print_writer);
                }
            }
            None => {
                println!("Rpc not found: {}", incoming_name)
            }
        }
    }
}
