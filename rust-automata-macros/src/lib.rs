//! Attribute‑style DSL for defining finite‑state machines.
//!
//! See the [`rust-automata` crate](https://docs.rs/rust-automata/) for more details.
//!
//! Documentation features:
//! - `"mermaid"`: embed a clickable Mermaid state diagram.
//! - `"dot"`: embed a clickable Graphviz state diagram.
//! - `"dsl"`: (re)generate a DSL for the machine.

#![recursion_limit = "256"]

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use std::collections::BTreeSet;
use syn::{parse_macro_input, Ident, ItemStruct, Path};

mod parser;
use parser::{MachineAttr, Transition};

mod annotations;

// Handlers that have this prefix receive states and inputs and should return a state and an output.
const HANDLE_PREFIX: &str = "handle_";
// Guards that have this prefix receive state reference and should return a boolean.
const GUARD_PREFIX: &str = "guard_";

mod util {
    use super::*;
    use heck::ToSnakeCase;

    /// `CamelCase` → `snake_case` ident.
    pub fn snake(id: &Ident) -> Ident {
        Ident::new(&id.to_string().to_snake_case(), id.span())
    }

    /// Last segment of `syn::Path` → `snake_case` ident.
    pub fn snake_path(p: &Path) -> Ident {
        snake(last(p))
    }

    /// Last identifier of a path (`states::Open` → `Open`)
    pub fn last(p: &Path) -> &Ident {
        &p.segments.last().unwrap().ident
    }

    /// A unique string key for set membership (`states::Open`)
    pub fn key(p: &Path) -> String {
        p.segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::")
    }

    /// Strip a trailing `Machine` from a type name if present.
    pub fn strip_machine(id: &Ident) -> String {
        let s = id.to_string();
        s.strip_suffix("Machine").unwrap_or(&s).to_owned()
    }

    pub fn compile_error_if(condition: bool, message: &str) -> Option<TokenStream2> {
        condition.then(|| quote! { compile_error!(#message); })
    }
}

use util::*;

mod building_blocks {
    use super::*;
    /// Generate a signature check for a transition.
    pub fn make_handler_sig_check(tr: &Transition, machine_ident: &Ident) -> TokenStream2 {
        match tr.handler {
            Some(ref handler) if handler.to_string().starts_with(HANDLE_PREFIX) => {
                let state_ty = &tr.from_state;
                let to_ty = &tr.to_state;

                match (tr.input.as_ref(), tr.output.as_ref()) {
                    (Some(inp_ty), Some(out_ty)) => quote! {
                        super::#machine_ident::#handler as fn(&mut super::#machine_ident, super::#state_ty, super::#inp_ty) -> (super::#to_ty, super::#out_ty);
                    },
                    (Some(inp_ty), None) => quote! {
                        super::#machine_ident::#handler as fn(&mut super::#machine_ident, super::#state_ty, super::#inp_ty) -> super::#to_ty;
                    },
                    (None, Some(out_ty)) => quote! {
                        super::#machine_ident::#handler as fn(&mut super::#machine_ident, super::#state_ty) -> (super::#to_ty, super::#out_ty);
                    },
                    (None, None) => quote! {
                        super::#machine_ident::#handler as fn(&mut super::#machine_ident, super::#state_ty) -> super::#to_ty;
                    },
                }
            }
            _ => quote! {},
        }
    }

    pub fn instantiate_vals(tr: &parser::Transition, state_var: &Ident) -> TokenStream2 {
        let next_val = if key(&tr.from_state) == key(&tr.to_state) {
            quote! { #state_var }
        } else {
            let to_path = &tr.to_state;
            quote! { super::#to_path::default() }
        };
        let out_val = if tr.output.is_some() {
            let out_path = tr.output.as_ref().unwrap();
            quote! { super::#out_path::default() }
        } else {
            quote! { () }
        };

        quote! {
            next_val = #next_val;
            out_val = #out_val;
        }
    }

    pub fn build_handler_code(
        tr: &Transition,
        state_var: &Ident,
        input_var: &Ident,
    ) -> (TokenStream2, TokenStream2) {
        match &tr.handler {
            Some(handler) if handler.to_string().starts_with(HANDLE_PREFIX) => {
                let has_input = tr.input.is_some();
                let has_output = tr.output.is_some();
                let call = match (has_input, has_output) {
                    (true, true) => {
                        quote! { (next_val, out_val) = self.#handler(#state_var, #input_var); }
                    }
                    (true, false) => {
                        quote! { next_val = self.#handler(#state_var, #input_var); out_val = (); }
                    }
                    (false, true) => quote! { (next_val, out_val) = self.#handler(#state_var); },
                    (false, false) => {
                        quote! { next_val = self.#handler(#state_var); out_val = (); }
                    }
                };
                (call, quote! {})
            }
            Some(callback) => (
                quote! { self.#callback(); },
                instantiate_vals(tr, state_var),
            ),
            None => (quote! {}, instantiate_vals(tr, state_var)),
        }
    }

    pub fn build_guard_code(tr: &Transition, state_var: &Ident) -> TokenStream2 {
        match tr.guard {
            // explicit guard that uses values
            Some(ref guard) if guard.to_string().starts_with(GUARD_PREFIX) => {
                quote! { if (&self).#guard(&#state_var) }
            }
            // guard exists but doesn't require values
            Some(ref guard) => quote! { if (&self).#guard() },
            // no guard
            None => quote! {},
        }
    }

    /// Validates a MachineAttr for correctness.
    /// Returns a TokenStream2 containing any compile errors found.
    pub fn validate_machine_attr(m: &MachineAttr) -> TokenStream2 {
        let states_set: BTreeSet<String> = m.states.iter().map(key).collect();
        let inputs_set: BTreeSet<String> = m.inputs.iter().map(key).collect();
        let outputs_set: BTreeSet<String> = m.outputs.iter().map(key).collect();

        if states_set.is_empty() {
            return quote! { compile_error!("No states are defined"); };
        }
        let errors = m.transitions.iter().flat_map(|tr| {
            let tr_descr = tr.to_string();
            vec![
                compile_error_if(
                    !states_set.contains(&key(&tr.from_state)),
                    &format!("Unknown state: {} in {}", key(&tr.from_state), tr_descr),
                ),
                compile_error_if(
                    !states_set.contains(&key(&tr.to_state)),
                    &format!("Unknown state: {} in {}", key(&tr.to_state), tr_descr),
                ),
                tr.input.as_ref().and_then(|i| {
                    compile_error_if(
                        !inputs_set.contains(&key(i)),
                        &format!("Unknown input: {} in {}", key(i), tr_descr),
                    )
                }),
                tr.output.as_ref().and_then(|o| {
                    compile_error_if(
                        !outputs_set.contains(&key(o)),
                        &format!("Unknown output: {} in {}", key(o), tr_descr),
                    )
                }),
                tr.handler.as_ref().and_then(|h| {
                    compile_error_if(
                        h.to_string().starts_with(GUARD_PREFIX),
                        &format!("Handler cannot start with guard_ prefix: {}", h),
                    )
                }),
                tr.guard.as_ref().and_then(|g| {
                    compile_error_if(
                        g.to_string().starts_with(HANDLE_PREFIX),
                        &format!("Guard cannot start with handle_ prefix: {}", g),
                    )
                }),
            ]
            .into_iter()
            .flatten()
        });
        quote! { #(#errors)* }
    }

    // A helper function that maps an iterable collection of identifiers to our enum match arms.
    pub fn generate_enum_matches(ids: &Vec<&Ident>) -> Vec<proc_macro2::TokenStream> {
        ids.iter()
            .enumerate()
            .map(|(idx, id)| {
                let idx = idx + 1;
                quote! {
                    Self::#id(_) => rust_automata::EnumId::new(#idx)
                }
            })
            .collect()
    }

    pub fn build_getters(alphabet_paths: &[Path]) -> TokenStream2 {
        let getters = alphabet_paths.iter().map(|p| {
            let id = last(p);
            let direct_fn = snake_path(p);
            let is_fn = format_ident!("is_{}", direct_fn);
            let maybe_fn = format_ident!("maybe_{}", direct_fn);
            quote! {
                pub fn #is_fn(&self) -> bool {
                    matches!(self, Self::#id(_))
                }
                pub fn #maybe_fn(&self) -> Option<&super::#p> {
                    if let Self::#id(o) = self { Some(o) } else { None }
                }
                pub fn #direct_fn(&self) -> &super::#p {
                    self.#maybe_fn().expect(&format!("No such symbol like {}", stringify!(#direct_fn)))
                }
            }
        });
        quote! { #( #getters )* }
    }

    pub fn build_conversions(enum_ident: &Ident, alphabet_paths: &[Path]) -> TokenStream2 {
        let conversions = alphabet_paths.iter().enumerate().map(|(idx, p)| {
            let id = last(p);
            quote! {
                impl From<super::#p> for #enum_ident {
                    fn from(i: super::#p) -> Self { Self::#id(i) }
                }
                impl rust_automata::Enumerated<#enum_ident> for super::#p {
                    fn enum_id() -> rust_automata::EnumId<#enum_ident> {
                        rust_automata::EnumId::new(#idx + 1)
                    }
                }
                impl From<#enum_ident> for super::#p {
                    fn from(o: #enum_ident) -> Self {
                        match o {
                            #enum_ident::#id(v) => v,
                            _ => panic!("Invalid symbol requested from {}", stringify!(#p)),
                        }
                    }
                }
            }
        });

        quote! { #( #conversions )* }
    }

    pub fn build_alphabet(
        derive_attr: &TokenStream2,
        enum_ident: &Ident,
        alphabet_paths: &Vec<Path>,
    ) -> TokenStream2 {
        let alphabet_ids: Vec<_> = alphabet_paths.iter().map(last).collect();
        let enumerable_ids_alphabet = generate_enum_matches(&alphabet_ids);
        let alphabet_getters = build_getters(alphabet_paths);
        let alphabet_conversions = build_conversions(enum_ident, alphabet_paths);
        quote! {
            #derive_attr
            pub enum #enum_ident {
                Nothing(()),
                #( #alphabet_ids ( super::#alphabet_paths ) ),*
            }
            impl rust_automata::Alphabet for #enum_ident {
                fn nothing() -> Self { Self::Nothing(()) }
                fn any(&self) -> bool { !matches!(self, Self::Nothing(_)) }
            }
            impl rust_automata::Enumerable<#enum_ident> for #enum_ident {
                fn enum_id(&self) -> rust_automata::EnumId<#enum_ident> {
                    match self {
                        Self::Nothing(_) => rust_automata::EnumId::new(0),
                        #( #enumerable_ids_alphabet ),*
                    }
                }
            }
            impl #enum_ident {
                #alphabet_getters
            }
            #alphabet_conversions
        }
    }

    pub fn build_set(
        derive_attr: &TokenStream2,
        enum_ident: &Ident,
        state_paths: &Vec<Path>,
    ) -> TokenStream2 {
        let state_ids: Vec<_> = state_paths.iter().map(last).collect();
        let enumerable_ids_states = generate_enum_matches(&state_ids);
        let state_getters = build_getters(state_paths);
        let state_conversions = build_conversions(enum_ident, state_paths);

        quote! {
            #derive_attr
            pub enum #enum_ident {
                Failure(()),
                 #( #state_ids ( super::#state_paths ) ),*
            }
            impl rust_automata::StateTrait for #enum_ident {
                fn failure() -> Self { Self::Failure(()) }
                fn is_failure(&self) -> bool { matches!(self, Self::Failure(_)) }
            }
            impl rust_automata::Enumerable<#enum_ident> for #enum_ident {
                fn enum_id(&self) -> rust_automata::EnumId<#enum_ident> {
                    match self {
                        Self::Failure(_) => rust_automata::EnumId::new(0),
                        #( #enumerable_ids_states ),*
                    }
                }
            }
            impl #enum_ident {
                #state_getters
            }
            #state_conversions
        }
    }

    pub fn compute_symbol_index(
        needle: Option<&syn::Path>,
        symbols: &[syn::Path],
        tr: &parser::Transition,
    ) -> usize {
        match needle {
            Some(symbol) => {
                1 + symbols
                    .iter()
                    .position(|p| key(p) == key(symbol))
                    .unwrap_or_else(|| {
                        panic!("Symbol {} not found in transition: {}", key(symbol), tr);
                    })
            }
            None => 0,
        }
    }
}

/// The main macro for defining automata.
///
/// See [rust-automata](https://github.com/michalsustr/rust-automata) crate for more details.
#[proc_macro_attribute]
pub fn state_machine(attr: TokenStream, item: TokenStream) -> TokenStream {
    use building_blocks::*;

    // Parse attribute + struct
    let m: MachineAttr = parse_macro_input!(attr as MachineAttr);
    let errors = validate_machine_attr(&m);
    if !errors.is_empty() {
        return errors.into();
    }

    // Prepare all the identifiers and lists
    let machine_ts: TokenStream2 = item.clone().into();
    let machine: ItemStruct = parse_macro_input!(item as ItemStruct);
    let machine_ident = machine.ident.clone();
    let vis = machine.vis.clone();
    let base = strip_machine(&machine_ident);
    let internal_mod = format_ident!("internal_{}", base);
    let state_enum_ident = format_ident!("{}State", base);
    let input_enum_ident = format_ident!("{}Input", base);
    let output_enum_ident = format_ident!("{}Output", base);
    let initial_state_ident = &m.states.first().unwrap();
    let nothing_ident = format_ident!("Nothing");
    // pre‑compute frequently‑used lists
    let state_paths = &m.states;
    let input_paths = &m.inputs;
    let output_paths = &m.outputs;
    let derives = &m.derives;

    // Simplify derive attribute generation
    let (derive_attr, derive_struct) = if derives.is_empty() {
        (quote!( #[derive(strum_macros::Display)] ), quote! {})
    } else {
        (
            quote!( #[derive(strum_macros::Display, #( #derives ),* )] ),
            quote! {#[derive(Default, #( #derives ),* )]},
        )
    };

    let maybe_generate_structs = state_paths
        .iter()
        .chain(input_paths.iter())
        .chain(output_paths.iter())
        .filter_map(|p| {
            if m.generate_structs {
                Some(quote! {
                    #derive_struct
                    pub struct #p;
                })
            } else {
                None
            }
        });

    let transition_match_arms = m.transitions.iter().enumerate().map(|(idx, tr)| {
        let from_id = last(&tr.from_state);
        let to_id = last(&tr.to_state);
        let inp_id = tr.input.as_ref().map(last).unwrap_or(&nothing_ident);
        let out_id = tr.output.as_ref().map(last).unwrap_or(&nothing_ident);
        let state_var = format_ident!("state{idx}");
        let input_var = format_ident!("input{idx}");
        let to_path = &tr.to_state;
        let type_declaration = match tr.output {
            Some(ref out_path) => quote! {
                let next_val: super::#to_path;
                let out_val: super::#out_path;
            },
            None => quote! {
                let next_val: super::#to_path;
                let out_val:  ();
            },
        };
        let (transition_call, value_instantiation) = build_handler_code(tr, &state_var, &input_var);
        let guard_call = build_guard_code(tr, &state_var);

        quote! {
            (Self::State::#from_id(#state_var), Self::Input::#inp_id(#input_var)) #guard_call => {
                #type_declaration
                #transition_call
                #value_instantiation
                (
                    Self::State::#to_id(next_val),
                    Self::Output::#out_id(out_val)
                )
            }
        }
    });
    let can_transition_match_arms = m.transitions.iter().enumerate().map(|(idx, tr) | {
        let from_id = last(&tr.from_state);
        let state_var = format_ident!("state{idx}");
        let input_idx: usize = compute_symbol_index(tr.input.as_ref(), input_paths, tr);
        let output_idx: usize = compute_symbol_index(tr.output.as_ref(), output_paths, tr);
        let guard_call = build_guard_code(tr, &state_var);
        quote! {
            (Self::State::#from_id(#state_var), #input_idx) #guard_call => Some(rust_automata::EnumId::new(#output_idx))
        }
    });

    let input_alphabet = build_alphabet(&derive_attr, &input_enum_ident, input_paths);
    let output_alphabet = build_alphabet(&derive_attr, &output_enum_ident, output_paths);
    let state_set = build_set(&derive_attr, &state_enum_ident, state_paths);

    let sig_checks = m
        .transitions
        .iter()
        .map(|tr| make_handler_sig_check(tr, &machine_ident));

    // ────────────────── annotations ──────────────────
    let mermaid_attr = annotations::mermaid_attr(&m);
    let dot_attr = annotations::dot_attr(&m);
    let dsl_attr = annotations::dsl_attr(&m);

    // ────────────────── put everything together ──────────────────
    let output = quote! {
        #mermaid_attr
        #dot_attr
        #dsl_attr
        #machine_ts

        #( #maybe_generate_structs )*

        #[allow(non_snake_case)]
        #[doc(hidden)]
        #vis mod #internal_mod {
            use rust_automata::*;

            #state_set
            #input_alphabet
            #output_alphabet

            impl rust_automata::StateMachineImpl for super::#machine_ident {
                type Input  = #input_enum_ident;
                type State  = #state_enum_ident;
                type Output = #output_enum_ident;
                type InitialState = super::#initial_state_ident;
                fn transition(
                    &mut self,
                    mut state: rust_automata::Takeable<Self::State>,
                    input: Self::Input,
                ) -> (rust_automata::Takeable<Self::State>, Self::Output) {

                    // Make nice error messages
                    #( #sig_checks )*

                    let out = state.borrow_result(|old_state| {
                        match (old_state, input) {
                            #( #transition_match_arms , )*
                            (_, _) => { (Self::State::failure(), Self::Output::nothing()) }
                        }
                    });
                    (state, out)
                }

                fn can_transition(&self, state: &Self::State, input: EnumId<Self::Input>) -> Option<EnumId<Self::Output>> {
                    match (state, input.id) {
                        #( #can_transition_match_arms , )*
                        (_, _) => None,
                    }
                }
            }
        }
    };

    output.into()
}
