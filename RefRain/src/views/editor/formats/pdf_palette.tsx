// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createSignal, onSettled } from "solid-js";
import { storeValue } from "../../../core/solid_store";
import { wallpaperSettings } from "../../../core/wallpaper";

type Channels = readonly [number, number, number];

interface Palette {
  readonly paper: Channels;
  readonly ink: Channels;
}

const COOL: Palette = { paper: [7, 14, 25], ink: [195, 205, 212] };

function converted(context: CanvasRenderingContext2D, css: string, fallback: Channels): Channels {
  if (!CSS.supports("color", css)) return fallback;
  context.fillStyle = css;
  context.fillRect(0, 0, 1, 1);
  const { data } = context.getImageData(0, 0, 1, 1);
  return [data[0] ?? fallback[0], data[1] ?? fallback[1], data[2] ?? fallback[2]];
}

function chosenPalette(ctx: CanvasRenderingContext2D | null): Palette {
  if (ctx === null) return COOL;
  const style = getComputedStyle(document.documentElement);
  return {
    paper: converted(ctx, style.getPropertyValue("--pdf-paper").trim(), COOL.paper),
    ink: converted(ctx, style.getPropertyValue("--pdf-ink").trim(), COOL.ink),
  };
}

export function PdfToneFilter(props: { readonly id: string }) {
  const appearance = storeValue(wallpaperSettings());
  const [palette, setPalette] = createSignal(COOL);
  const sampler = document.createElement("canvas");
  sampler.width = sampler.height = 1;
  const ctx = sampler.getContext("2d", { colorSpace: "srgb", willReadFrequently: true });
  const refresh = (): void => {
    setPalette(chosenPalette(ctx));
  };
  createEffect(() => appearance().look.choice.brightness, refresh);
  onSettled(() => {
    refresh();
    const forced = window.matchMedia("(forced-colors: active)");
    forced.addEventListener("change", refresh);
    return () => forced.removeEventListener("change", refresh);
  });
  const range = (part: number): string => {
    const { ink, paper } = palette();
    return `${String((ink[part] ?? 0) / 255)} ${String((paper[part] ?? 0) / 255)}`;
  };
  return (
    <svg aria-hidden="true" class="pdf-tone-map" width="0" height="0">
      <defs>
        <filter
          id={props.id}
          x="0"
          y="0"
          width="100%"
          height="100%"
          color-interpolation-filters="sRGB"
        >
          <feComponentTransfer>
            <feFuncR type="table" tableValues={range(0)} />
            <feFuncG type="table" tableValues={range(1)} />
            <feFuncB type="table" tableValues={range(2)} />
          </feComponentTransfer>
        </filter>
      </defs>
    </svg>
  );
}
