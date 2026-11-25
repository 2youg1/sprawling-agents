// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { unit } from "./pdf_anchor";
import type { Size } from "./pdf";
import type { PdfProgress } from "./pdf_anchor";

export const PDF_LAYOUTS = ["continuous", "spread", "single"] as const;
export type PdfLayout = (typeof PDF_LAYOUTS)[number];

export const PDF_SCALE = { min: 50, max: 200, step: 10 } as const;

export function pagesPerTurn(layout: PdfLayout): number {
  return layout === "spread" ? 2 : 1;
}

export function firstPdfPage(layout: PdfLayout, page: number): number {
  const count = pagesPerTurn(layout);
  return Math.floor((page - 1) / count) * count + 1;
}

export function numbered(pageList: HTMLElement, page: number): HTMLElement | null {
  return pageList.querySelector(`.pdf-sheet[data-page="${String(page)}"]`);
}

const FALLBACK: Size = { width: 595, height: 842 };

function lowBit(value: number): number {
  const low = (value & -value) >>> 0;
  if (low !== 0) return low;
  const high = value / 2 ** 32;
  return ((high & -high) >>> 0) * 2 ** 32;
}

export class PdfPageHeights {
  private readonly sizes = new Map<number, Size>();
  private readonly sums = new Map<number, number>();
  private estimate: Size | undefined;

  constructor(readonly count: number) {}

  size(page: number): Size {
    return this.sizes.get(page) ?? this.estimate ?? FALLBACK;
  }

  measure(page: number, size: Size): void {
    const estimate = (this.estimate ??= size);
    const previous = this.sizes.get(page) ?? estimate;
    this.sizes.set(page, size);
    const delta = size.height / size.width - previous.height / previous.width;
    if (delta === 0) return;
    for (let end = page; end <= this.count; end += lowBit(end)) {
      this.sums.set(end, (this.sums.get(end) ?? 0) + delta);
    }
  }

  height(page: number, width: number): number {
    const size = this.size(page);
    return (size.height / size.width) * width;
  }

  top(page: number, width: number, gap: number): number {
    const preceding = page - 1;
    let extra = 0;
    for (let end = preceding; end > 0; end -= lowBit(end)) {
      extra += this.sums.get(end) ?? 0;
    }
    return preceding * (this.height(0, width) + gap) + extra * width;
  }

  pageAt(top: number, width: number, gap: number): number {
    const stride = this.height(0, width) + gap;
    let preceding = 0;
    let extra = 0;
    for (let step = 2 ** Math.floor(Math.log2(this.count)); step >= 1; step /= 2) {
      const end = preceding + step;
      if (end >= this.count) continue;
      const next = extra + (this.sums.get(end) ?? 0);
      if (end * stride + next * width <= top) {
        preceding = end;
        extra = next;
      }
    }
    return preceding + 1;
  }

  position(top: number, width: number, gap: number): PdfProgress {
    const page = this.pageAt(top, width, gap);
    return { page, offset: unit((top - this.top(page, width, gap)) / this.height(page, width)) };
  }
}
