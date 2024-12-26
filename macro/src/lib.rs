#![allow(non_snake_case)]
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use serde::{Deserialize, Serialize};
use syn::parse::Parser;
use syn::Stmt;
use syn::{parse_macro_input, ItemStruct};
use syn::{ImplItem, ItemImpl};

#[derive(Debug, Default, Serialize, Deserialize)]
struct ComponentConfig {
    port: Option<ComponentPortConfig>,
    // serde default on bool is false
    #[serde(default)]
    is_primary: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ComponentPortConfig {
    input: Option<Vec<(String, String)>>,
    output: Option<Vec<(String, String)>>,
    #[serde(default)]
    clock: bool,
}
#[proc_macro_attribute]
pub fn ComponentAttribute(config: TokenStream, input: TokenStream) -> TokenStream {
    let mut item_struct = parse_macro_input!(input as ItemStruct);
    let struct_name = item_struct.ident.clone();

    let component_impl_ts = quote! {
        impl Component for #struct_name {
            fn init(&mut self) { self.init_impl(); }

            fn reset(&mut self) { self.reset_impl(); }

            fn poll_recv(&mut self) { self.poll_impl(); }

            fn get_component_id(&self) -> ComponentId { self.component_id }

            fn as_any(&self) -> &dyn std::any::Any { self }
        }
    }
    .into();

    let mut component_impl_item = parse_macro_input!(component_impl_ts as ItemImpl);

    let component_config: ComponentConfig = serde_json::from_str(&config.to_string()).unwrap();

    // Every component should have these values
    let mut extended_field = vec![
        syn::Field::parse_named
            .parse2(quote! { component_id: ComponentId })
            .unwrap(),
        syn::Field::parse_named
            .parse2(quote! { sim_manager: Arc<SimManager> })
            .unwrap(),
        syn::Field::parse_named
            .parse2(quote! { ack_sender: Sender<EventId> })
            .unwrap(),
    ];

    if component_config.is_primary {
        let _ = component_impl_item
            .items
            .iter_mut()
            .map(|item| {
                if let ImplItem::Fn(func) = item {
                    if func.sig.ident == format_ident!("init") {
                        func.block.stmts.push(syn::parse_quote! {self.sim_manager.register_do_not_end(self.get_component_id());})
                    }
                }
            })
            .collect::<Vec<_>>();
    }

    if let Some(port) = component_config.port {
        // If the component has clock, we need to
        // 1. register the clock with the sim manager
        // 2. call on_clock when clock ticks
        if port.clock {
            extended_field.extend(vec![
                syn::Field::parse_named
                    .parse2(quote! { clock_sender: Output })
                    .unwrap(),
                syn::Field::parse_named
                    .parse2(quote! { clock_receiver: Input })
                    .unwrap(),
            ]);
            let _ = component_impl_item
                .items
                .iter_mut()
                .map(|item| {
                    if let ImplItem::Fn(func) = item {
                        if func.sig.ident == format_ident!("init") {
                            func.block.stmts.push(syn::parse_quote! {self.sim_manager
                            .register_clock_tick(self.clock_sender.clone());})
                        } else if func.sig.ident == format_ident!("poll_recv") {
                            push_clock_recv_stmt(&mut func.block.stmts)
                        }
                    }
                })
                .collect::<Vec<_>>();
        }
        // For each input port, it will have
        // 1. a mpsc receiver
        // 2. a variable holding the value
        // 3. a corresponding try_recv in poll_recv, acks and calls on_comb if successful
        port.input.map(|input| {
            input
                .iter()
                .map(|(port_name, port_type)| {
                    let net = format_ident!("{}", port_name);
                    let net_old = format_ident!("{}_old", port_name);
                    let receiver = format_ident!("{}_receiver", port_name);
                    let net_type: proc_macro2::TokenStream = port_type.parse().unwrap();
                    extended_field.extend(vec![
                        syn::Field::parse_named
                            .parse2(quote! { #receiver: Input })
                            .unwrap(),
                        syn::Field::parse_named
                            .parse2(quote! { pub #net: #net_type })
                            .unwrap(),
                        syn::Field::parse_named
                            .parse2(quote! { pub #net_old: #net_type })
                            .unwrap(),
                    ]);
                    let _ = component_impl_item
                        .items
                        .iter_mut()
                        .map(|item| {
                            if let ImplItem::Fn(func) = item {
                                if func.sig.ident == format_ident!("poll_recv") {
                                    push_comb_recv_stmt(&mut func.block.stmts, port_name, port_type)
                                }
                            }
                        })
                        .collect::<Vec<_>>();
                })
                .collect::<Vec<_>>()
        });
        // We assume outputs are not registered
        port.output.map(|output| {
            output
                .iter()
                .map(|(port_name, _port_type)| {
                    let receiver = format_ident!("{}", port_name);
                    extended_field.extend(vec![syn::Field::parse_named
                        .parse2(quote! { #receiver: Output })
                        .unwrap()])
                })
                .collect::<Vec<_>>()
        });
    };

    if let syn::Fields::Named(ref mut fields) = item_struct.fields {
        fields.named.extend(extended_field);
    }

    quote! {
        #item_struct

        #component_impl_item
    }
    .into()
}

fn push_clock_recv_stmt(stmt: &mut Vec<Stmt>) {
    let receiver = format_ident!("clock_receiver");

    stmt.push(syn::parse_quote! {
        if let Ok(event) = self.#receiver.try_recv() {
            self.on_clock();
            self.on_comb();
            ack!(self, event.get_event_id());
        }
    })
}

fn push_comb_recv_stmt(stmt: &mut Vec<Stmt>, port_name: &str, port_type: &str) {
    let net = format_ident!("{}", port_name);
    let net_old = format_ident!("{}_old", port_name);
    let receiver = format_ident!("{}_receiver", port_name);
    let net_type: proc_macro2::TokenStream = port_type.parse().unwrap();

    stmt.push(syn::parse_quote! {
        if let Ok(event) = self.#receiver.try_recv() {
            self.#net = get_inner::<#net_type>(&*event);
            if self.#net != self.#net_old {
                self.on_comb();
            }
            self.#net_old = self.#net;
            ack!(self, event.get_event_id());
        }
    })
}
