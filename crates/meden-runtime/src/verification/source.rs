//! The backend accepts a declared pure, finite Rust transition module. Validate this
//! untrusted syntax in a worker, before extraction/build: attributes, macros,
//! conditional compilation and compile-time environment access could otherwise
//! make the extracted and delivered programs differ.

use std::io;

use super::Contract;

use syn::visit::{self, Visit};
use syn::{
    BinOp, Expr, Fields, FnArg, GenericArgument, Item, Pat, PathArguments, ReturnType, Type,
    Visibility,
};

fn named(ty: &Type, name: &str) -> bool {
    matches!(ty, Type::Path(p) if p.qself.is_none() && p.path.is_ident(name))
}

/// Check the contract's declared API and its supported pure source subset.
/// Call only inside the offline worker when the source is untrusted. This is
/// a syntax boundary, not a correctness proof; Lean must still check behavior.
pub fn validate_source(source: &str, contract: &Contract) -> io::Result<()> {
    contract.validate()?;
    if source.len() > 65_536 {
        return Err(unsupported("kernel source exceeds 64 KiB"));
    }
    let file = syn::parse_file(source).map_err(|e| unsupported(&e.to_string()))?;
    if !file.attrs.is_empty() || file.items.len() != 2 {
        return Err(unsupported(
            "expected only the declared state struct and transition function; remove imports, other items and attributes",
        ));
    }
    let mut structure = None;
    let mut function = None;
    for item in &file.items {
        match item {
            Item::Struct(s) if s.ident == contract.state => structure = Some(s),
            Item::Fn(f) if f.sig.ident == contract.function => function = Some(f),
            _ => {
                return Err(unsupported(
                    "additional items are outside this backend's scope",
                ))
            }
        }
    }
    let s = structure.ok_or_else(|| unsupported("missing declared state"))?;
    let Fields::Named(fields) = &s.fields else {
        return Err(unsupported("state needs named fields"));
    };
    if !s.attrs.is_empty()
        || !matches!(s.vis, Visibility::Public(_))
        || !s.generics.params.is_empty()
        || s.generics.where_clause.is_some()
        || fields.named.len() != contract.fields.len()
        || contract.fields.iter().any(|name| {
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
            "state fields differ from the declared public u64 fields",
        ));
    }
    let f = function.ok_or_else(|| unsupported("missing declared transition"))?;
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
        || signature.inputs.len() != contract.parameters.len().saturating_add(1)
        || signature
            .inputs
            .iter()
            .zip(
                std::iter::once(contract.state.as_str())
                    .chain(contract.parameters.iter().map(|_| "u64")),
            )
            .any(|(arg, ty)| {
                !matches!(arg, FnArg::Typed(a) if a.attrs.is_empty()
                && named(&a.ty, ty)
                && matches!(a.pat.as_ref(), Pat::Ident(p) if p.attrs.is_empty()
                    && p.by_ref.is_none() && p.subpat.is_none()))
            })
        || !matches!(&signature.output, ReturnType::Type(_, ty) if named(ty, &contract.state))
    {
        return Err(unsupported(
            "transition signature differs from the accepted state/parameter API",
        ));
    }
    let mut checker = Pure {
        contract,
        valid: true,
        nodes: 0,
        reason: "use finite if/match/return expressions, local variables, fields, comparisons, u64 arithmetic (+, -, *, /, %) and supported Option operations; attributes, other calls, macros, loops and nested items are unsupported",
    };
    checker.visit_signature(signature);
    checker.visit_block(&f.block);
    if !checker.valid {
        return Err(unsupported(checker.reason));
    }
    Ok(())
}

fn unsupported(message: &str) -> io::Error {
    io::Error::other(format!("unsupported Rust transition source: {message}"))
}

struct Pure<'a> {
    contract: &'a Contract,
    valid: bool,
    nodes: usize,
    reason: &'static str,
}

impl<'ast> Visit<'ast> for Pure<'_> {
    fn visit_expr(&mut self, expression: &'ast Expr) {
        self.nodes = self.nodes.saturating_add(1);
        self.valid &= self.nodes <= 4096
            && match expression {
                Expr::Assign(_)
                | Expr::Block(_)
                | Expr::If(_)
                | Expr::Let(_)
                | Expr::Match(_)
                | Expr::Paren(_)
                | Expr::Return(_) => true,
                Expr::Binary(e) => matches!(
                    e.op,
                    BinOp::Add(_)
                        | BinOp::Sub(_)
                        | BinOp::Mul(_)
                        | BinOp::Div(_)
                        | BinOp::Rem(_)
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
                    matches!(&e.member, syn::Member::Named(n) if self.contract.fields.iter().any(|field| n == field))
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
                Expr::Struct(e) => e.path.is_ident(&self.contract.state),
                Expr::Unary(e) => matches!(e.op, syn::UnOp::Not(_)),
                Expr::MethodCall(e) => {
                    let arity = match e.method.to_string().as_str() {
                        "saturating_add" | "saturating_sub" | "checked_add" | "checked_sub"
                        | "unwrap_or" => Some(1),
                        "is_some" | "is_none" | "unwrap" => Some(0),
                        _ => None,
                    };
                    let supported = arity == Some(e.args.len()) && e.turbofish.is_none();
                    if !supported {
                        self.reason = "supported methods are u64 saturating_add/sub, checked_add/sub and Option is_some/is_none/unwrap/unwrap_or; other methods, wrong arity and generic method arguments are unsupported";
                    }
                    supported
                }
                Expr::Call(e) => {
                    let supported = e.args.len() == 1
                        && matches!(e.func.as_ref(), Expr::Path(p) if p.qself.is_none() && p.path.is_ident("Some"));
                    if !supported {
                        self.reason = "only Some(value) constructor calls are supported; free function calls are unsupported";
                    }
                    supported
                }
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
        self.valid &= [self.contract.state.as_str(), "u64", "bool"]
            .iter()
            .any(|name| named(ty, name))
            || matches!(ty, Type::Path(p) if p.qself.is_none() && p.path.segments.len() == 1
                && p.path.segments.first().is_some_and(|segment| segment.ident == "Option"
                    && matches!(&segment.arguments, PathArguments::AngleBracketed(a)
                        if a.args.len() == 1 && matches!(a.args.first(), Some(GenericArgument::Type(t)) if named(t, "u64")))));
    }
    fn visit_pat(&mut self, pattern: &'ast Pat) {
        self.valid &= match pattern {
            Pat::Ident(p) => p.ident != "Some",
            Pat::Type(_) | Pat::Wild(_) | Pat::Lit(_) | Pat::Paren(_) | Pat::Or(_) => true,
            Pat::TupleStruct(p) => {
                p.qself.is_none() && p.path.is_ident("Some") && p.elems.len() == 1
            }
            Pat::Path(p) => p.qself.is_none() && p.path.is_ident("None"),
            _ => false,
        };
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
    const SOURCE: &str = include_str!("../../../../verification/examples/ledger.rs");

    fn validate_ledger_source(source: &str) -> io::Result<()> {
        let contract = Contract::parse(include_str!(
            "../../../../verification/examples/ledger.contract.json"
        ))?;
        validate_source(source, &contract)
    }

    #[test]
    fn declared_api_is_generic_and_constructor_shadowing_cannot_add_recursion() -> io::Result<()> {
        let mut contract = Contract::parse(include_str!(
            "../../../../verification/examples/counter.contract.json"
        ))?;
        let source = include_str!("../../../../verification/examples/counter.rs");
        assert!(validate_source(source, &contract).is_ok());
        assert!(validate_source(
            &source.replace("pub value: u64", "pub other: u64"),
            &contract
        )
        .is_err());
        assert!(validate_source(&source.replace("fn increment", "fn other"), &contract).is_err());

        contract.parameters.clear();
        contract.properties.clear();
        contract.examples.clear();
        contract.specification = "s".into();
        let identity =
            "pub struct Counter { pub value: u64 }\npub fn increment(s: Counter) -> Counter { s }";
        assert!(validate_source(identity, &contract).is_ok());
        // Some(value) is the only permitted free call. A local binding must
        // not turn that spelling into a recursive call of the one-argument API.
        assert!(validate_source(
            &identity.replace("{ s }", "{ let Some = increment; Some(s) }"),
            &contract
        )
        .is_err());
        contract.function = "Some".into();
        assert!(contract.validate().is_err());
        Ok(())
    }

    #[test]
    fn candidate_cannot_change_its_meaning_between_extraction_and_delivery() {
        assert!(validate_ledger_source(SOURCE).is_ok());
        let saturating = SOURCE
            .replace(
                "state.debit -= amount",
                "state.debit = state.debit.saturating_sub(amount)",
            )
            .replace(
                "state.credit += amount",
                "state.credit = state.credit.saturating_add(amount)",
            );
        assert!(validate_ledger_source(&saturating).is_ok());
        let checked = SOURCE
            .replace(
                "state.debit -= amount;",
                "let Some(debit) = state.debit.checked_sub(amount) else { return state; }; state.debit = debit;",
            )
            .replace(
                "state.credit += amount;",
                "let credit: Option<u64> = state.credit.checked_add(amount); if credit.is_none() { return state; } state.credit = credit.unwrap();",
            );
        assert!(validate_ledger_source(&checked).is_ok());
        let constructor = SOURCE.replace(
            "state.credit += amount;",
            "let credit = Some(state.credit + amount); state.credit = credit.unwrap_or(0);",
        );
        assert!(validate_ledger_source(&constructor).is_ok());
        for value in [
            "env!(\"KEY\")",
            "std::process::exit(0)",
            "{ #[cfg(charon)] let x = 0; x }",
        ] {
            assert!(validate_ledger_source(
                &constructor.replace("Some(state.credit + amount)", &format!("Some({value})"))
            )
            .is_err());
        }
        for method in [
            "wrapping_add",
            "rotate_left",
            "unwrap_unchecked",
            "unwrap_or_else",
        ] {
            let candidate = SOURCE.replace(
                "state.credit += amount",
                &format!("state.credit = state.credit.{method}(amount)"),
            );
            assert!(validate_ledger_source(&candidate)
                .is_err_and(|error| error.to_string().contains("supported methods")));
        }
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
