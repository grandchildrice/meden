export type Bounds = {
  left: number;
  top: number;
  right: number;
  bottom: number;
};
type Rectangle = { x: number; y: number; width: number; height: number };
const BLOCK = 32;

/** Bounds over contiguous original siblings, never a reordered layout. The
 * immutable index is disposable; its query emits the original item identities.
 */
export class OrderedBounds<T extends Rectangle> {
  #items: readonly T[];
  #bounds: Float64Array;
  #leaves: number;

  constructor(items: readonly T[]) {
    this.#items = items;
    let leaves = 1;
    while (leaves * BLOCK < items.length) leaves *= 2;
    this.#leaves = leaves;
    const boxes = (this.#bounds = new Float64Array(leaves * 8));
    for (let node = leaves; node < leaves * 2; node++) {
      const offset = node * 4;
      boxes[offset] = boxes[offset + 1] = Infinity;
      boxes[offset + 2] = boxes[offset + 3] = -Infinity;
      const first = (node - leaves) * BLOCK;
      for (let i = first; i < Math.min(first + BLOCK, items.length); i++) {
        const area = items[i]!;
        boxes[offset] = Math.min(boxes[offset]!, area.x);
        boxes[offset + 1] = Math.min(boxes[offset + 1]!, area.y);
        boxes[offset + 2] = Math.max(boxes[offset + 2]!, area.x + area.width);
        boxes[offset + 3] = Math.max(boxes[offset + 3]!, area.y + area.height);
      }
    }
    for (let node = leaves - 1; node > 0; node--) {
      const offset = node * 4,
        a = node * 8,
        b = a + 4;
      boxes[offset] = Math.min(boxes[a]!, boxes[b]!);
      boxes[offset + 1] = Math.min(boxes[a + 1]!, boxes[b + 1]!);
      boxes[offset + 2] = Math.max(boxes[a + 2]!, boxes[b + 2]!);
      boxes[offset + 3] = Math.max(boxes[a + 3]!, boxes[b + 3]!);
    }
  }

  /** Inclusive intersections in original order, including touching edges. */
  visit(view: Bounds, emit: (item: T) => void) {
    const boxes = this.#bounds,
      stack = [1];
    while (stack.length) {
      const node = stack.pop()!,
        offset = node * 4;
      if (
        boxes[offset]! > view.right ||
        boxes[offset + 1]! > view.bottom ||
        boxes[offset + 2]! < view.left ||
        boxes[offset + 3]! < view.top
      )
        continue;
      if (node < this.#leaves) stack.push(node * 2 + 1, node * 2);
      else {
        const first = (node - this.#leaves) * BLOCK;
        for (
          let i = first;
          i < Math.min(first + BLOCK, this.#items.length);
          i++
        ) {
          const area = this.#items[i]!;
          if (!(
            area.x > view.right ||
            area.y > view.bottom ||
            area.x + area.width < view.left ||
            area.y + area.height < view.top
          ))
            emit(area);
        }
      }
    }
  }
}
