//! The first backend accepts a pure, finite Rust ledger module. Validate this
//! untrusted syntax in a worker, before extraction/build: attributes, macros,
//! conditional compilation and compile-time environment access could otherwise
//! make the extracted and delivered programs differ.

use std::io;

use syn::visit::{self, Visit};
use syn::{BinOp, Expr, Fields, FnArg, Item, Pat, ReturnType, Type, Visibility};

fn named(ty: &Type, name: &str) -> bool {
    matches!(ty, Type::Path(p) if p.qself.is_none() && p.path.is_ident(name))
}

/// Check the declared first-backend API and its supported pure source subset.
/// Call only inside the offline worker when the source is untrusted. This is
/// a syntax boundary, not a correctness proof; Lean must still check behavior.
pub fn validate_ledger_source(source: &str) -> io::Result<()> {
    if source.len() > 65_536 {
        return Err(unsupported("ledger source exceeds 64 KiB"));
    }
    let file = syn::parse_file(source).map_err(|e| unsupported(&e.to_string()))?;
    if !file.attrs.is_empty() || file.items.len() != 2 {
        return Err(unsupported(
            "expected only LedgerState and transfer, without attributes",
        ));
    }
    let mut structure = None;
    let mut function = None;
    for item in &file.items {
        match item {
            Item::Struct(s) if s.ident == "LedgerState" => structure = Some(s),
            Item::Fn(f) if f.sig.ident == "transfer" => function = Some(f),
            _ => {
                return Err(unsupported(
                    "additional items are outside this backend's scope",
                ))
            }
        }
    }
    let s = structure.ok_or_else(|| unsupported("missing LedgerState"))?;
    let Fields::Named(fields) = &s.fields else {
        return Err(unsupported("LedgerState needs named fields"));
    };
    if !s.attrs.is_empty()
        || !matches!(s.vis, Visibility::Public(_))
        || !s.generics.params.is_empty()
        || s.generics.where_clause.is_some()
        || fields.named.len() != 3
        || ["debit", "credit", "sequence"].iter().any(|name| {
            fields
                .named
                .iter()
                .filter(|f| {
                    f.ident.as_ref().is_some_and(|id| id == name)
                        && f.attrs.is_empty()
                        && matches!(f.vis, Visibility::Public(_))
                        && named(&f.ty, "u64")
                })
                .count()
                != 1
        })
    {
        return Err(unsupported(
            "expected three public u64 fields: debit, credit, sequence",
        ));
    }
    let f = function.ok_or_else(|| unsupported("missing transfer"))?;
    let signature = &f.sig;
    if !f.attrs.is_empty()
        || !matches!(f.vis, Visibility::Public(_))
        || signature.constness.is_some()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
        || signature.inputs.len() != 4
        || signature
            .inputs
            .iter()
            .zip(["LedgerState", "u64", "u64", "u64"])
            .any(|(arg, ty)| {
                !matches!(arg, FnArg::Typed(a) if a.attrs.is_empty()
                && named(&a.ty, ty)
                && matches!(a.pat.as_ref(), Pat::Ident(p) if p.attrs.is_empty()
                    && p.by_ref.is_none() && p.subpat.is_none()))
            })
        || !matches!(&signature.output, ReturnType::Type(_, ty) if named(ty, "LedgerState"))
    {
        return Err(unsupported(
            "expected pub fn transfer(LedgerState, u64, u64, u64) -> LedgerState",
        ));
    }
    let mut checker = Pure {
        valid: true,
        nodes: 0,
    };
    checker.visit_signature(signature);
    checker.visit_block(&f.block);
    if !checker.valid {
        return Err(unsupported("use finite if/return expressions, local variables, fields, comparisons and u64 addition/subtraction; attributes, calls, macros, loops and nested items are unsupported"));
    }
    Ok(())
}

fn unsupported(message: &str) -> io::Error {
    io::Error::other(format!("unsupported Rust ledger source: {message}"))
}

struct Pure {
    valid: bool,
    nodes: usize,
}

impl<'ast> Visit<'ast> for Pure {
    fn visit_expr(&mut self, expression: &'ast Expr) {
        self.nodes = self.nodes.saturating_add(1);
        self.valid &= self.nodes <= 4096
            && match expression {
                Expr::Assign(_)
                | Expr::Block(_)
                | Expr::If(_)
                | Expr::Paren(_)
                | Expr::Return(_) => true,
                Expr::Binary(e) => matches!(
                    e.op,
                    BinOp::Add(_)
                        | BinOp::Sub(_)
                        | BinOp::AddAssign(_)
                        | BinOp::SubAssign(_)
                        | BinOp::Eq(_)
                        | BinOp::Ne(_)
                        | BinOp::Lt(_)
                        | BinOp::Le(_)
                        | BinOp::Gt(_)
                        | BinOp::Ge(_)
                        | BinOp::And(_)
                        | BinOp::Or(_)
                ),
                Expr::Field(e) => {
                    matches!(&e.member, syn::Member::Named(n) if n == "debit" || n == "credit" || n == "sequence")
                }
                Expr::Lit(e) => {
                    matches!(&e.lit, syn::Lit::Bool(_))
                        || matches!(&e.lit, syn::Lit::Int(i) if i.suffix().is_empty() || i.suffix() == "u64")
                }
                Expr::Path(e) => {
                    e.qself.is_none()
                        && (e.path.get_ident().is_some()
                            || e.path
                                .segments
                                .iter()
                                .map(|s| s.ident.to_string())
                                .collect::<Vec<_>>()
                                == ["u64", "MAX"])
                }
                Expr::Struct(e) => e.path.is_ident("LedgerState"),
                Expr::Unary(e) => matches!(e.op, syn::UnOp::Not(_)),
                _ => false,
            };
        if self.valid {
            visit::visit_expr(self, expression);
        }
    }
    fn visit_attribute(&mut self, _: &'ast syn::Attribute) {
        self.valid = false;
    }
    fn visit_item(&mut self, _: &'ast Item) {
        self.valid = false;
    }
    fn visit_macro(&mut self, _: &'ast syn::Macro) {
        self.valid = false;
    }
    fn visit_type(&mut self, ty: &'ast Type) {
        self.valid &= ["LedgerState", "u64", "bool"]
            .iter()
            .any(|name| named(ty, name));
    }
    fn visit_pat(&mut self, pattern: &'ast Pat) {
        self.valid &= matches!(pattern, Pat::Ident(_) | Pat::Type(_) | Pat::Wild(_));
        if self.valid {
            visit::visit_pat(self, pattern);
        }
    }
    fn visit_ident(&mut self, id: &'ast syn::Ident) {
        self.valid &= id
            .to_string()
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_');
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const SOURCE: &str = include_str!("../../../../verification/ledger/ledger.rs");

    #[test]
    fn candidate_cannot_change_its_meaning_between_extraction_and_delivery() {
        assert!(validate_ledger_source(SOURCE).is_ok());
        for extra in ["#[cfg(charon)]", "#[inline]", "#[path=\"other.rs\"]"] {
            assert!(validate_ledger_source(&format!("{extra}\n{SOURCE}")).is_err());
        }
        for expression in [
            "env!(\"RUSTC_WRAPPER\")",
            "include!(\"other.rs\")",
            "std::process::exit(0)",
            "loop {}",
            "{fn hidden() {} 0}",
        ] {
            assert!(validate_ledger_source(
                &SOURCE.replace("amount == 0", &format!("amount == {expression}"))
            )
            .is_err());
        }
        assert!(
            validate_ledger_source(&SOURCE.replace("pub credit: u64", "pub credit: u32")).is_err()
        );
    }
}
