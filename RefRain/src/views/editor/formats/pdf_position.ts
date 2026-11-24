// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { unit } from "./pdf_anchor";
import type { PdfProgress } from "./pdf_anchor";

export function numbered(pageList: HTMLElement, page: number): HTMLElement | null {
  const element = pageList.children.item(page - 1);
  return element instanceof HTMLElement ? element : null;
}

export function positionAt(
  scroller: HTMLElement,
  pageList: HTMLElement,
  height = 0,
): PdfProgress | null {
  if (scroller.clientWidth === 0 || pageList.childElementCount === 0) return null;
  const here = scroller.scrollTop + height;
  let begin = 1;
  let end = pageList.childElementCount;
  while (begin < end) {
    const middle = Math.ceil((begin + end) / 2);
    const next = numbered(pageList, middle);
    if (next !== null && here >= next.offsetTop) begin = middle;
    else end = middle - 1;
  }
  const node = numbered(pageList, begin);
  if (node === null || node.offsetHeight === 0) return null;
  return { page: begin, offset: unit((here - node.offsetTop) / node.offsetHeight) };
}

export function scrollToPosition(
  scroller: HTMLElement,
  pageList: HTMLElement,
  progress: PdfProgress,
): void {
  if (!Number.isSafeInteger(progress.page) || !Number.isFinite(progress.offset)) return;
  const node = numbered(pageList, Math.min(pageList.childElementCount, progress.page));
  if (node === null) return;
  scroller.scrollTop =
    progress.page === 1 && progress.offset <= 0
      ? 0
      : node.offsetTop + unit(progress.offset) * node.offsetHeight;
}
