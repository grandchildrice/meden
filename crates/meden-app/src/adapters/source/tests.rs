#[path = "oracle.rs"]
mod oracle;
use super::*;
use proptest::prelude::*;

fn original(source: &str, lean: bool) -> (usize, usize, Vec<String>, Vec<Symbol>) {
    let (count, theorems, imports) = oracle::headers(source, lean);
    (count, theorems, imports, oracle::symbols(source, lean))
}
fn current(source: &str, lean: bool) -> (usize, usize, Vec<String>, Vec<Symbol>) {
    let result = index(source, lean);
    (
        result.declarations,
        result.theorems,
        result.imports,
        result.symbols,
    )
}

#[test]
fn malformed_names_imports_modifiers_and_namespace_edges_keep_original_observations() {
    for source in [
        "def\ntheorem named : True := by trivial\nimport A B\n",
        "namespace Outer\nsection\nprivate def x: Nat := 0\nend\nlemma y : x = x := rfl\nend Outer\n",
        "pub(crate) async fn f() { other::g() }\n/* /- import Fake -/ */\ntrait T {}\n",
        "public import A B\npublic noncomputable theorem claim : True := True.intro\n",
        "def a := b\nimport A\nend\nnamespace 末端\nprivate axiom x : Prop\n",
        "def\nlemma\nconstant\nopaque\nstructure\n",
        "namespace A\n/- nested /- def hidden -/ -- still hidden\n-/\ndef visible : String := \"end A\\\"\"\nend A\n",
    ] {
        for lean in [false, true] {
            assert!(current(source, lean) == original(source, lean), "{source}");
        }
    }
    // Counts include keyword-only observations even without a named symbol.
    let incomplete = index("def\nlemma\nconstant\n", true);
    assert_eq!((incomplete.declarations, incomplete.theorems), (3, 1));
    assert!(incomplete.symbols.is_empty());
}

fn fragment() -> impl Strategy<Value = String> {
    prop_oneof![
        prop::collection::vec(any::<char>(), 0..80).prop_map(|c| c.into_iter().collect()),
        (
            prop::sample::select(vec![
                "",
                "public ",
                "private ",
                "pub(crate) async ",
                "noncomputable "
            ]),
            prop::sample::select(vec![
                "def",
                "lemma",
                "theorem",
                "axiom",
                "fn",
                "trait",
                "structure",
                "class",
                "inductive",
                "abbrev",
                "opaque",
                "constant",
                "struct",
                "enum",
                "type",
                "namespace",
                "section",
                "end",
                "import"
            ]),
            prop::sample::select(vec!["", "x", "N_DD_AA", "A.b", "path::node", "末端", "x:"]),
            prop::sample::select(vec![
                "",
                " : Nat := other",
                " : True := by exact helper",
                " A B",
                " -- theorem hidden",
                " /* nested /* x */ end */"
            ]),
        )
            .prop_map(|(prefix, command, name, suffix)| format!(
                "{prefix}{command} {name}{suffix}"
            )),
        prop::sample::select(vec![
            "/- outer\n/- inner -/\ndef hidden\n-/",
            "\"def hidden\\\"\"",
            "-- theorem hidden",
            "/* outer\n/* inner */ fn hidden */"
        ])
        .prop_map(str::to_owned),
    ]
}
proptest! {
    #[test]
    fn one_pass_preserves_the_independent_two_pass_result(
        fragments in prop::collection::vec(fragment(), 0..100), lean in any::<bool>(), crlf in any::<bool>()
    ) {
        let source = fragments.join(if crlf { "\r\n" } else { "\n" });
        prop_assert!(current(&source, lean) == original(&source, lean), "source: {source:?}; Lean: {lean}");
    }
}
#[test]
fn source_navigation_does_not_count_imports_or_theorems_in_comments_and_literals() {
    let source = "/- import Fake\n/- theorem fake : False -/\n-/\npublic import Real.Module\n-- theorem fake : False\ndef text := \"theorem fake : False\"\npublic theorem actual (n : Nat) : n = n := rfl\n";
    let observed = index(source, true);
    assert_eq!(
        (observed.declarations, observed.theorems, observed.imports),
        (2, 1, vec!["Real.Module".to_owned()])
    );
}
#[test]
fn cached_symbols_keep_original_lines_names_and_masked_reference_candidates() {
    let source = "namespace Demo\nprivate def keep : Nat := 1\n/- theorem fake : False := keep -/\npublic theorem actual : keep = keep := rfl\nend Demo\n";
    let found = index(source, true).symbols;
    assert_eq!(found.len(), 2);
    assert!(found
        .first()
        .is_some_and(|symbol| symbol.name == "Demo.keep" && symbol.private));
    assert!(found
        .get(1)
        .is_some_and(|symbol| symbol.name == "Demo.actual"
            && symbol.declaration == "theorem"
            && symbol.line == 4
            && symbol.references.contains(&"keep".into())
            && !symbol.references.contains(&"fake".into())));
}
#[test]
fn declaration_keywords_distinguish_lemmas_from_theorems_and_data() {
    let found = index("lemma helper : True := True.intro\ntheorem result : True := helper\nstructure Record where\n  field : Nat\n", true).symbols;
    assert_eq!(
        found
            .iter()
            .map(|item| item.declaration.as_str())
            .collect::<Vec<_>>(),
        vec!["lemma", "theorem", "structure"]
    );
}
