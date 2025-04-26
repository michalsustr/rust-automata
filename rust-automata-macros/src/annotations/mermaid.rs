use crate::parser;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

#[cfg(feature = "mermaid")]
fn transition_label(tr: &parser::Transition) -> String {
    use crate::annotations::doc_link;
    use crate::parser::guard_expr_to_string;
    use crate::util;
    use crate::GUARD_PREFIX;
    use crate::HANDLE_PREFIX;
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
        let guard_str = guard_expr_to_string(g, &|path| {
            let guard_id = util::key(path);
            format!(
                "<a href='#method.{}'>{}</a>",
                guard_id,
                guard_id.replace(GUARD_PREFIX, "")
            )
        });
        label.push_str(&format!(
            "{0}&nbsp;{1}",
            if label.is_empty() { "" } else { "<br>" },
            guard_str
        ));
    }
    if let Some(ref h) = tr.handler {
        label.push_str(&format!(
            "{0}↪️&nbsp;<a href='#method.{h}'>{1}</a>",
            if label.is_empty() { "" } else { "<br>" },
            h.to_string().replace(HANDLE_PREFIX, "")
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
    writeln!(
        md,
        "///    classDef selfLoop fill:#eee,stroke-width:0px,shape:rectangle,margin:0,padding:0"
    )
    .unwrap();
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
    for (i, tr) in m.transitions.iter().enumerate() {
        let from = util::last(&tr.from_state);
        let to = util::last(&tr.to_state);
        let label = transition_label(tr);
        if from == to {
            writeln!(md, "///    state \"{label}\" as tran_{from}_{to}_{i}").unwrap();
            writeln!(md, "///    class tran_{from}_{to}_{i} selfLoop").unwrap();
            writeln!(md, "///    {from} --> tran_{from}_{to}_{i}").unwrap();
            writeln!(md, "///    tran_{from}_{to}_{i} --> {from}").unwrap();
        } else {
            writeln!(md, "///    {from} --> {to}: {label}").unwrap();
        }
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
