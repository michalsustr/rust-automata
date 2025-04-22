use crate::parser;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

#[cfg(feature = "mermaid")]
fn transition_label(tr: &parser::Transition) -> String {
    use crate::util;

    let mut label = String::new();
    if let Some(ref i) = tr.input {
        let ev = util::last(i);
        label = format!("{}?", ev);
    }
    if let Some(ref out) = tr.output {
        let out_id = util::last(out);
        label.push_str(&format!("\\n{}!", out_id));
    }
    // if let Some(ref g) = tr.guard {
    //     label.push_str(&format!("\\nguard&nbsp;{}", g));
    // }
    // if let Some(ref h) = tr.handler {
    //     label.push_str(&format!("\\nvia&nbsp;{}", h));
    // }
    label
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
        writeln!(dot, "///    {} [href=\"{}\"];", id, doc_link(path)).unwrap();
    }

    // // Transitions
    for tr in &m.transitions {
        let from = util::last(&tr.from_state);
        let to = util::last(&tr.to_state);
        let label = transition_label(tr);
        if !label.is_empty() {
            writeln!(dot, "///    {} -> {} [label=\"{}\"];", from, to, label).unwrap();
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
