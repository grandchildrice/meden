import { expect, test } from "vitest";
import { highlightSource } from "./highlight";
test("Lean highlighting preserves source and keeps keywords inside comments as comment text", async () => {
  const source =
    '-- theorem for def\ndef text := "-- literal"\n/- nested /- theorem -/ comment -/\ntheorem result : True := by trivial';
  const rows = await highlightSource(source, "Proof.lean");
  expect(
    rows.map((row) => row.map((token) => token.content).join("")).join("\n"),
  ).toBe(source);
  expect(new Set(rows[0]!.map((token) => token.color)).size).toBe(1);
  expect(rows[0]![0]!.color).not.toBe(
    rows[3]!.find((token) => token.content === "theorem")!.color,
  );
  expect(
    rows[1]!.find((token) => token.content.includes("-- literal"))!.color,
  ).not.toBe(rows[0]![0]!.color);
});
