// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The one shape the city drawing is built from: a rectangle whose
// corners are superellipse arcs, so a building on the map has the same
// continuous curvature the panels around it have. `theme.css` carries
// the exponent per scale; this takes it as a parameter.

export interface Box {
  readonly x: number;
  readonly y: number;
  readonly w: number;
  readonly h: number;
}

const STEPS = 8;

function corner(cx: number, cy: number, r: number, n: number, from: number, sx: number, sy: number): string {
  let d = "";
  for (let i = 0; i <= STEPS; i += 1) {
    const t = from + (i / STEPS) * (Math.PI / 2);
    const c = Math.cos(t);
    const s = Math.sin(t);
    const px = cx + sx * r * Math.sign(c) * Math.abs(c) ** (2 / n);
    const py = cy + sy * r * Math.sign(s) * Math.abs(s) ** (2 / n);
    d += `${i === 0 ? "L" : "L"}${px.toFixed(1)} ${py.toFixed(1)}`;
  }
  return d;
}

// An SVG path for `box` with corner radius `r` and exponent `n`
// (`4` is the panel's squircle, `2` an ordinary arc).
export function squircle(box: Box, r: number, n: number): string {
  const radius = Math.min(r, box.w / 2, box.h / 2);
  const { x, y, w, h } = box;
  const right = x + w;
  const bottom = y + h;
  return (
    `M${(x + radius).toFixed(1)} ${y.toFixed(1)}` +
    `L${(right - radius).toFixed(1)} ${y.toFixed(1)}` +
    corner(right - radius, y + radius, radius, n, -Math.PI / 2, 1, 1) +
    `L${right.toFixed(1)} ${(bottom - radius).toFixed(1)}` +
    corner(right - radius, bottom - radius, radius, n, 0, 1, 1) +
    `L${(x + radius).toFixed(1)} ${bottom.toFixed(1)}` +
    corner(x + radius, bottom - radius, radius, n, Math.PI / 2, 1, 1) +
    `L${x.toFixed(1)} ${(y + radius).toFixed(1)}` +
    corner(x + radius, y + radius, radius, n, Math.PI, 1, 1) +
    "Z"
  );
}
