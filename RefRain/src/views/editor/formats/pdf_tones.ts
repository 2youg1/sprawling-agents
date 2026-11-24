// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const INK_LEVEL = 7;
const PAPER_LEVEL = 247;

function luminance(pixels: Uint8ClampedArray, offset: number): number {
  return (
    0.2126 * (pixels[offset] ?? 0) +
    0.7152 * (pixels[offset + 1] ?? 0) +
    0.0722 * (pixels[offset + 2] ?? 0)
  );
}

function edgeLightness(luma: Float32Array, index: number, width: number): number {
  const centre = luma[index] ?? 0;
  const west = luma[index - 1] ?? centre;
  const east = luma[index + 1] ?? centre;
  const north = luma[index - width] ?? centre;
  const south = luma[index + width] ?? centre;
  const mean = (west + east + north + south) / 4;
  return Math.min(
    Math.max(centre, west, east, north, south),
    Math.max(Math.min(centre, west, east, north, south), centre + (centre - mean) * 0.22),
  );
}

export function neutralTones(
  { data, width, height }: Pick<ImageData, "data" | "width" | "height">,
  scan: boolean,
): void {
  const luma = scan ? new Float32Array(width * height) : null;
  if (luma !== null)
    for (let index = 0; index < luma.length; index++) luma[index] = luminance(data, index * 4);

  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const index = y * width + x;
      const offset = index * 4;
      const red = data[offset] ?? 0;
      const green = data[offset + 1] ?? 0;
      const blue = data[offset + 2] ?? 0;
      const chroma = Math.max(red, green, blue) - Math.min(red, green, blue);
      const neutral = Math.min(1, Math.max(0, (48 - chroma) / 36));
      const interior = x > 0 && x + 1 < width && y > 0 && y + 1 < height;
      const light =
        luma !== null && interior ? edgeLightness(luma, index, width) : luminance(data, offset);
      const tone = ((light - INK_LEVEL) / (PAPER_LEVEL - INK_LEVEL)) * 255;
      data[offset] = tone;
      data[offset + 1] = tone;
      data[offset + 2] = tone;
      data[offset + 3] = (data[offset + 3] ?? 0) * neutral;
    }
  }
}
