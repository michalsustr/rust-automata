mod dsl;
mod mermaid;

pub use dsl::attr as dsl_attr;
pub use mermaid::attr as mermaid_attr;

/// Convert a path to the rust‑doc HTML file path.
///
/// `states::Open`   → `"states/struct.Open.html"`  
/// `a::b::C`        → `"a/b/struct.C.html"`  
/// `Top`            → `"struct.Top.html"`
#[allow(dead_code)]
fn doc_link(p: &syn::Path) -> String {
    let mut segs = p
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>();
    let last = segs.pop().unwrap(); // safe: at least one segment
    if segs.is_empty() {
        format!("struct.{last}.html")
    } else {
        format!("{}/struct.{last}.html", segs.join("/"))
    }
}
