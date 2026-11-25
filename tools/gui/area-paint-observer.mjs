// Serialized into an opt-in diagnostic browser. Never imported by the app.
export function installAreaPaintObserver() {
  const report = (window.__medenAreaPaints = { frames: [], truncated: false });
  const owners = new WeakMap(),
    current = new WeakMap(),
    previous = new WeakMap();
  let fonts = 0,
    queued = 0;
  void document.fonts.ready.then(() => fonts++);
  document.fonts.addEventListener("loadingdone", () => fonts++);
  const transfer = HTMLCanvasElement.prototype.transferControlToOffscreen;
  HTMLCanvasElement.prototype.transferControlToOffscreen = function () {
    const surface = transfer.call(this);
    owners.set(surface, this);
    return surface;
  };
  const prototype = OffscreenCanvasRenderingContext2D.prototype;
  for (const name of ["clearRect", "roundRect", "fill", "stroke", "fillText"]) {
    const original = prototype[name];
    prototype[name] = function (...args) {
      const result = original.apply(this, args);
      if (
        report.truncated ||
        !owners.get(this.canvas)?.matches(".map-area-canvas")
      )
        return result;
      if (name === "clearRect") {
        if (report.frames.length + queued >= 10000) {
          report.truncated = true;
          current.delete(this);
          return result;
        }
        queued++;
        const m = this.getTransform();
        const frame = {
          phase: window.__medenProfile?.phase ?? "setup",
          fonts,
          size: [this.canvas.width, this.canvas.height],
          clear: args,
          transform: [m.a, m.b, m.c, m.d, m.e, m.f],
          commands: [],
        };
        current.set(this, frame);
        queueMicrotask(() => {
          queued--;
          if (report.frames.length >= 10000) {
            report.truncated = true;
            return;
          }
          const key = JSON.stringify([
            frame.fonts,
            frame.size,
            frame.clear,
            frame.transform,
            frame.commands,
          ]);
          report.frames.push({
            phase: frame.phase,
            areas: frame.commands.length,
            identical_to_previous: previous.get(this) === key,
            font_epoch: frame.fonts,
          });
          previous.set(this, key);
          if (current.get(this) === frame) current.delete(this);
        });
      } else {
        const frame = current.get(this);
        if (!frame) return result;
        if (name === "roundRect") frame.commands.push([args]);
        const last = frame.commands.at(-1);
        if (!last) return result;
        if (name === "fill")
          last.push(["fill", this.fillStyle, this.globalAlpha]);
        if (name === "stroke")
          last.push([
            "stroke",
            this.strokeStyle,
            this.globalAlpha,
            this.lineWidth,
          ]);
        if (name === "fillText")
          last.push([
            "text",
            args,
            this.font,
            this.fillStyle,
            this.globalAlpha,
          ]);
      }
      return result;
    };
  }
}
