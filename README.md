# Pirates!
*Rust ArrrrPC Lib*

[![Build Status](https://github.com/tehsmeely/pirate/workflows/CI/badge.svg)](https://github.com/tehsmeely/pirate/actions)
[![Current Crates.io Version]()](https://crates.io/crates/pirates)


Pirates, a simple and straightforward interface for serving and calling RPCs from async rust programs


Define an RPC
```rust
    pub struct AddName {}
    #[pirate::rpc_definition]
    impl AddName {
        fn name() -> RpcId {
            RpcId::AddName
        }
        fn implement(state: &mut ServerState, query: String) -> RpcResult<()> {
            state.names.push(query);
            Ok(())
        }
    }
```

Serve it
```rust
    let mut server = RpcServer::new(state.clone());
    server.add_rpc(Box::new(rpcs::AddName::server()));
```

Call it
```rust
    let addr = "200.1.3.7:5959";
    let name = "Gaspode the wonder dog";
    pirate::call_client(addr, name, rpcs::AddName::client()).await;
```

## Documentation

TBC

## Examples

And example name server is available in `example/`.
This produces a CLI binary from which you can host the server and then query it
separately to add and print names. See the README in that directory for more info


## License

TBD?