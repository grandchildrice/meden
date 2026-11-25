// Browser-side conformance observations shared by the native map journey.
export function installMapPaintObserver() {
  // Observe actual drawing calls without a production instrumentation API.
  const frames = new WeakMap<HTMLCanvasElement | OffscreenCanvas, any[]>();
  (window as any).__areaPaints = frames;
  const surfaces = new WeakMap<HTMLCanvasElement, OffscreenCanvas>(),
    owners = new WeakMap<OffscreenCanvas, HTMLCanvasElement>();
  (window as any).__areaSurfaces = surfaces;
  const control = HTMLCanvasElement.prototype.transferControlToOffscreen;
  HTMLCanvasElement.prototype.transferControlToOffscreen = function () {
    const surface = control.call(this);
    surfaces.set(this, surface);
    owners.set(surface, this);
    return surface;
  };
  const activity = {
    font_settings: 0,
    text_draws: 0,
    image_copies: 0,
  };
  (window as any).__areaPaintActivity = activity;
  const font = Object.getOwnPropertyDescriptor(
    CanvasRenderingContext2D.prototype,
    "font",
  )!;
  Object.defineProperty(CanvasRenderingContext2D.prototype, "font", {
    ...font,
    set(value: string) {
      if (this.canvas.matches(".map-area-canvas")) activity.font_settings++;
      font.set!.call(this, value);
    },
  });
  for (const prototype of [
    CanvasRenderingContext2D.prototype,
    OffscreenCanvasRenderingContext2D.prototype,
  ]) {
    for (const name of [
      "clearRect",
      "roundRect",
      "fill",
      "stroke",
      "fillText",
      "drawImage",
    ] as const) {
      const original = prototype[name];
      (prototype as any)[name] = function (...args: any[]) {
        const result = (original as Function).apply(this, args);
        if (
          this.canvas instanceof HTMLCanvasElement &&
          !this.canvas.matches(".map-area-canvas")
        )
          return result;
        if (this.canvas instanceof HTMLCanvasElement) {
          if (name === "fillText") activity.text_draws++;
          if (name === "drawImage") activity.image_copies++;
        }
        if (name === "clearRect") {
          const frame: any[] = [];
          frames.set(this.canvas, frame);
          if (this.canvas instanceof OffscreenCanvas) {
            const owner = owners.get(this.canvas);
            if (owner) frames.set(owner, frame);
          }
        }
        if (name === "drawImage" && frames.has(args[0]))
          frames.set(this.canvas, frames.get(args[0])!);
        const frame = frames.get(this.canvas);
        if (!frame) return result;
        if (name === "roundRect") frame.push({ rect: args });
        const last = frame.at(-1);
        if (!last) return result;
        if (name === "fill")
          last.fill = { color: this.fillStyle, alpha: this.globalAlpha };
        if (name === "stroke")
          last.stroke = {
            color: this.strokeStyle,
            alpha: this.globalAlpha,
            width: this.lineWidth,
          };
        if (name === "fillText")
          last.label = { args, font: this.font, alpha: this.globalAlpha };
        return result;
      };
    }
  }
}

export function measureMapGeometry(element: Element) {
  const svg = element.querySelector<SVGSVGElement>(".continuous-map-scene")!,
    root = svg.getScreenCTM()!,
    unit = Math.hypot(root.a, root.b);
  let maximum = 0;
  const close = (actual: number, expected: number, label: string) => {
    const error = Math.abs(actual - expected);
    if (!Number.isFinite(error) || error > 0.003)
      throw Error(`${label}: ${actual} instead of ${expected}`);
    maximum = Math.max(maximum, error);
  };
  const point = (matrix: DOMMatrix | null, x: number, y: number) =>
    new DOMPoint(x, y).matrixTransform(matrix!);
  const boxes = [
    ...svg.querySelectorAll<SVGRectElement>(
      "[data-map-path] > rect, rect[data-map-path]",
    ),
  ];
  const canvas = element.querySelector<HTMLCanvasElement>(".map-area-canvas");
  if (canvas) {
    const frame: any[] = (window as any).__areaPaints.get(canvas);
    const origin = canvas.getBoundingClientRect();
    const surface = (window as any).__areaSurfaces.get(canvas) || canvas;
    close(
      surface.width,
      Math.round(origin.width * devicePixelRatio),
      "canvas backing width",
    );
    close(
      surface.height,
      Math.round(origin.height * devicePixelRatio),
      "canvas backing height",
    );
    let matched = 0;
    for (const paint of frame) {
      const box = boxes[matched];
      const p = box && point(root, box.x.baseVal.value, box.y.baseVal.value);
      if (
        !box ||
        !p ||
        Math.abs(paint.rect[0] - (p.x - origin.x)) > 0.003 ||
        Math.abs(paint.rect[1] - (p.y - origin.y)) > 0.003
      ) {
        if (paint.fill.alpha >= 0.5 || paint.label)
          throw Error("Interactive painted area has no ordered hit target");
        close(paint.stroke.width, 0.6, "inert painted stroke");
        close(
          paint.stroke.alpha,
          paint.fill.alpha * 0.75,
          "inert painted opacity",
        );
        continue;
      }
      matched++;
      const width = box.width.baseVal.value * unit,
        height = box.height.baseVal.value * unit;
      close(paint.rect[0], p.x - origin.x, "painted area x");
      close(paint.rect[1], p.y - origin.y, "painted area y");
      close(paint.rect[2], width, "painted area width");
      close(paint.rect[3], height, "painted area height");
      close(
        paint.rect[4],
        Math.min(3, width / 2, height / 2),
        "painted radius",
      );
      close(paint.stroke.width, 0.6, "painted stroke");
      close(paint.stroke.alpha, paint.fill.alpha * 0.75, "painted opacity");
      const shouldLabel = paint.fill.alpha > 0.65 && width > 65 && height > 28;
      if (Boolean(paint.label) !== shouldLabel)
        throw Error("Area label visibility changed");
      if (paint.label) {
        if (
          paint.label.args[0] !==
          box
            .closest("[data-map-path]")!
            .getAttribute("data-map-path")!
            .split("/")
            .at(-1)
        )
          throw Error("Wrong painted heading");
        close(paint.label.args[1] - paint.rect[0], 10, "painted label x");
        close(
          paint.label.args[2] - paint.rect[1],
          Math.min(height / 2, 25),
          "painted label y",
        );
        close(parseFloat(paint.label.font), 14, "painted label font");
        close(paint.label.alpha, 1, "painted label opacity");
      }
    }
    if (matched !== boxes.length)
      throw Error("Hit target has no original ordered painted area");
    const reader = new OffscreenCanvas(
      surface.width,
      surface.height,
    ).getContext("2d")!;
    reader.drawImage(canvas, 0, 0);
    const pixels = reader.getImageData(
      0,
      0,
      surface.width,
      surface.height,
    ).data;
    if (!pixels.some((value, i) => i % 4 === 3 && value > 0))
      throw Error("Area canvas is blank");
    if (
      getComputedStyle(canvas).display === "none" ||
      getComputedStyle(canvas).visibility === "hidden"
    )
      throw Error("Area canvas is hidden");
  }
  for (const box of canvas ? [] : boxes) {
    const style = getComputedStyle(box);
    close(parseFloat(style.strokeWidth) * unit, 0.6, "area stroke");
    close(parseFloat(style.rx) * unit, 3, "area radius");
    close(
      parseFloat(style.strokeOpacity),
      parseFloat(style.fillOpacity) * 0.75,
      "area opacity",
    );
    const label = box.parentElement!.querySelector<SVGTextElement>("text");
    if (label) {
      const p = point(
          label.getScreenCTM(),
          label.x.baseVal[0]!.value,
          label.y.baseVal[0]!.value,
        ),
        origin = point(root, box.x.baseVal.value, box.y.baseVal.value);
      close(p.x - origin.x, 10, "area label x");
      close(
        p.y - origin.y,
        Math.min(box.height.baseVal.value * unit * 0.5, 25),
        "area label y",
      );
      close(
        parseFloat(getComputedStyle(label).fontSize) * unit,
        14,
        "area font",
      );
      close(
        parseFloat(getComputedStyle(label).fillOpacity),
        1,
        "label opacity",
      );
    }
  }
  const nodes = [...svg.querySelectorAll<SVGGElement>("[data-symbol-id]")],
    centers: DOMPoint[] = [];
  for (const node of nodes) {
    const marker = node.querySelector<SVGGElement>(":scope > g")!,
      m = marker.getScreenCTM()!;
    close(Math.hypot(m.a, m.b), 1, "marker scale");
    centers.push(point(m, 0, 0));
    const text = node.querySelector<SVGTextElement>(":scope > text")!,
      p = point(
        text.getScreenCTM(),
        text.x.baseVal[0]!.value,
        text.y.baseVal[0]!.value,
      );
    close(p.x - m.e, 9, "symbol label x");
    close(p.y - m.f, 4, "symbol label y");
    close(
      parseFloat(getComputedStyle(text).fontSize) * unit,
      11,
      "symbol font",
    );
  }
  const lines = [
    ...svg.querySelectorAll<SVGLineElement>(
      '[data-map-layer="references"] line',
    ),
  ];
  for (const line of lines) {
    close(
      parseFloat(getComputedStyle(line).strokeWidth) * unit,
      1,
      "reference stroke",
    );
    for (const p of [
      point(root, line.x1.baseVal.value, line.y1.baseVal.value),
      point(root, line.x2.baseVal.value, line.y2.baseVal.value),
    ]) {
      const distance = Math.min(
        ...centers.map((q) => Math.hypot(q.x - p.x, q.y - p.y)),
      );
      close(distance, 0, "reference endpoint");
    }
  }
  const pins = [...svg.querySelectorAll<SVGGElement>(".map-main-pin")];
  for (const pin of pins) {
    const m = pin.getScreenCTM()!;
    close(Math.hypot(m.a, m.b), 1, "main pin scale");
  }
  return {
    zoom: unit,
    areas: boxes.length,
    declarations: nodes.length,
    references: lines.length,
    pins: pins.length,
    maximum_pixel_error: maximum,
  };
}

export function expectMapKeyboardFocus(element: Element) {
  const canvas = element.querySelector<HTMLCanvasElement>(".map-area-canvas");
  if (!canvas) return;
  const groups = [...element.querySelectorAll("[data-map-path]")];
  const active = document.activeElement!;
  const index = groups.indexOf(active);
  if (index < 0 || !active.matches(":focus-visible"))
    throw Error("Missing map keyboard focus");
  const paints = (window as any).__areaPaints.get(canvas);
  const origin = canvas.getBoundingClientRect(),
    box = active.getBoundingClientRect();
  const paint = paints.find(
    (p: any) =>
      Math.abs(p.rect[0] - (box.x - origin.x)) < 0.003 &&
      Math.abs(p.rect[1] - (box.y - origin.y)) < 0.003,
  );
  if (
    active.getAttribute("aria-hidden") !== "false" ||
    paint?.stroke.color !== "#ffe59c"
  )
    throw Error("Keyboard focus not painted");
}

export function observeRetainedAreaWrites(svg: SVGSVGElement) {
  const retained = new Set(
    svg.querySelectorAll(
      '[data-map-layer="areas"] rect, [data-map-layer="areas"] text',
    ),
  );
  const changes: Record<string, number> = {};
  const observe = (records: MutationRecord[]) => {
    for (const r of records)
      if (retained.has(r.target as Element)) {
        const name = r.attributeName!;
        changes[name] = (changes[name] || 0) + 1;
      }
  };
  const observer = new MutationObserver(observe);
  observer.observe(svg, {
    attributes: true,
    subtree: true,
    attributeFilter: [
      "x",
      "y",
      "width",
      "height",
      "rx",
      "stroke-width",
      "font-size",
      "transform",
      "style",
    ],
  });
  (window as any).__areaWrites = () => {
    observe(observer.takeRecords());
    observer.disconnect();
    return changes;
  };
}

// Count JavaScript attribute writes on actual targets, including creation.
// Query the actual target elements afterward so initial x/y writes are counted
// even when React has not assigned their data-map-path attribute yet.
export function observeTargetCreation() {
  const writes = new WeakMap<Element, number>();
  (window as any).__targetAttributeWrites = writes;
  const original = Element.prototype.setAttribute;
  Element.prototype.setAttribute = function (name, value) {
    if (this instanceof SVGRectElement)
      writes.set(this, (writes.get(this) || 0) + 1);
    original.call(this, name, value);
  };
}
