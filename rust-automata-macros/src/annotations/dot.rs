use crate::parser;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

#[cfg(feature = "mermaid")]
fn transition_label(tr: &parser::Transition) -> String {
    use crate::util;
    use crate::annotations::doc_link;
    use crate::GUARD_PREFIX;
    use crate::HANDLE_PREFIX;


    let mut label = String::new();
    if let Some(ref i) = tr.input {
        let ev = util::last(i);
        label = format!("<tr><td href=\"{}\" align=\"left\" cellpadding=\"0\" cellspacing=\"0\"><u>{}</u>?</td></tr>", doc_link(i), ev);
    }
    if let Some(ref out) = tr.output {
        let out_id = util::last(out);
        label.push_str(&format!("<tr><td href=\"{}\" align=\"left\" cellpadding=\"0\" cellspacing=\"0\"><u>{}</u>!</td></tr>", doc_link(out), out_id));
    }
    if let Some(ref g) = tr.guard {
        label.push_str(&format!("<tr><td href=\"#method.{g}\" align=\"left\" cellpadding=\"0\" cellspacing=\"0\"><u>{0}</u></td></tr>", g.to_string().replace(GUARD_PREFIX, "")));
    }
    if let Some(ref h) = tr.handler {
        label.push_str(&format!("<tr><td href=\"#method.{h}\" align=\"left\" cellpadding=\"0\" cellspacing=\"0\"><u>{0}</u></td></tr>", h.to_string().replace(HANDLE_PREFIX, "")));
    }
    if label.is_empty() {   
        "".to_string()
    } else {
        format!("<<table border=\"0\" cellpadding=\"0\" cellspacing=\"0\">{}</table>>", label)
    }
}

#[cfg(feature = "dot")]
pub fn attr(m: &parser::MachineAttr) -> TokenStream2 {
    use crate::annotations::doc_link;
    use crate::util;
    use std::fmt::Write;

    let state_paths = &m.states;
    let state_ids: Vec<_> = state_paths.iter().map(util::last).collect();
    let initial = state_ids.first().unwrap();

    let mut dot = String::new();
    writeln!(dot, "///```dot").unwrap();
    writeln!(dot, "///digraph automaton {{").unwrap();
    writeln!(dot, "///    rankdir=LR;").unwrap();
    writeln!(dot, "///    node [shape=box, style=rounded];").unwrap();

    // Initial state marker
    writeln!(
        dot,
        "///    start [shape=circle, label=\"\", style=filled, fillcolor=black, width=0.25];"
    )
    .unwrap();
    writeln!(dot, "///    start -> {};", initial).unwrap();

    // State nodes with clickable links
    for (id, path) in state_ids.iter().zip(state_paths) {
        writeln!(dot, "///    {} [href=\"{}\", label=<<u>{}</u>>];", id, doc_link(path), id).unwrap();
    }

    // // Transitions
    for (i, tr) in m.transitions.iter().enumerate() {
        let from = util::last(&tr.from_state);
        let to = util::last(&tr.to_state);
        let label = transition_label(tr);
        if !label.is_empty() {
            writeln!(dot, "///    {from} -> tran_{from}_{to}_{i} [arrowhead=none];").unwrap();
            writeln!(dot, "///    tran_{from}_{to}_{i} [label={}, margin=0, bgcolor=white,fixedsize=shape, shape=none, style=none, fontsize=10];", label).unwrap();
            writeln!(dot, "///    tran_{from}_{to}_{i} -> {to};").unwrap();
            // writeln!(dot, "///    {from} -> {to} [label={}, margin=0, shape=none, fontsize=10];", label).unwrap();
        } else {
            writeln!(dot, "///    {} -> {};", from, to).unwrap();
        }
    }

    writeln!(dot, "///}}").unwrap();
    writeln!(dot, "///```").unwrap();
    writeln!(dot, "///").unwrap();

    let tokens: TokenStream2 = dot.parse().unwrap();
    quote! { #tokens }
}

#[cfg(not(feature = "dot"))]
pub fn attr(_: &parser::MachineAttr) -> TokenStream2 {
    quote!()
}
