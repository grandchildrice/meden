import { memo, type KeyboardEvent } from "react";
import { type MapArea, type Symbol } from "./project-map-model";

export const mapColors = [
  "#275f69",
  "#414b7e",
  "#73516c",
  "#695a38",
  "#365b51",
];
const keyboard = (event: KeyboardEvent, action: () => void) => {
  if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    action();
  }
};

export const mapAreaColor = (a: MapArea) =>
  a.depth === 1
    ? mapColors[a.color]!
    : `hsl(${(a.color * 67 + a.depth * 31 + (a.path.split("").reduce((n, c) => n + c.charCodeAt(0), 0) % 47)) % 360} 32% 28%)`;

function SymbolMarker({
  symbol,
  selected,
  zoom,
}: {
  symbol: Symbol;
  selected: boolean;
  zoom: number;
}) {
  const r = selected ? 8 : 6,
    shape = symbol.role.shape;
  const style = {
    fill: symbol.role.color,
    stroke: selected ? "#fff2ad" : "#263a46",
    strokeWidth: selected ? 2 : 1,
  };
  return (
    <g
      className="map-symbol-marker"
      transform={`translate(${symbol.x}, ${symbol.y}) scale(${1 / zoom})`}
    >
      {shape === "circle" ? (
        <circle r={r} {...style} />
      ) : shape === "square" ? (
        <rect x={-r} y={-r} width={r * 2} height={r * 2} rx={1} {...style} />
      ) : (
        <path
          d={
            shape === "diamond"
              ? `M 0 ${-r - 2} L ${r + 1} 0 L 0 ${r + 2} L ${-r - 1} 0 Z`
              : shape === "triangle"
                ? `M 0 ${-r - 2} L ${r + 2} ${r} L ${-r - 2} ${r} Z`
                : Array.from({ length: 10 }, (_, i) => {
                    const angle = (i * Math.PI) / 5 - Math.PI / 2,
                      radius = i % 2 ? r * 0.45 : r + 2;
                    return `${i ? "L" : "M"} ${Math.cos(angle) * radius} ${Math.sin(angle) * radius}`;
                  }).join(" ") + " Z"
          }
          {...style}
        />
      )}
    </g>
  );
}

export const MapDeclaration = memo(function MapDeclaration({
  symbol,
  opacity: alpha,
  label,
  selected,
  select,
  zoom,
}: {
  zoom: number;
  symbol: Symbol;
  opacity: number;
  label: string;
  selected: boolean;
  select: (symbol: Symbol) => void;
}) {
  return (
    <g
      className="nodrag"
      role="button"
      tabIndex={alpha > 0.5 ? 0 : -1}
      aria-hidden={alpha <= 0.5}
      aria-label={`Declaration: ${symbol.name} · ${symbol.file}:${symbol.line}`}
      aria-description={symbol.role.label}
      data-declaration-role={symbol.role.id}
      opacity={alpha}
      data-symbol-id={symbol.id}
      onClick={(e) => {
        e.stopPropagation();
        select(symbol);
      }}
      onKeyDown={(e) => keyboard(e, () => select(symbol))}
    >
      <title>{symbol.role.label + " · " + symbol.name}</title>
      <SymbolMarker symbol={symbol} selected={selected} zoom={zoom} />
      <text
        x={symbol.x + 9 / zoom}
        y={symbol.y + 4 / zoom}
        fontSize={11 / zoom}
        fill="#fff"
      >
        {label}
      </text>
    </g>
  );
});

export const MapReference = memo(function MapReference({
  from,
  to,
  opacity,
  zoom,
}: {
  zoom: number;
  from: Symbol;
  to: Symbol;
  opacity: number;
}) {
  return (
    <line
      className="map-reference"
      x1={from.x}
      y1={from.y}
      x2={to.x}
      y2={to.y}
      strokeWidth={1 / zoom}
      stroke="#bbd4de"
      strokeOpacity={opacity * 0.65}
      pointerEvents="none"
    />
  );
});

export const MapMainPin = memo(function MapMainPin({
  area,
  symbols,
  origin,
  focus,
  zoom,
}: {
  zoom: number;
  area: MapArea;
  symbols: Symbol[];
  origin: string;
  focus: (path: string) => void;
}) {
  return (
    <g
      className="nodrag map-main-pin"
      role="button"
      tabIndex={0}
      aria-label={`Main theorems: ${area.path} (${symbols.length})`}
      transform={`translate(${area.x + area.width / 2}, ${area.y + area.height / 2}) scale(${1 / zoom})`}
      onClick={(e) => {
        e.stopPropagation();
        focus(area.path);
      }}
      onKeyDown={(e) => keyboard(e, () => focus(area.path))}
    >
      <title>{origin + ": " + symbols.map((s) => s.name).join(", ")}</title>
      <circle r={13} fill="#132631" stroke="#ffdc78" strokeWidth={1.5} />
      <text textAnchor="middle" y={5} fill="#ffdc78" fontSize={18}>
        ★
      </text>
      {symbols.length > 1 && (
        <text x={16} y={4} fill="#ffdc78" fontSize={11}>
          {symbols.length}
        </text>
      )}
    </g>
  );
});
