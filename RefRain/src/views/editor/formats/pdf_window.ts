// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, onCleanup } from "solid-js";

const TRAILING_PAGES = 2;
const LEADING_PAGES = 4;
const PREVIEW_CONCURRENCY = 3;
const SCROLL_SETTLE = 110;

export type RasterStage = "preview" | "full" | "empty";

interface PageRange {
  readonly start: number;
  readonly end: number;
}

interface WindowInput {
  readonly count: number;
  readonly width: () => number;
  readonly continuous: () => boolean;
  readonly first: () => number;
  readonly turns: () => number;
  readonly picked: () => number | undefined;
}

export function pdfWindow(input: WindowInput) {
  const [visible, setVisible] = createSignal<PageRange>(
    { start: 1, end: 1 },
    {
      equals: (a, b) => a.start === b.start && a.end === b.end,
    },
  );
  const [ready, setReady] = createSignal<ReadonlyMap<number, number>>(new Map());
  const [direction, setDirection] = createSignal(1);
  const [moving, setMoving] = createSignal(false);
  const [opening, setOpening] = createSignal(true);
  const [rested, setRested] = createSignal(false);
  const picked = createMemo(input.picked);
  let lastScrollTop = 0;
  let scrollTimer: ReturnType<typeof setTimeout> | undefined;
  let idleTimer: ReturnType<typeof setTimeout> | undefined;
  onCleanup(() => {
    clearTimeout(scrollTimer);
    clearTimeout(idleTimer);
  });

  const nearby = createMemo(() => {
    const { start, end } = visible();
    const forward = direction() > 0;
    return {
      start: Math.max(1, start - (forward ? TRAILING_PAGES : LEADING_PAGES)),
      end: Math.min(input.count, end + (forward ? LEADING_PAGES : TRAILING_PAGES)),
    };
  });
  const pages = createMemo(() => {
    if (!input.continuous()) {
      const first = input.first();
      return Array.from(
        { length: Math.min(input.turns(), input.count - first + 1) },
        (_, i) => first + i,
      );
    }
    const { start, end } = nearby();
    const numbers = Array.from({ length: end - start + 1 }, (_, i) => start + i);
    const selected = picked();
    if (selected !== undefined && selected < start) numbers.unshift(selected);
    if (selected !== undefined && selected > end) numbers.push(selected);
    return numbers;
  });

  const prefetch = createMemo(() => {
    const pending = new Set<number>();
    if (!input.continuous()) return pending;
    const painted = ready();
    const width = input.width();
    const { start, end } = visible();
    const scrolling = moving();
    if (!scrolling && (opening() || !rested())) return pending;
    let available = scrolling ? PREVIEW_CONCURRENCY : 1;
    for (let page = start; page <= end; page++) {
      if (painted.get(page) === width) continue;
      if (!scrolling) return pending;
      available--;
    }
    if (available <= 0) return pending;
    const range = nearby();
    const forward = direction() > 0;
    const leading = forward ? end + 1 : start - 1;
    const boundary = forward ? range.end : range.start;
    const step = forward ? 1 : -1;
    for (
      let page = leading;
      (boundary - page) * step >= 0 && pending.size < available;
      page += step
    ) {
      if (painted.get(page) !== width) pending.add(page);
    }
    return pending;
  });

  function settle(): void {
    clearTimeout(scrollTimer);
    setMoving(false);
    deferIdle();
  }

  function deferIdle(): void {
    clearTimeout(idleTimer);
    setRested(false);
    idleTimer = setTimeout(() => setRested(true), SCROLL_SETTLE);
  }

  function trackScroll(top: number): void {
    if (top === lastScrollTop || !input.continuous()) return;
    setDirection(top > lastScrollTop ? 1 : -1);
    lastScrollTop = top;
    setMoving(true);
    clearTimeout(scrollTimer);
    scrollTimer = setTimeout(settle, SCROLL_SETTLE);
  }

  function noteReady(page: number, width: number, stage: RasterStage): void {
    if (stage === "full" && opening()) {
      setOpening(false);
      deferIdle();
    }
    setReady((prior) => {
      const preparedWidth = prior.get(page);
      if (stage === "empty") {
        if (preparedWidth !== width) return prior;
        const next = new Map(prior);
        next.delete(page);
        return next;
      }
      if (preparedWidth === width) return prior;
      return new Map(prior).set(page, width);
    });
  }

  return {
    pages,
    prefetch,
    moving,
    settle,
    trackScroll,
    noteReady,
    show: (start: number, end: number) => setVisible({ start, end }),
    placed: (top: number) => {
      lastScrollTop = top;
    },
    inView: (page: number) => {
      if (!input.continuous()) return true;
      const { start, end } = visible();
      return page >= start && page <= end;
    },
  };
}
