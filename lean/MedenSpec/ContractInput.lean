import MedenSpec.ContractSyntax

/-! Correspondence for the actual pure JSON-to-term admission entry points.
Lean's JSON accessors and parser are used with their original inputs. Their
own parsing/elaboration semantics and physical invocation remain dependencies. -/
namespace MedenSpec.ContractInput
open Lean MedenTerms ContractSyntax
set_option linter.unusedSimpArgs false

def builtinNames : List Name :=
  [`s, `Nat, `Bool, `True, `False, `true, `false, `Nat.min, `Nat.max, `min, `max]

def PermittedName (fields parameters : List String) (property : Bool) (name : Name) : Prop :=
  name ∈ builtinNames ∨
  (property = true ∧ (name = `out ∨ name = `transition)) ∨
  (∃ parameter ∈ parameters, name = Name.mkSimple parameter) ∨
  (∃ field ∈ fields, name = Name.str `s field) ∨
  (property = true ∧ ∃ field ∈ fields, name = Name.str `out field)

theorem names_exact (fields parameters : List String) (property : Bool) (name : Name) :
    name ∈ names fields parameters property ↔ PermittedName fields parameters property name := by
  cases property <;> simp [names, PermittedName, builtinNames, eq_comm, or_assoc]

private theorem bind_ok_iff {α β : Type} (r : Except String α)
    (f : α → Except String β) (value : β) :
    (r >>= f) = .ok value ↔ ∃ input, r = .ok input ∧ f input = .ok value := by
  cases r <;> simp [bind, Except.bind]

private theorem mapError_ok_iff {α : Type} (r : Except String α)
    (f : String → String) (value : α) :
    r.mapError f = .ok value ↔ r = .ok value := by
  cases r <;> simp [Except.mapError]

private theorem except_ok {α β : Type} (value : α) (f : α → Except String β) :
    (Except.ok value >>= f) = f value := rfl

private theorem except_error {α β : Type} (message : String) (f : α → Except String β) :
    (Except.error message >>= f) = .error message := rfl

private theorem except_pure {α : Type} (value : α) :
    (pure value : Except String α) = .ok value := rfl

private theorem except_throw {α : Type} (message : String) :
    (throw message : Except String α) = .error message := rfl

private theorem exists_unit (predicate : Unit → Prop) :
    (∃ result, predicate result) ↔ predicate () := by
  constructor
  · rintro ⟨result, h⟩; cases result; exact h
  · intro h; exact ⟨(), h⟩

def TermChecked (env : Environment) (allowed fields : List Name) (label text : String) : Prop :=
  text.utf8ByteSize ≤ 8192 ∧ ∃ stx,
    Parser.runParserCategory env `term text label = .ok stx ∧
    checkSyntax allowed fields 128 stx = .ok ()

theorem term_exact (env : Environment) (allowed fields : List Name) (label text : String) :
    checkTerm env allowed fields label text = .ok () ↔
      TermChecked env allowed fields label text := by
  by_cases size : text.utf8ByteSize > 8192
  · have large : ¬text.utf8ByteSize ≤ 8192 := by omega
    simp [checkTerm, TermChecked, size, large, except_throw, except_error]
  · have bounded : text.utf8ByteSize ≤ 8192 := by omega
    simp [checkTerm, TermChecked, size, bounded, except_pure, except_ok,
      mapError_ok_iff, bind_ok_iff]

def TermAdmissible (env : Environment) (allowed fields : List Name) (label text : String) : Prop :=
    text.utf8ByteSize ≤ 8192 ∧ ∃ stx,
      Parser.runParserCategory env `term text label = .ok stx ∧
      Admissible allowed fields stx

theorem term_admissible (env : Environment) (allowed fields : List Name) (label text : String)
    (h : checkTerm env allowed fields label text = .ok ()) :
    TermAdmissible env allowed fields label text := by
  obtain ⟨bound, stx, parsed, checked⟩ := (term_exact _ _ _ _ _).mp h
  exact ⟨bound, stx, parsed, accepted_admissible _ _ _ _ checked⟩

inductive StringValues : List Json → List String → Prop where
  | nil : StringValues [] []
  | cons (text : String) {values strings} : StringValues values strings →
      StringValues (.str text :: values) (text :: strings)

private theorem getStr_exact (json : Json) (text : String) :
    json.getStr? = .ok text ↔ json = .str text := by
  cases json <;> simp [Json.getStr?, except_pure]

private theorem map_strings (values : List Json) (strings : List String) :
    values.mapM Json.getStr? = .ok strings ↔ StringValues values strings := by
  induction values generalizing strings with
  | nil =>
    simp only [List.mapM_nil, except_pure, Except.ok.injEq]
    constructor
    · intro eq; subst strings; exact .nil
    · intro h; cases h; rfl
  | cons value values ih =>
    simp only [List.mapM_cons, bind_ok_iff, except_pure, Except.ok.injEq,
      getStr_exact, ih]
    constructor
    · rintro ⟨text, rfl, rest, checked, rfl⟩
      exact .cons text checked
    · intro h
      cases h with
      | cons text checked => exact ⟨text, rfl, _, checked, rfl⟩

def StringField (json : Json) (key : String) (strings : List String) : Prop :=
  ∃ value values, json.getObjVal? key = .ok value ∧ value.getArr? = .ok values ∧
    StringValues values.toList strings

theorem stringList_exact (json : Json) (key : String) (strings : List String) :
    stringList json key = .ok strings ↔ StringField json key strings := by
  simp only [stringList, bind_ok_iff, map_strings, StringField]
  constructor
  · rintro ⟨value, object, values, array, checked⟩
    exact ⟨value, values, object, array, checked⟩
  · rintro ⟨value, values, object, array, checked⟩
    exact ⟨value, object, values, array, checked⟩

private theorem fold_ok_iff {α : Type} (f : α → Except String Unit) (items : List α) :
    items.foldlM (fun _ item => f item) () = .ok () ↔
      ∀ item ∈ items, f item = .ok () := by
  induction items with
  | nil => simp [List.foldlM_nil, except_pure]
  | cons item rest ih =>
    simp only [List.foldlM_cons, bind_ok_iff]
    constructor
    · rintro ⟨result, checked, tail⟩
      cases result
      exact fun next mem => by
        rcases List.mem_cons.mp mem with rfl | mem
        · exact checked
        · exact ih.mp tail next mem
    · intro checked
      exact ⟨(), checked _ (by simp), ih.mpr fun next mem => checked next (by simp [mem])⟩

private theorem loop_ok_iff {α : Type} (f : α → Except String Unit) (items : Array α) :
    (forIn items () fun item _ => do f item; pure (.yield ())) = .ok () ↔
      ∀ item ∈ items.toList, f item = .ok () := by
  have unit_map (r : Except String Unit) : (fun _ => ()) <$> r = r := by
    cases r with
    | error => rfl
    | ok result => cases result; rfl
  have loop : (forIn items () fun item _ => do f item; pure (.yield ())) =
      items.foldlM (fun _ item => f item) () := by
    change (forIn items () fun item _ => (fun _ : Unit => .yield ()) <$> f item) = _
    simpa only [unit_map] using
      (Array.forIn_yield_eq_foldlM (fun item _ => f item) (fun _ _ _ => ()) ())
  rw [loop, ← Array.foldlM_toList, fold_ok_iff]

def PropertyChecked (env : Environment) (fields parameters : List String) (property : Json) : Prop :=
  ∃ name text, property.getObjValAs? String "name" = .ok name ∧
    property.getObjValAs? String "statement" = .ok text ∧
    TermChecked env (names fields parameters true) (fields.map Name.mkSimple)
      s!"properties.{name}.statement" text

theorem property_exact (env : Environment) (fields parameters : List String) (property : Json) :
    checkProperty env fields parameters property = .ok () ↔
      PropertyChecked env fields parameters property := by
  simp only [checkProperty, bind_ok_iff, term_exact, PropertyChecked]
  constructor
  · rintro ⟨name, hn, text, ht, checked⟩; exact ⟨name, text, hn, ht, checked⟩
  · rintro ⟨name, text, hn, ht, checked⟩; exact ⟨name, hn, text, ht, checked⟩

def ContractChecked (env : Environment) (json : Json) : Prop :=
  ∃ fields parameters specification propertiesValue properties,
    StringField json "fields" fields ∧ StringField json "parameters" parameters ∧
    json.getObjValAs? String "specification" = .ok specification ∧
    json.getObjVal? "properties" = .ok propertiesValue ∧
    propertiesValue.getArr? = .ok properties ∧
    TermChecked env (names fields parameters false) (fields.map Name.mkSimple)
      "specification" specification ∧
    ∀ property ∈ properties.toList, PropertyChecked env fields parameters property

private def ContractSteps (env : Environment) (json : Json) : Prop :=
  ∃ fields, StringField json "fields" fields ∧
    ∃ parameters, StringField json "parameters" parameters ∧
      ∃ specification, json.getObjValAs? String "specification" = .ok specification ∧
        TermChecked env (names fields parameters false) (fields.map Name.mkSimple)
          "specification" specification ∧
        ∃ propertiesValue, json.getObjVal? "properties" = .ok propertiesValue ∧
          ∃ properties, propertiesValue.getArr? = .ok properties ∧
            (forIn properties () fun property _ => do
              checkProperty env fields parameters property
              pure (.yield ())) = (Except.ok () : Except String Unit)

private theorem entry_steps (env : Environment) (json : Json) :
    checkContract env json = .ok () ↔ ContractSteps env json := by
  simp only [checkContract, bind_ok_iff, exists_unit, stringList_exact, term_exact,
    ContractSteps, except_pure, and_true]

theorem contract_exact (env : Environment) (json : Json) :
    checkContract env json = .ok () ↔ ContractChecked env json := by
  rw [entry_steps]
  unfold ContractSteps ContractChecked
  constructor
  · rintro ⟨fields, hf, parameters, hp, spec, hs, checked, pv, hv, properties, ha, all⟩
    have every := (loop_ok_iff (checkProperty env fields parameters) properties).mp
      (by simpa only [except_pure] using all)
    exact ⟨fields, parameters, spec, pv, properties, hf, hp, hs, hv, ha, checked,
      fun property mem => (property_exact _ _ _ _).mp (every property mem)⟩
  · rintro ⟨fields, parameters, spec, pv, properties, hf, hp, hs, hv, ha, checked, all⟩
    have every := (loop_ok_iff (checkProperty env fields parameters) properties).mpr
      (fun property mem => (property_exact _ _ _ _).mpr (all property mem))
    exact ⟨fields, hf, parameters, hp, spec, hs, checked, pv, hv, properties, ha,
      by simpa only [except_pure] using every⟩

def ContractAdmissible (env : Environment) (json : Json) : Prop :=
  ∃ fields parameters specification propertiesValue properties,
    StringField json "fields" fields ∧ StringField json "parameters" parameters ∧
    json.getObjValAs? String "specification" = .ok specification ∧
    json.getObjVal? "properties" = .ok propertiesValue ∧
    propertiesValue.getArr? = .ok properties ∧
    TermAdmissible env (names fields parameters false) (fields.map Name.mkSimple)
      "specification" specification ∧
    ∀ property ∈ properties.toList, ∃ name text,
      property.getObjValAs? String "name" = .ok name ∧
      property.getObjValAs? String "statement" = .ok text ∧
      TermAdmissible env (names fields parameters true) (fields.map Name.mkSimple)
        s!"properties.{name}.statement" text

theorem contract_admissible (env : Environment) (json : Json)
    (h : checkContract env json = .ok ()) : ContractAdmissible env json := by
  obtain ⟨fields, parameters, spec, pv, properties, hf, hp, hs, hv, ha, checked, all⟩ :=
    (contract_exact _ _).mp h
  refine ⟨fields, parameters, spec, pv, properties, hf, hp, hs, hv, ha,
    term_admissible _ _ _ _ _ ((term_exact _ _ _ _ _).mpr checked), ?_⟩
  intro property mem
  obtain ⟨name, text, hn, ht, checked⟩ := all property mem
  exact ⟨name, text, hn, ht, term_admissible _ _ _ _ _ ((term_exact _ _ _ _ _).mpr checked)⟩

end MedenSpec.ContractInput
