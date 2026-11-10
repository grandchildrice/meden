import MedenSpec.Generated.Repair

/-! Exact canonical decimal examples at the production contract boundary.
Serde/UTF-8 decoding and native string formatting remain adapters. -/
namespace MedenSpec.DecimalRefinement
open Aeneas Aeneas.Std Result ControlFlow
open Aeneas.Std.WP
open MedenRepairImpl
set_option maxHeartbeats 4000000

abbrev maximum : Nat := 18446744073709551615

abbrev digits (bytes : List U8) : Prop :=
  bytes.all (fun byte => decide (48 ≤ byte.val ∧ byte.val ≤ 57)) = true

def value (acc : Nat) (bytes : List U8) : Nat :=
  bytes.foldl (fun n byte => n * 10 + (byte.val - 48)) acc

def read (acc : Nat) (bytes : List U8) : Option Nat :=
  if digits bytes ∧ value acc bytes ≤ maximum then some (value acc bytes) else none

abbrev shape (bytes : List U8) : Prop :=
  bytes ≠ [] ∧ (bytes.length = 1 ∨ bytes[0]!.val ≠ 48)

def canonical (bytes : List U8) : Prop := shape bytes ∧ digits bytes

def decode (bytes : List U8) : Option Nat :=
  if shape bytes then read 0 bytes else none

private theorem value_ge (acc : Nat) (bytes : List U8) : acc ≤ value acc bytes := by
  induction bytes generalizing acc with
  | nil => simp [value]
  | cons byte rest ih =>
    have h := ih (acc * 10 + (byte.val - 48))
    simpa [value] using (show acc ≤ value (acc * 10 + (byte.val - 48)) rest by omega)

private theorem read_nil (acc : Nat) (h : acc ≤ maximum) : read acc [] = some acc := by
  simp [read, digits, value, h]

private theorem read_cons (acc : Nat) (byte : U8) (rest : List U8) :
    read acc (byte :: rest) =
      if 48 ≤ byte.val ∧ byte.val ≤ 57 ∧ acc * 10 + (byte.val - 48) ≤ maximum then
        read (acc * 10 + (byte.val - 48)) rest else none := by
  have hge := value_ge (acc * 10 + (byte.val - 48)) rest
  by_cases hb : 48 ≤ byte.val ∧ byte.val ≤ 57
  · by_cases hn : acc * 10 + (byte.val - 48) ≤ maximum
    · simp [read, digits, value, hb.1, hb.2, hn]
    · have hlarge : ¬value (acc * 10 + (byte.val - 48)) rest ≤ maximum := by omega
      simp only [value] at hlarge
      simp [read, digits, value, hb.1, hb.2, hn, hlarge]
  · by_cases hl : 48 ≤ byte.val <;> by_cases hh : byte.val ≤ 57 <;>
      simp (disch := omega) [read, digits, value, *]

@[step] theorem loop_refines (bytes : Slice U8) (initial : U64) (index : Usize)
    (hi : index.val ≤ bytes.val.length) :
    verification.contract.decimal_words.parse_u64_loop bytes initial index ⦃ out =>
      out.map UScalar.val = read initial.val (bytes.val.drop index.val) ⦄ := by
  unfold verification.contract.decimal_words.parse_u64_loop
  apply Aeneas.Std.loop.spec_decr_nat
    (fun s : U64 × Usize => bytes.val.length - s.2.val)
    (fun s : U64 × Usize => s.2.val ≤ bytes.val.length ∧
      read s.1.val (bytes.val.drop s.2.val) = read initial.val (bytes.val.drop index.val))
    (fun out : Option U64 => out.map UScalar.val = read initial.val (bytes.val.drop index.val))
    (fun (v, i) => verification.contract.decimal_words.parse_u64_loop.body bytes v i)
    (initial, index)
  · rintro ⟨current, i⟩ ⟨hbound, heval⟩
    dsimp only at *
    unfold verification.contract.decimal_words.parse_u64_loop.body
    by_cases hp : i.val < bytes.val.length
    · have hg : i < Slice.len bytes := by scalar_tac
      simp only [if_pos hg]
      step as ⟨byte, hbyte⟩
      have hdrop : bytes.val.drop i.val = byte :: bytes.val.drop (i.val + 1) := by
        rw [List.drop_eq_getElem_cons hp]
        simpa [getElem!_pos bytes.val i.val hp] using congrArg (fun b => b :: bytes.val.drop (i.val + 1)) hbyte.symm
      rw [hdrop, read_cons] at heval
      repeat' (first | step | split)
      all_goals try simp only [Aeneas.Std.lift, Aeneas.Std.bind_ok]
      repeat' (first | step | split)
      all_goals simp_all [U64.rMax, U64.max_eq, maximum, UScalar.eq_equiv]
      all_goals try split_ifs at heval
      all_goals try simp_all
      all_goals omega
    · have heq : i.val = bytes.val.length := by omega
      have hg : ¬i < Slice.len bytes := by scalar_tac
      simp only [if_neg hg, spec_ok]
      simpa [heq, read_nil current.val (by scalar_tac)] using heval
  · exact ⟨hi, rfl⟩

private theorem native_total (bytes : Slice U8) :
    verification.contract.decimal_words.parse_u64 bytes ⦃ out =>
      out.map UScalar.val = decode bytes.val ⦄ := by
  cases hlist : bytes.val with
  | nil =>
    simp [verification.contract.decimal_words.parse_u64, core.slice.Slice.is_empty,
      Slice.length, hlist, decode, shape]
  | cons first rest =>
    unfold verification.contract.decimal_words.parse_u64
    simp [core.slice.Slice.is_empty, Slice.length, hlist]
    step as ⟨byte, hbyte⟩
    have heq : byte = first := by simpa [hlist] using hbyte
    subst byte
    simp only [heq]
    by_cases hz : first.val = 48
    · simp [hz, UScalar.eq_equiv]
      cases rest with
      | nil => simp [decode, shape, read, digits, value, hlist, hz]
      | cons second rest => simp [decode, shape, hlist, hz]
    · simp [hz]
      apply spec_mono (loop_refines bytes (0#u64) (0#usize) (by simp))
      intro out hout
      simpa [decode, shape, hlist, hz] using hout

/-- Total exact decoding of the actual production parser for every represented slice. -/
theorem parse_u64_refines (bytes : Slice U8) :
    (do let out ← verification.contract.decimal_words.parse_u64 bytes
        ok (out.map UScalar.val)) = ok (decode bytes.val) := by
  have h := native_total bytes
  cases he : verification.contract.decimal_words.parse_u64 bytes with
  | ret out => simpa [he] using congrArg ok (by simpa [he] using h)
  | vis effect continuation => simp [he] at h
  | div => simp [he] at h

/-- Accepted text is canonical ASCII decimal and denotes exactly the returned integer. -/
theorem accepted_value_exact (bytes : Slice U8) (word : U64)
    (h : verification.contract.decimal_words.parse_u64 bytes = ok (some word)) :
    canonical bytes.val ∧ value 0 bytes.val = word.val := by
  have hs := parse_u64_refines bytes
  simp only [h, bind_ok, ok.injEq] at hs
  unfold decode read at hs
  split_ifs at hs <;> simp_all [canonical]
  assumption

private theorem decode_none (bytes : Slice U8) (hd : decode bytes.val = none) :
    verification.contract.decimal_words.parse_u64 bytes = ok none := by
  have h := native_total bytes
  cases he : verification.contract.decimal_words.parse_u64 bytes with
  | ret out =>
    cases out with
    | none => rfl
    | some word => simp [he, hd] at h
  | vis effect continuation => simp [he] at h
  | div => simp [he] at h

/-- Empty strings, leading zeroes and every non-ASCII-decimal byte are refused. -/
theorem noncanonical_rejected (bytes : Slice U8) (h : ¬canonical bytes.val) :
    verification.contract.decimal_words.parse_u64 bytes = ok none := by
  apply decode_none bytes
  unfold decode read
  by_cases hs : shape bytes.val
  · have hd : ¬digits bytes.val := by intro hd; exact h ⟨hs, hd⟩
    simp [hd]
  · simp [hs]

/-- Overflow never wraps or truncates an accepted example value. -/
theorem overflow_rejected (bytes : Slice U8) (h : maximum < value 0 bytes.val) :
    verification.contract.decimal_words.parse_u64 bytes = ok none := by
  apply decode_none bytes
  simp [decode, read, show ¬value 0 bytes.val ≤ maximum by omega]

end MedenSpec.DecimalRefinement
