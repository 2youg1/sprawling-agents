// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createSignal, flush, onCleanup } from "solid-js";

import { cropPdf, pointAt, readableRegion, regionBetween, textWithin, unit } from "./pdf_anchor";
import type { PdfRegion, PdfSelection, Point } from "./pdf_anchor";
import { numbered } from "./pdf_position";

export interface Picked {
  readonly page: number;
  readonly region: PdfRegion;
  readonly text?: string;
}

interface PickingDoor {
  readonly scroller: () => HTMLElement | undefined;
  readonly pages: () => HTMLElement | undefined;
  readonly centeredPage: () => number;
  readonly version: () => string | null;
  readonly attach: () => ((selection: PdfSelection) => void) | undefined;
  readonly offer: (picked: Picked, event: PointerEvent | KeyboardEvent) => void;
}

export function pdfPicking(door: PickingDoor) {
  const [selecting, setSelecting] = createSignal(false);
  const [box, setBox] = createSignal<Picked>();
  const [selectedText, setSelectedText] = createSignal<Picked>();
  const [problem, setProblem] = createSignal<"pdf_crop_failed" | "pdf_changed">();
  const [attaching, setAttaching] = createSignal(false);
  let drag: { pointer: number; bounds: DOMRect; page: number; start: Point } | null = null;
  let alive = true;
  let committing = false;
  onCleanup(() => {
    alive = false;
  });

  function pageOf(target: EventTarget | null): HTMLElement | null {
    return target instanceof Element ? target.closest<HTMLElement>(".pdf-sheet[data-page]") : null;
  }

  function textPicked(node?: HTMLElement | null): Picked | undefined {
    const anchor = document.getSelection()?.anchorNode;
    const pages = door.pages();
    const source =
      node ?? pageOf(anchor instanceof Element ? anchor : (anchor?.parentElement ?? null));
    if (source === null || pages?.contains(source) !== true) return undefined;
    const picked = textWithin(source);
    return picked === null ? undefined : { page: Number(source.dataset.page), ...picked };
  }

  function choose(): Picked | undefined {
    return box() ?? textPicked() ?? selectedText();
  }

  function capture(): Picked | undefined {
    const selection = textPicked();
    if (selection !== undefined) setSelectedText(selection);
    return box() ?? selection ?? selectedText();
  }

  function keepVisible(shown: (page: number) => boolean): void {
    const region = box();
    if (region !== undefined && !shown(region.page)) setBox(undefined);
    const lastText = selectedText();
    if (lastText !== undefined && !shown(lastText.page)) setSelectedText(undefined);
    const native = textPicked();
    if (native !== undefined && !shown(native.page)) document.getSelection()?.removeAllRanges();
    if (drag !== null && !shown(drag.page)) drag = null;
  }

  function anchor(pick: Picked): Point | undefined {
    const pages = door.pages();
    const node = pages === undefined ? null : numbered(pages, pick.page);
    if (node === null) return undefined;
    const bounds = node.getBoundingClientRect();
    return {
      x: bounds.left + (pick.region.x + pick.region.width / 2) * bounds.width,
      y: bounds.top + (pick.region.y + pick.region.height) * bounds.height,
    };
  }

  function begin(event: PointerEvent): void {
    if (event.button !== 0 || !event.isPrimary) return;
    if (door.attach() === undefined) return;
    if (!selecting()) return;
    const node = pageOf(event.target);
    if (node === null) return;
    event.preventDefault();
    const bounds = node.getBoundingClientRect();
    const start = pointAt(bounds, event.clientX, event.clientY);
    drag = { pointer: event.pointerId, bounds, page: Number(node.dataset.page), start };
    setBox({ page: drag.page, region: regionBetween(start, start) });
    node.setPointerCapture(event.pointerId);
  }

  function extend(event: PointerEvent): Picked | undefined {
    if (drag?.pointer !== event.pointerId) return;
    const picked = {
      page: drag.page,
      region: regionBetween(drag.start, pointAt(drag.bounds, event.clientX, event.clientY)),
    };
    setBox(picked);
    return picked;
  }

  function end(event: PointerEvent): void {
    if (drag?.pointer === event.pointerId) {
      const picked = extend(event);
      drag = null;
      if (picked !== undefined && readableRegion(picked.region) && event.pointerType !== "mouse")
        door.offer(picked, event);
    } else if (!selecting()) {
      const text = textPicked();
      if (text !== undefined || event.pointerType === "mouse") setSelectedText(text);
      if (text !== undefined) setBox(undefined);
    }
  }

  function chooseRegion(): void {
    setSelecting(!selecting());
    door.scroller()?.focus({ preventScroll: true });
  }

  function changeRegion(key: string, resize: boolean): boolean {
    const step = 0.02;
    const dx = key === "ArrowLeft" ? -step : key === "ArrowRight" ? step : 0;
    const dy = key === "ArrowUp" ? -step : key === "ArrowDown" ? step : 0;
    if (dx === 0 && dy === 0) return false;
    const page = box()?.page ?? door.centeredPage();
    const current = box()?.region ?? { x: 0.45, y: 0.45, width: 0.1, height: 0.1 };
    const region = resize
      ? {
          ...current,
          width: Math.max(step, Math.min(1 - current.x, current.width + dx)),
          height: Math.max(step, Math.min(1 - current.y, current.height + dy)),
        }
      : {
          ...current,
          x: Math.min(1 - current.width, unit(current.x + dx)),
          y: Math.min(1 - current.height, unit(current.y + dy)),
        };
    setBox({ page, region });
    return true;
  }

  function keyDown(event: KeyboardEvent): void {
    if (event.isComposing) return;
    if (event.key === "Escape" && selecting()) {
      event.preventDefault();
      event.stopPropagation();
      setSelecting(false);
      return;
    }
    if (!selecting() || door.attach() === undefined) return;
    if (changeRegion(event.key, event.shiftKey)) event.preventDefault();
    if (event.key !== "Enter" || event.target !== door.scroller()) return;
    const picked = capture();
    if (picked !== undefined && readableRegion(picked.region)) {
      event.preventDefault();
      door.offer(picked, event);
    }
  }

  async function attach(picked: Picked): Promise<void> {
    if (committing) return;
    setProblem(undefined);
    const source = door.version();
    const target = door.attach();
    if (target === undefined) return;
    const pages = door.pages();
    if (source === null || pages === undefined) {
      setProblem("pdf_crop_failed");
      return;
    }
    const page = numbered(pages, picked.page);
    const anchor = { page: picked.page, region: picked.region, version: source };
    if (picked.text !== undefined) {
      committing = true;
      target({ ...anchor, kind: "pdf-text", text: picked.text });
      setSelecting(false);
      flush();
      committing = false;
      return;
    }
    const canvas = page?.querySelector<HTMLCanvasElement>(".pdf-original");
    if (canvas == null) {
      setProblem("pdf_crop_failed");
      return;
    }
    committing = true;
    setAttaching(true);
    await cropPdf(canvas, picked.region)
      .then((image) => {
        if (!alive) return;
        if (door.version() !== source) {
          setProblem("pdf_changed");
          return;
        }
        if (image === null) {
          setProblem("pdf_crop_failed");
          return;
        }
        target({ ...anchor, kind: "pdf-region", image });
        setSelecting(false);
      })
      .catch(() => {
        if (alive) setProblem("pdf_crop_failed");
      })
      .finally(() => {
        committing = false;
        if (alive) setAttaching(false);
      });
  }

  function canHold(event: PointerEvent): boolean {
    if (selecting()) return false;
    const text =
      event.target instanceof Element && event.target.closest(".pdf-text-layer span") !== null;
    return !text || textPicked() !== undefined;
  }

  return {
    selecting,
    box,
    problem,
    setProblem,
    attaching,
    begin,
    extend,
    end,
    keyDown,
    chooseRegion,
    attach,
    chosen: choose,
    capture,
    keepVisible,
    anchor,
    canHold,
    cancelDrag: () => {
      drag = null;
    },
  };
}
