use syn::punctuated::Punctuated;
use syn::{
    parenthesized,
    parse::{Parse, ParseStream, Result},
    Ident, Path, Token,
};

/// Parsed representation of a single FSM transition line.
///
/// Grammar accepted now:
/// ```text
/// (from_state[, input]) -> (to_state[, output]) [ : guard ] [ = handler ]
/// ```
/// * `guard` and `handler` are optional.
/// * `from_state`, `input`, `to_state`, `output`, `guard`, `handler` are all parsed as `Path`,
///   so module‐qualified identifiers work out of the box.
pub struct Transition {
    pub from_state: Path,
    pub input: Option<Path>,
    pub to_state: Path,
    pub output: Option<Path>,
    pub guard: Option<Ident>,
    pub handler: Option<Ident>,
}

impl Parse for Transition {
    fn parse(input: ParseStream) -> Result<Self> {
        // -------------------------
        // Left‑hand side
        // -------------------------
        let lhs;
        parenthesized!(lhs in input);
        let from_state: Path = lhs.parse()?;
        let input_event: Option<Path> = if lhs.peek(Token![,]) {
            lhs.parse::<Token![,]>()?;
            Some(lhs.parse()?)
        } else {
            None
        };

        // -------------------------
        // Arrow
        // -------------------------
        input.parse::<Token![->]>()?;

        // -------------------------
        // Right‑hand side
        // -------------------------
        let rhs;
        parenthesized!(rhs in input);
        let to_state: Path = rhs.parse()?;
        let output_event: Option<Path> = if rhs.peek(Token![,]) {
            rhs.parse::<Token![,]>()?;
            Some(rhs.parse()?)
        } else {
            None
        };

        // -------------------------
        // Optional guard after ':'
        // -------------------------
        let guard: Option<Ident> = if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            Some(input.parse()?)
        } else {
            None
        };

        // -------------------------
        // Optional handler after '='
        // -------------------------
        let handler: Option<Ident> = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            Some(input.parse()?)
        } else {
            None
        };

        Ok(Self {
            from_state,
            input: input_event,
            to_state,
            output: output_event,
            guard,
            handler,
        })
    }
}

pub fn key(p: &Path) -> String {
    p.segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

use std::fmt;
use std::fmt::Display;

impl Display for Transition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({},{}) -> ({},{}) : {:?} = {:?}",
            key(&self.from_state),
            &self
                .input
                .as_ref()
                .map(key)
                .unwrap_or("NoInput".to_string()),
            key(&self.to_state),
            &self
                .output
                .as_ref()
                .map(key)
                .unwrap_or("NoOutput".to_string()),
            self.guard
                .as_ref()
                .map(|g| g.to_string())
                .unwrap_or("NoGuard".to_string()),
            self.handler
                .as_ref()
                .map(|h| h.to_string())
                .unwrap_or("NoHandler".to_string())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_form() {
        let src = r#"(S1) -> (S2, E1) : guard_xyz = handler_xyz"#;
        let t: Transition = syn::parse_str(src).unwrap();
        assert_eq!(t.guard.unwrap().to_string(), "guard_xyz");
        assert_eq!(t.handler.unwrap().to_string(), "handler_xyz");
    }

    #[test]
    fn parses_minimal_form() {
        let src = "(A) -> (B)";
        let t: Transition = syn::parse_str(src).unwrap();
        assert!(t.input.is_none());
        assert!(t.output.is_none());
        assert!(t.guard.is_none());
        assert!(t.handler.is_none());
    }
}

/// Parsed contents of the whole attribute.
///
/// Grammar (sections may appear in any order)
///
/// ```text
/// section := inputs(..) | states(..) | outputs(..) | transitions(..)
/// attr    := section (, section)*
/// ```
pub struct MachineAttr {
    pub inputs: Vec<Path>,
    pub states: Vec<Path>,
    pub outputs: Vec<Path>,
    pub transitions: Vec<Transition>,
    pub derives: Vec<Path>,
    pub generate_structs: bool,
}

impl Parse for MachineAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut inputs: Option<Vec<Path>> = None;
        let mut states: Option<Vec<Path>> = None;
        let mut outputs: Option<Vec<Path>> = None;
        let mut transitions: Option<Vec<Transition>> = None;
        let mut derives: Option<Vec<Path>> = None;
        let mut generate_structs: Option<bool> = None;
        while !input.is_empty() {
            let section: Ident = input.parse()?;
            let content;
            parenthesized!(content in input);

            match &*section.to_string() {
                "inputs" => {
                    inputs = Some(parse_path_list(&content)?);
                }
                "states" => {
                    states = Some(parse_path_list(&content)?);
                }
                "outputs" => {
                    outputs = Some(parse_path_list(&content)?);
                }
                "transitions" => {
                    transitions = Some(parse_transition_list(&content)?);
                }
                "derive" => {
                    derives = Some(parse_path_list(&content)?);
                }
                "generate_structs" => {
                    generate_structs = Some(parse_bool(&content)?);
                }
                section => return Err(syn::Error::new_spanned(section, "unknown section")),
            }

            // consume optional trailing comma after the section
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self {
            inputs: inputs.unwrap_or_default(),
            states: states.unwrap_or_default(),
            outputs: outputs.unwrap_or_default(),
            transitions: transitions.unwrap_or_default(),
            derives: derives.unwrap_or_default(),
            generate_structs: generate_structs.unwrap_or(false),
        })
    }
}

fn parse_transition_list(input: ParseStream) -> Result<Vec<Transition>> {
    let list: Punctuated<Transition, Token![,]> =
        Punctuated::<Transition, Token![,]>::parse_terminated(input)?;
    Ok(list.into_iter().collect())
}

fn parse_path_list(input: ParseStream) -> Result<Vec<Path>> {
    let list: Punctuated<Path, Token![,]> = Punctuated::<Path, Token![,]>::parse_terminated(input)?;
    Ok(list.into_iter().collect())
}

fn parse_bool(input: ParseStream) -> Result<bool> {
    let b: syn::LitBool = input.parse()?;
    Ok(b.value)
}
