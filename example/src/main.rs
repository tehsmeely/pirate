use clap::{arg, value_parser};
use pirates::{call_client, RpcDefinition, RpcName, RpcServer, TransportConfig};
use serde::{Deserialize, Serialize};
use std::fmt::Formatter;
use std::sync::{Arc, Mutex};
use tokio;

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:5858";
    let cmd = clap::Command::new("example")
        .bin_name("pirate_example")
        .subcommand_required(true)
        .subcommand(clap::Command::new("server").about("Start the server"))
        .subcommand(
            clap::Command::new("add-name")
                .about("Add a name to the server")
                .arg(
                    arg!(-n --name <NAME> "name to add")
                        .required(true)
                        .value_parser(value_parser!(String)),
                ),
        )
        .subcommand(
            clap::Command::new("print-names")
                .about("Fetch all names from the server and print them"),
        )
        .get_matches();

    match cmd.subcommand() {
        Some(("server", _)) => {
            server(addr).await;
        }
        Some(("add-name", sub_match)) => {
            let name = sub_match.get_one::<String>("name").unwrap().clone();
            client(addr, CliSelection::Add(name)).await;
        }
        Some(("print-names", _)) => {
            client(addr, CliSelection::Print).await;
        }
        _ => {}
    }
}

struct ServerState {
    names: Vec<String>,
}

#[derive(PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
enum RpcId {
    AddName,
    GetNames,
}
impl std::fmt::Display for RpcId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AddName => write!(f, "AddName"),
            Self::GetNames => write!(f, "GetNames"),
        }
    }
}

impl RpcName for RpcId {}

async fn server(addr: &str) {
    let state = ServerState { names: Vec::new() };
    let state_ref = Arc::new(Mutex::new(state));
    let transport_config = TransportConfig::default();
    let mut server = RpcServer::new(state_ref, transport_config);
    server.add_rpc(Box::new(rpcs::AddName::server()));
    server.add_rpc(Box::new(rpcs::GetNames::server()));
    println!("Serving on {}!", addr);
    server.serve(addr).await;
}

enum CliSelection {
    Add(String),
    Print,
}

async fn client(addr: &str, selection: CliSelection) {
    match selection {
        CliSelection::Add(name) => add_name_cli(addr, name).await,
        CliSelection::Print => print_names_cli(addr).await,
    }
}

async fn add_name_cli(addr: &str, name: String) {
    call_client(addr, name, rpcs::AddName::client())
        .await
        .unwrap();
}
async fn print_names_cli(addr: &str) {
    let names = call_client(addr, (), rpcs::GetNames::client())
        .await
        .unwrap();
    for name in names {
        println!("{}", name);
    }
}

mod rpcs {
    use crate::{RpcId, ServerState};
    use pirates::error::RpcResult;

    pub struct AddName {}
    #[pirates::rpc_definition]
    impl AddName {
        fn name() -> RpcId {
            RpcId::AddName
        }
        fn implement(state: &mut ServerState, query: String) -> RpcResult<()> {
            state.names.push(query);
            Ok(())
        }
    }

    pub struct GetNames {}
    #[pirates::rpc_definition]
    impl GetNames {
        fn name() -> RpcId {
            RpcId::GetNames
        }
        fn implement(state: &mut ServerState, _query: ()) -> RpcResult<Vec<String>> {
            Ok(state.names.clone())
        }
    }
}
