use attribute_derive::Attribute;
use pluralizer::pluralize;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use std::cmp::Ordering;
use syn::spanned::Spanned;
use syn::{parse_macro_input, Error, FnArg, ItemFn, Pat, ReturnType, Type};

#[derive(Debug, Attribute)]
#[attribute(ident = editor_node)]
struct AttributeInput {
    name: Ident,
    outputs: Vec<Ident>,
}

#[derive(Default)]
struct NodeData<'a> {
    inputs: Vec<(&'a Type, &'a Ident)>,
    outputs: Vec<(&'a Type, &'a Ident)>,
}

fn process(attr: TokenStream, data: ItemFn) -> Result<TokenStream, Error> {
    let mut node = NodeData::default();

    let crate_name = match crate_name("dbe") {
        Ok(found) => match found {
            FoundCrate::Itself => "crate".to_string(),
            FoundCrate::Name(name) => name,
        },
        Err(error) => return Err(Error::new(Span::call_site(), error.to_string())),
    };
    let crate_ident = format_ident!("{crate_name}");

    let mut uses_commands = false;

    for (i, arg) in data.sig.inputs.iter().enumerate() {
        let FnArg::Typed(pat) = arg else {
            return Err(Error::new(arg.span(), "`self` argument is not allowed"));
        };
        let Pat::Ident(ident) = &*pat.pat else {
            return Err(Error::new(
                pat.span(),
                format!(
                    "Got {:?} where identifier was expected (while parsing argument {})",
                    pat.pat,
                    pat.to_token_stream()
                ),
            ));
        };

        if i == 0 && ident.ident == "commands" {
            uses_commands = true;
            continue;
        }

        node.inputs.push((&pat.ty, &ident.ident))
    }

    let attribute = AttributeInput::from_args(dbg!(attr).into())?;

    let mut is_tuple = false;
    let mut return_types = vec![];
    if let ReturnType::Type(_, ty) = &data.sig.output {
        match &**ty {
            Type::Tuple(items) => {
                is_tuple = true;
                return_types.reserve(items.elems.len());
                for ty in &items.elems {
                    return_types.push(ty);
                }
            }
            ty => return_types.push(ty),
        }
    }

    node.outputs = match return_types.len().cmp(&attribute.outputs.len()) {
        Ordering::Less => {
            let error = format!(
                "There are {} named {}, but the function only returns {}",
                return_types.len(),
                pluralize("output", attribute.outputs.len() as isize, false),
                pluralize("value", return_types.len() as isize, true),
            );
            return Err(Error::new(data.sig.output.span(), error));
        }
        Ordering::Greater => {
            let error = format!(
                "Function return {}, but only {} are named",
                pluralize("value", return_types.len() as isize, true),
                pluralize("output", attribute.outputs.len() as isize, true),
            );
            return Err(Error::new(data.sig.output.span(), error));
        }
        _ => return_types
            .into_iter()
            .zip(attribute.outputs.iter())
            .collect(),
    };
    let data_struct = {
        let struct_name = attribute.name;
        let input_ports = node.inputs.iter().map(|(ty, ident)| {
            let name = ident.to_string();
            quote_spanned!(ident.span() => #crate_ident::nodes::create_input_port::<#ty>(graph, user_state, node_id, #name.into());)
        });

        let output_ports = node
            .outputs
            .iter()
            .map(|(ty, ident)| {
                let name = ident.to_string();
                quote_spanned!(ident.span() => #crate_ident::nodes::create_output_port::<#ty>(graph, user_state, node_id, #name.into());)
            });

        let input_eval_ports = node.inputs.iter().map(|(ty, ident)| {
            let name = ident.to_string();
            quote_spanned!(ident.span() => let #ident = #crate_ident::evaluator::evaluate_input_as::<#ty>(graph, outputs_cache, commands, node_id, #name)?;)
        });

        let input_args = node.inputs.iter().map(|e| e.1);

        let function_name = &data.sig.ident;

        let output_bindings = if is_tuple {
            let idents = node.outputs.iter().map(|(_, ident)| ident);
            quote! {
                (#(#idents,)*)
            }
        } else {
            node.outputs
                .first()
                .map(|(_, ident)| {
                    quote! {
                        #ident
                    }
                })
                .unwrap_or_else(|| {
                    quote! {
                        _
                    }
                })
        };

        let output_eval_ports = node.outputs.iter().map(|(_, ident)| {
            let name = ident.to_string();
            quote_spanned!(ident.span() => #crate_ident::evaluator::populate_output(graph, outputs_cache, node_id, #name, std::convert::Into::<EValue>::into(#ident))?;)
        });

        let commands = if uses_commands {
            quote!(commands,)
        } else {
            quote!()
        };

        quote! {
            #[derive(Debug, Copy, Clone, Default, serde::Serialize, serde::Deserialize)]
            pub struct #struct_name;

            impl #crate_ident::nodes::EditorNode for #struct_name {
                fn create_ports(
                    &self,
                    graph: &mut #crate_ident::EditorGraph,
                    user_state: &mut #crate_ident::MyGraphState,
                    node_id: egui_node_graph::NodeId,
                ) {
                    #(#input_ports)*
                    #(#output_ports)*
                }

                fn evaluate(
                    &self,
                    graph: &EditorGraph,
                    outputs_cache: &mut OutputsCache,
                    commands: &mut Vec<Command>,
                    node_id: NodeId,
                ) -> anyhow::Result<()> {
                    #(#input_eval_ports)*
                    let #output_bindings = #function_name(#commands #(#input_args,)*);
                    #(#output_eval_ports)*
                    Ok(())
                }
            }
        }
    };

    // Build the output, possibly using quasi-quotation
    let expanded = quote! {
        #data_struct
        #data
    };

    // Hand the output tokens back to the compiler
    Ok(TokenStream::from(expanded))
}

#[proc_macro_attribute]
pub fn editor_node(attr: TokenStream, input: TokenStream) -> TokenStream {
    let data: ItemFn = parse_macro_input!(input);
    match process(attr, data) {
        Ok(data) => data,
        Err(err) => err.to_compile_error().into(),
    }
}
