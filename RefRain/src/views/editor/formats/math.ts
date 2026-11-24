// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Result } from "effect";

export type MathPlacement = "inline" | "display";

export function formulaOf(
  source: string,
): { readonly tex: string; readonly placed: MathPlacement } | null {
  const fence = source.startsWith("$$") && source.endsWith("$$") ? "$$" : "$";
  if (!source.startsWith(fence) || !source.endsWith(fence) || source.length <= 2 * fence.length)
    return null;
  const tex = source.slice(fence.length, -fence.length);
  return tex.trim() === "" ? null : { tex, placed: fence === "$$" ? "display" : "inline" };
}

export async function drawMath(
  tex: string,
  host: HTMLElement,
  placed: MathPlacement,
  current: () => boolean,
): Promise<boolean> {
  const { default: katex } = await import("katex");
  if (!current()) return false;
  return Result.isSuccess(
    Result.try(() => {
      katex.render(tex, host, {
        displayMode: placed === "display",
        output: "mathml",
        throwOnError: true,
        strict: "ignore",
        trust: false,
        maxExpand: 1000,
        maxSize: 20,
      });
    }),
  );
}
