use crate::parser;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

#[cfg(feature = "mermaid")]
fn transition_label(tr: &parser::Transition) -> String {
    use crate::annotations::doc_link;
    use crate::util;

    let mut label = String::new();
    if let Some(ref i) = tr.input {
        let ev = util::last(i);
        label = format!("<a href='{}'>{}?</a>", doc_link(i), ev);
    }
    if let Some(ref out) = tr.output {
        let out_id = util::last(out);
        label.push_str(&format!(
            "{}<a href='{}'>{}!</a>",
            if label.is_empty() { "" } else { "<br>" },
            doc_link(out),
            out_id
        ));
    }
    if let Some(ref g) = tr.guard {
        label.push_str(&format!(
            "{0}guard&nbsp;<a href='#method.{1}'>{1}</a>",
            if label.is_empty() { "" } else { "<br>" },
            g
        ));
    }
    if let Some(ref h) = tr.handler {
        label.push_str(&format!(
            "{0}via&nbsp;<a href='#method.{1}'>{1}</a>",
            if label.is_empty() { "" } else { "<br>" },
            h
        ));
    }
    label
}

#[cfg(feature = "mermaid")]
pub fn attr(m: &parser::MachineAttr) -> TokenStream2 {
    use crate::annotations::doc_link;
    use crate::util;
    use std::fmt::Write;

    let state_paths = &m.states;
    let state_ids: Vec<_> = state_paths.iter().map(util::last).collect();
    let initial = state_ids.first().unwrap();

    let mut md = String::new();
    writeln!(md, "///```mermaid").unwrap();
    writeln!(md, "///stateDiagram-v2").unwrap();
    writeln!(md, "///    [*] --> {}", initial).unwrap();

    // Clickable state aliases
    for (id, path) in state_ids.iter().zip(state_paths) {
        writeln!(
            md,
            "///    state \"<a href='{}'>{}</a>\" as {}",
            doc_link(path),
            id,
            id
        )
        .unwrap();
    }

    // Transitions
    for tr in &m.transitions {
        let from = util::last(&tr.from_state);
        let to = util::last(&tr.to_state);
        let label = transition_label(tr);
        writeln!(md, "///    {} --> {}: {}", from, to, label).unwrap();
    }

    writeln!(md, "///```").unwrap();
    writeln!(md, "///").unwrap();

    let tokens: TokenStream2 = md.parse().unwrap();
    quote! { #[cfg_attr(doc, ::rust_automata::aquamarine)] #tokens }
}

#[cfg(not(feature = "mermaid"))]
pub fn attr(_: &parser::MachineAttr) -> TokenStream2 {
    quote!()
}
