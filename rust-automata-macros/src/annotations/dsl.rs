use crate::parser;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

#[cfg(feature = "dsl")]
pub fn attr(m: &parser::MachineAttr) -> TokenStream2 {
    use crate::util;
    use crate::util::key;
    use std::fmt::Write;

    let mut dsl = String::new();
    writeln!(dsl, "///```text").unwrap();

    // Write inputs
    write!(dsl, "///inputs(").unwrap();
    for (i, path) in m.inputs.iter().enumerate() {
        let id = util::last(path);
        write!(dsl, "{}", id).unwrap();
        if i < m.inputs.len() - 1 {
            write!(dsl, ", ").unwrap();
        }
    }
    writeln!(dsl, "),").unwrap();

    // Write states
    write!(dsl, "///states(").unwrap();
    for (i, path) in m.states.iter().enumerate() {
        let id = util::last(path);
        write!(dsl, "{}", id).unwrap();
        if i < m.states.len() - 1 {
            write!(dsl, ", ").unwrap();
        }
    }
    writeln!(dsl, "),").unwrap();

    // Write outputs
    write!(dsl, "///outputs(").unwrap();
    for (i, path) in m.outputs.iter().enumerate() {
        let id = util::last(path);
        write!(dsl, "{}", id).unwrap();
        if i < m.outputs.len() - 1 {
            write!(dsl, ", ").unwrap();
        }
    }
    writeln!(dsl, "),").unwrap();

    // Calculate alignment widths
    let max_left_side_length = m
        .transitions
        .iter()
        .map(|tr| {
            let from = util::last(&tr.from_state).to_string();
            let mut length = from.len() + 2; // +2 for the parentheses

            if let Some(ref input) = tr.input {
                let input_id = util::last(input).to_string();
                length += input_id.len() + 2; // +2 for ", "
            }

            length
        })
        .max()
        .unwrap_or(0);

    // Calculate max length of the middle part (to state + output if any)
    let max_middle_length = m
        .transitions
        .iter()
        .map(|tr| {
            let to = util::last(&tr.to_state).to_string();
            let mut length = to.len() + 7; // +4 for " -> (" and "," and ")"

            if let Some(ref output) = tr.output {
                let output_id = util::last(output).to_string();
                length += output_id.len();
            }

            length
        })
        .max()
        .unwrap_or(0);

    // Write transitions with aligned arrows and guards/handlers
    writeln!(dsl, "///transitions(").unwrap();
    for (i, tr) in m.transitions.iter().enumerate() {
        let from = util::last(&tr.from_state);
        let to = util::last(&tr.to_state);

        // Format left side (from state + input)
        let mut left_side = format!("({})", from);
        if let Some(ref input) = tr.input {
            let input_id = util::last(input);
            left_side = format!("({}, {})", from, input_id);
        }

        // Format middle part (to state + output)
        let mut middle_part = format!("-> ({})", to);
        if let Some(ref output) = tr.output {
            let output_id = util::last(output);
            middle_part = format!("-> ({}, {})", to, output_id);
        }

        // Calculate paddings - use saturating_sub to avoid overflow
        let left_padding = " ".repeat(max_left_side_length.saturating_sub(left_side.len()));
        let middle_padding = " ".repeat(max_middle_length.saturating_sub(middle_part.len()));

        // Write the transition with proper alignment
        write!(dsl, "///  {}{} {}", left_side, left_padding, middle_part).unwrap();

        // Add guard or handler with alignment
        if tr.guard.is_some() || tr.handler.is_some() {
            write!(dsl, "{}", middle_padding).unwrap();

            // Add guard if present
            if let Some(ref guard) = tr.guard {
                write!(dsl, " : {}", guard).unwrap();
            }

            // Add handler if present
            if let Some(ref handler) = tr.handler {
                write!(dsl, " = {}", handler).unwrap();
            }
        }

        if i < m.transitions.len() - 1 {
            writeln!(dsl, ",").unwrap();
        } else {
            writeln!(dsl).unwrap();
        }
    }
    writeln!(dsl, "///),").unwrap();

    // Write derives
    if !m.derives.is_empty() {
        write!(dsl, "///derive(").unwrap();
        for (i, derive) in m.derives.iter().enumerate() {
            write!(dsl, "{}", key(derive)).unwrap();
            if i < m.derives.len() - 1 {
                write!(dsl, ", ").unwrap();
            }
        }
        writeln!(dsl, ")").unwrap();
    }

    writeln!(dsl, "///```").unwrap();
    writeln!(dsl, "///").unwrap();

    let tokens: TokenStream2 = dsl.parse().unwrap();
    quote! { #tokens }
}

#[cfg(not(feature = "dsl"))]
pub fn attr(_: &parser::MachineAttr) -> TokenStream2 {
    quote!()
}
