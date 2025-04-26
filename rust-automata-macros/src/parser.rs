use quote::ToTokens;
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
/// (from_state[, input]) -> (to_state[, output]) [ : guard_expr ] [ = handler ]
/// ```
/// * `guard_expr` and `handler` are optional.
/// * `from_state`, `input`, `to_state`, `output` are all parsed as `Path`,
///   so module‐qualified identifiers work out of the box.
/// * `guard_expr` is parsed as a boolean expression (can use &&, ||, !, etc.)
/// * `handler` is parsed as an `Ident`.
pub struct Transition {
    pub from_state: Path,
    pub input: Option<Path>,
    pub to_state: Path,
    pub output: Option<Path>,
    // Guaranteed to be one of: syn::Expr::Path(_) | syn::Expr::Binary(_) | syn::Expr::Unary(_)
    // See also `try_match_guard`
    pub guard: Option<syn::Expr>,
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
        // and optional handler after '='
        // -------------------------
        let guard: Option<syn::Expr>;
        let handler: Option<Ident>;
        (guard, handler) = if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            let expr: syn::Expr = input.parse()?;
            match expr {
                // Having guards and handlers at the same time results in Assign expression.
                syn::Expr::Assign(assign) => (
                    Some(try_match_guard(*assign.left)?),
                    Some(try_match_handler(*assign.right)?),
                ),
                _ => (Some(try_match_guard(expr)?), None),
            }
        } else if input.peek(Token![=]) {
            // No guard, but we have a handler
            input.parse::<Token![=]>()?;
            (None, Some(input.parse::<Ident>()?))
        } else {
            // Neither guard nor handler
            (None, None)
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

// Only accept specific expression types for guard.
fn try_match_guard(expr: syn::Expr) -> Result<syn::Expr> {
    match expr {
        syn::Expr::Path(_) | syn::Expr::Binary(_) | syn::Expr::Unary(_) => Ok(expr),
        _ => Err(syn::Error::new_spanned(expr, "invalid guard expression")),
    }
}

fn try_match_handler(expr: syn::Expr) -> Result<syn::Ident> {
    match expr {
        syn::Expr::Path(expr_path) => Ok(expr_path.path.segments.last().unwrap().ident.clone()),
        _ => Err(syn::Error::new_spanned(expr, "invalid handler expression")),
    }
}

pub fn token_to_string<T: ToTokens>(token_source: &T) -> String {
    let mut ts = proc_macro2::TokenStream::new();
    token_source.to_tokens(&mut ts);
    ts.to_string()
}

pub fn guard_expr_to_string(expr: &syn::Expr, path_fn: &dyn Fn(&syn::Path) -> String) -> String {
    match expr {
        syn::Expr::Path(expr_path) => path_fn(&expr_path.path),
        syn::Expr::Binary(binary) => {
            let left = guard_expr_to_string(&binary.left, path_fn);
            let right = guard_expr_to_string(&binary.right, path_fn);
            let op_str = token_to_string(&binary.op);
            format!("{} {} {}", left, op_str, right)
        }
        syn::Expr::Unary(unary) => {
            let op_str = token_to_string(&unary.op);
            let expr = guard_expr_to_string(&unary.expr, path_fn);
            format!("{}{}", op_str, expr)
        }
        _ => panic!("Unsupported guard expression: {}", token_to_string(expr)),
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
                .map(|g| guard_expr_to_string(g, &|p| key(p)))
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
        assert!(t.guard.is_some());
        assert_eq!(t.handler.unwrap().to_string(), "handler_xyz");
        assert_eq!(
            guard_expr_to_string(&t.guard.unwrap(), &|p| key(p)),
            "guard_xyz"
        );
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

    #[test]
    fn parses_handler_only() {
        let src = "(A) -> (B) = handler_xyz";
        let t: Transition = syn::parse_str(src).unwrap();
        assert!(t.input.is_none());
        assert!(t.output.is_none());
        assert!(t.guard.is_none());
        assert_eq!(t.handler.unwrap().to_string(), "handler_xyz");
    }

    #[test]
    fn parses_complex_guard() {
        let src = r#"(S1) -> (S2) : a && b || !c"#;
        let t: Transition = syn::parse_str(src).unwrap();
        assert!(t.guard.is_some());
        assert_eq!(
            guard_expr_to_string(&t.guard.unwrap(), &|p| key(p)),
            "a && b || !c"
        );
    }

    #[test]
    fn parses_complex_guard_with_handler() {
        let src = r#"(S1) -> (S2) : a && b || !c = handler_xyz"#;
        let t: Transition = syn::parse_str(src).unwrap();
        assert!(t.guard.is_some());
        assert_eq!(t.handler.unwrap().to_string(), "handler_xyz");
        assert_eq!(
            guard_expr_to_string(&t.guard.unwrap(), &|p| key(p)),
            "a && b || !c"
        );
    }

    #[test]
    fn parses_invalid() {
        let src = r#"blabla"#;
        assert!(syn::parse_str::<Transition>(src).is_err());
        let src = r#"(S1,S2)"#;
        assert!(syn::parse_str::<Transition>(src).is_err());
        let src = r#"(S1) -> (S2) : a(some_invalid_expr)"#;
        assert!(syn::parse_str::<Transition>(src).is_err());
        let src = r#"(S1) -> (S2) = some_invalid_expr(handler_xyz)"#;
        assert!(syn::parse_str::<Transition>(src).is_err());
        let src = r#"(S1) -> (S2) : a(some_invalid_expr)"#;
        assert!(syn::parse_str::<Transition>(src).is_err());
        let src = r#"(S1) -> (S2) = some_invalid_expr(handler_xyz)"#;
        assert!(syn::parse_str::<Transition>(src).is_err());
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
