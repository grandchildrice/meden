export type Position = {
  x: number;
  y: number;
  r: number;
  th: number;
  level: number;
  slot: number;
  w: number;
};
export function longestPathLevels(
  root: string,
  adj: Record<string, string[]>,
  hop: Record<string, number>,
): { level: Record<string, number>; layers: string[][] };
export function layoutCone(spec: {
  layers: string[][];
  adj: Record<string, string[]>;
  th0: number;
  th1: number;
  groupOf?: (name: string) => string;
  pin?: Set<string>;
  r0?: number;
  dr?: number;
  slotMax?: number;
  slotMin?: number;
  slotPref?: number;
  rSoft?: number;
  maxPerLayer?: number;
}): {
  pos: Record<string, Position>;
  rings: { r: number }[];
  overflow: Record<string, number>;
  rMax: number;
};
