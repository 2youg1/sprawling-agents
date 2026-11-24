// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export type Rgb = readonly [number, number, number];
export type Rgba = readonly [number, number, number, number];

export function over([r, g, b, a]: Rgba, [dr, dg, db]: Rgb): Rgb {
  const remain = 1 - a;
  return [r * a + dr * remain, g * a + dg * remain, b * a + db * remain];
}

export function apcaOn(text: Rgba, surface: Rgba, backdrop: Rgb): number {
  const background = over(surface, backdrop);
  return apcaLc(over(text, background), background);
}

export function apcaLc(text: Rgb, surface: Rgb): number {
  const textY = softClamp(luminance(text));
  const surfaceY = softClamp(luminance(surface));
  if (Math.abs(surfaceY - textY) < 0.0005) return 0;
  if (surfaceY > textY) {
    const sapc = (surfaceY ** 0.56 - textY ** 0.57) * 1.14;
    return sapc < 0.1 ? 0 : (sapc - 0.027) * 100;
  }
  const sapc = (surfaceY ** 0.65 - textY ** 0.62) * 1.14;
  return sapc > -0.1 ? 0 : (sapc + 0.027) * 100;
}

function luminance([red, green, blue]: Rgb): number {
  return (
    0.2126729 * (red / 255) ** 2.4 +
    0.7151522 * (green / 255) ** 2.4 +
    0.072175 * (blue / 255) ** 2.4
  );
}

function softClamp(y: number): number {
  return y > 0.022 ? y : y + (0.022 - y) ** 1.414;
}
