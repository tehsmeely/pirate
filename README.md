# Pirates!
![pirate](resources/pirate.png) *Rust ArrrrPC Lib* 

[![CI](https://github.com/tehsmeely/pirates/actions/workflows/ci.yml/badge.svg)](https://github.com/tehsmeely/pirates/actions/workflows/ci.yml)
[![Current Crates.io Version](https://img.shields.io/crates/v/pirates.svg)](https://crates.io/crates/pirates)


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
    let name = String::from("Gaspode the wonder dog");
    pirate::call_client(addr, name, rpcs::AddName::client()).await;

    let names = call_client(addr, (), rpcs::GetNames::client()).await;
    assert_eq!(vec![String::from("Gaspode the wonder dog")], names);
```

## Documentation

TBC

## Examples

And example name server is available in `example/`.
This produces a CLI binary from which you can host the server and then query it
separately to add and print names. See the README in that directory for more info


## License

TBD?