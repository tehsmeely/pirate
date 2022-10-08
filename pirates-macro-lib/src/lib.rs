use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, ImplItem, ImplItemMethod, ItemImpl, ReturnType, Type};

/*
The macro takes:

    pub struct RPCIMPL {}
    impl RPCIMPL {
        fn name() -> NAME { ... }
        fn implement(state: &mut STATE, query: QUERY) -> RpcResult<RESPONSE> { ... }
    }

and generates an extra impl block:

    impl RpcDefinition<NAME, STATE, QUERY, RESPONSE> for RPCIMPL {
        fn client() -> Rpc<NAME, QUERY, RESPONSE> {
            Rpc::new(Self::name())
        }

        fn server() -> RpcImpl<NAME, STATE, QUERY, RESPONSE> {
            RpcImpl::new(Self::name(), Box::new(Self::implement))
        }
    }
*/

fn find_fn_by_name<'a, 'b>(name: &'b str, items: &'a Vec<ImplItem>) -> Option<&'a ImplItemMethod> {
    for item in items {
        match item {
            ImplItem::Method(impl_item_method) => {
                if impl_item_method.sig.ident == name {
                    return Some(impl_item_method);
                }
            }
            _ => (),
        };
    }
    None
}

fn unpack_mut_ref(in_: &Type) -> &Type {
    match in_ {
        Type::Reference(type_ref) => &type_ref.elem,
        _ => panic!("Was expecting a ref only"),
    }
}

fn unpack_rpcresult_type(in_: &Type) -> &Type {
    match in_ {
        Type::Path(type_path) => {
            if let syn::PathArguments::AngleBracketed(angle_bracketed_generic_arguments) =
                &type_path.path.segments.first().unwrap().arguments
            {
                if let syn::GenericArgument::Type(ty) =
                    angle_bracketed_generic_arguments.args.first().unwrap()
                {
                    &ty
                } else {
                    panic!("Angle bracketed arg is not type")
                }
            } else {
                panic!("Path is not Angle Bracketed")
            }
        }
        _ => panic!("Was expecting Result<T> type only"),
    }
}

#[proc_macro_attribute]
pub fn rpc_definition(args: TokenStream, item: TokenStream) -> TokenStream {
    let _args = parse_macro_input!(args as AttributeArgs);
    let mut output_tokens = item.clone();
    eprintln!("Original Tokens:\n\n{:?}\n.\n.\n", item);
    let item = parse_macro_input!(item as ItemImpl);

    //fn name() -> NAME { ... }
    let name_fn: &ImplItemMethod =
        find_fn_by_name("name", &item.items).expect("Function 'name' must exist");
    eprintln!("Name Fn: {:?}", name_fn);
    //fn implement(state: &mut STATE, query: QUERY) -> RpcResult<RESPONSE> { ... }
    let implement_fn: &ImplItemMethod =
        find_fn_by_name("implement", &item.items).expect("Function 'implement' must exist");
    eprintln!("Implement Fn: {:?}", implement_fn);

    // Fetch the type identifiers:
    let ty_rpc_impl = item.self_ty;
    let ty_name = match &name_fn.sig.output {
        ReturnType::Default => panic!("Output must be a type"),
        ReturnType::Type(_, ty) => ty,
    };
    let (ty_state, ty_query) = {
        let inputs = &implement_fn.sig.inputs;
        let fst = inputs.first().unwrap();
        let lst = inputs.last().unwrap();
        let ty_state = if let syn::FnArg::Typed(pat_type) = fst {
            &pat_type.ty
        } else {
            panic!("First argument was not a typed one")
        };
        let ty_query = if let syn::FnArg::Typed(pat_type) = lst {
            &pat_type.ty
        } else {
            panic!("last argument was not a typed one")
        };
        (ty_state, ty_query)
    };
    let ty_state = unpack_mut_ref(ty_state);

    let ty_response = match &implement_fn.sig.output {
        ReturnType::Default => panic!("Output must be a type"),
        ReturnType::Type(_, ty) => unpack_rpcresult_type(ty),
    };
    eprintln!("Struct Name: {:?}", ty_rpc_impl);
    eprintln!("Name Type: {:?}", ty_name);
    eprintln!("State Type:{:?}", ty_state);
    eprintln!("Query Type: {:?}", ty_query);
    eprintln!("Response Type: {:?}", ty_response);

    // generate trait impl block
    let new_block: TokenStream = quote! {
        impl pirates::RpcDefinition<#ty_name, #ty_state, #ty_query, #ty_response> for #ty_rpc_impl {
            fn client() -> pirates::Rpc<#ty_name, #ty_query, #ty_response> {
                pirates::Rpc::new(Self::name())
            }

            fn server() -> pirates::RpcImpl<#ty_name, #ty_state, #ty_query, #ty_response> {
                pirates::RpcImpl::new(Self::name(), std::boxed::Box::new(Self::implement))
            }
        }
    }
    .into();

    output_tokens.extend(new_block);
    output_tokens
}
