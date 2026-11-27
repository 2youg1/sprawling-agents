// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { PDFDocumentProxy } from "pdfjs-dist";
import {
  createEffect,
  createMemo,
  createSignal,
  createUniqueId,
  flush,
  For,
  onCleanup,
  onSettled,
  Show,
  untrack,
} from "solid-js";

import { say } from "../../../core/lang";
import { scopedUi } from "../../../ui";
import { attachContextTools, ContextTools, createContextTools } from "../../parts/context_tools";
import Glyph from "../../parts/glyph.solid";
import { cropFullPage, type PdfDetail, type Size } from "./pdf";
import { pointAt, readableRegion, unit } from "./pdf_anchor";
import type { PdfProgress, PdfRegion, Point } from "./pdf_anchor";
import { PdfSheet } from "./pdf_page_view";
import type { Graphics, PdfProps } from "./pdf_loading";
import { PdfToneFilter } from "./pdf_palette";
import { pdfPicking, type Picked } from "./pdf_pick";
import { pdfWindow } from "./pdf_window";
import { PdfRasterCache } from "./pdf_raster_cache";
import {
  bookPageWidth,
  firstPdfPage,
  numbered,
  pagesPerTurn,
  PdfPageHeights,
  pdfViewport,
  type PdfLayout,
} from "./pdf_position";
import { pdfActions, PdfReaderControls, PdfZoom } from "./pdf_tools";
import "./pdf.css";

interface ReadingProps extends PdfProps {
  readonly pdf: PDFDocumentProxy;
  readonly detailDocument: PdfDetail;
  readonly graphics: Graphics | undefined;
}

interface Zoomed {
  readonly page: number;
  readonly anchor: Point;
}

export function PdfReading(props: ReadingProps) {
  const tone = createUniqueId();
  const onProgress = untrack(() => props.onProgress);
  const count = untrack(() => props.pdf.numPages);
  const heights = new PdfPageHeights(count);
  const rasters = new PdfRasterCache();
  onCleanup(() => rasters.close());
  const [sizeRevision, setSizeRevision] = createSignal(0);
  const [width, setWidth] = createSignal(0);
  const [layout, setLayout] = createSignal<PdfLayout>("continuous");
  const [scale, setScale] = createSignal(100);
  const [pageAt, setPageAt] = createSignal(1);
  const first = () => firstPdfPage(layout(), pageAt());
  const turns = () => pagesPerTurn(layout());
  const continuous = () => layout() === "continuous";
  const [original, setOriginal] = createSignal(false);
  const [night, setNight] = createSignal(untrack(() => props.initialNight ?? false));
  const [zoom, setZoom] = createSignal<Zoomed>();
  let scroller: HTMLDivElement | undefined;
  let pages: HTMLDivElement | undefined;
  const menus = createContextTools();
  const canChangeBrightness = scopedUi() === undefined;
  let viewport = { width: 0, height: 0, paddingTop: 0, gap: 0 };
  let placed: { readonly point: PdfProgress; readonly top: number } | undefined;
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  let nextProgress: PdfProgress | undefined;

  const sizeOf = (page: number): Size => {
    sizeRevision();
    return heights.size(page);
  };
  const pageTop = (page: number): number => {
    sizeRevision();
    return heights.top(page, width(), viewport.gap);
  };
  const stripHeight = () => pageTop(count + 1) - viewport.gap;
  const highlight = createMemo(() => {
    const reference = props.reference;
    return reference != null &&
      reference.version === props.version &&
      readableRegion(reference.region) &&
      Number.isSafeInteger(reference.page) &&
      reference.page >= 1 &&
      reference.page <= count
      ? reference
      : null;
  });

  function position(height = 0): PdfProgress | null {
    if (scroller === undefined || pages === undefined || viewport.width <= 0) return null;
    const top = scroller.scrollTop + height;
    if (continuous()) return heights.position(top - viewport.paddingTop, width(), viewport.gap);
    const page = pageAt();
    const node = numbered(pages, page);
    if (node === null || node.offsetHeight === 0) return null;
    return { page, offset: unit((top - node.offsetTop) / node.offsetHeight) };
  }

  function readingPoint(): PdfProgress | null {
    return placed !== undefined && scroller?.scrollTop === placed.top ? placed.point : position();
  }

  function scroll(point: PdfProgress): void {
    if (scroller === undefined || pages === undefined) return;
    const { page, offset } = point;
    if (page === 1 && offset <= 0) scroller.scrollTop = 0;
    else if (continuous())
      scroller.scrollTop =
        viewport.paddingTop + pageTop(page) + unit(offset) * heights.height(page, width());
    else {
      const node = numbered(pages, page);
      if (node !== null) scroller.scrollTop = node.offsetTop + unit(offset) * node.offsetHeight;
    }
    placed = { point, top: scroller.scrollTop };
    windowed.placed(scroller.scrollTop);
    updateWindow();
  }

  function updateWindow(): void {
    if (scroller === undefined || viewport.width <= 0 || !continuous()) return;
    const top = scroller.scrollTop - viewport.paddingTop;
    windowed.show(
      heights.pageAt(top, width(), viewport.gap),
      heights.pageAt(top + viewport.height, width(), viewport.gap),
    );
  }

  function place(point: PdfProgress, nextLayout = layout(), nextScale = scale()): void {
    const view = scroller;
    const list = pages;
    if (view === undefined || list === undefined) return;
    if (nextLayout !== layout()) {
      setLayout(nextLayout);
      flush();
    }
    viewport = pdfViewport(view, list, nextLayout);
    if (viewport.width <= 0 || viewport.height <= 0) return;
    const page = Math.min(count, Math.max(1, point.page));
    const start = firstPdfPage(nextLayout, page);
    const columns = Math.min(pagesPerTurn(nextLayout), count - start + 1);
    if (nextLayout !== "continuous")
      pick.keepVisible((selected) => selected >= start && selected < start + columns);
    const ratios = Array.from({ length: columns }, (_, index) => heights.height(start + index, 1));
    const fitted = nextLayout === "spread" ? bookPageWidth(viewport, ratios) : viewport.width;
    windowed.settle();
    setPageAt(page);
    setScale(nextScale);
    setWidth(Math.max(1, Math.floor((fitted * nextScale) / 100)));
    flush();
    scroll({ page, offset: point.offset });
  }

  function moveReading(nextLayout: PdfLayout, nextScale: number): void {
    place(readingPoint() ?? { page: pageAt(), offset: 0 }, nextLayout, nextScale);
    recordScroll();
  }

  function turn(direction: number): void {
    if (scroller !== undefined) scroller.scrollLeft = 0;
    place({ page: first() + direction * turns(), offset: 0 });
    recordScroll();
  }

  const pick = pdfPicking({
    scroller: () => scroller,
    pages: () => pages,
    centeredPage: () => position((scroller?.clientHeight ?? 0) / 2)?.page ?? 1,
    version: () => props.version,
    attach: () => props.onSelection,
    cropFull: (picked, signal) => cropFullPage(props.detailDocument, width(), picked, signal),
    offer: (picked, event) => {
      const pointer = event instanceof PointerEvent;
      menus.open(request(picked), {
        kind: pointer ? "touch" : "keyboard",
        event,
        x: pointer ? event.clientX : 0,
        y: pointer ? event.clientY : 0,
      });
    },
  });

  const windowed = pdfWindow({
    count,
    width,
    continuous,
    first,
    turns,
    picked: () => pick.box()?.page,
  });

  function saveProgress(): void {
    clearTimeout(saveTimer);
    const recorded = nextProgress;
    nextProgress = undefined;
    if (recorded !== undefined) onProgress?.(recorded);
  }

  function recordScroll(): void {
    if (onProgress === undefined) return;
    const recorded = readingPoint();
    if (recorded === null) return;
    nextProgress = recorded;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(saveProgress, 250);
  }

  onSettled(() => {
    const view = scroller;
    const list = pages;
    if (view === undefined || list === undefined) return;

    let resizeFrame: number | undefined;
    let closed = false;
    const resize = (): void => {
      if (closed) return;
      const next = pdfViewport(view, list, layout());
      if (next.width <= 0 || next.height <= 0) return;
      if (
        next.width === viewport.width &&
        next.height === viewport.height &&
        next.gap === viewport.gap &&
        next.paddingTop === viewport.paddingTop
      )
        return;
      const source = highlight();
      const desired =
        viewport.width > 0
          ? readingPoint()
          : source === null
            ? props.progress
            : { page: source.page, offset: source.region.y };
      place(desired ?? { page: pageAt(), offset: 0 });
    };
    const observer = new ResizeObserver(() => {
      if (resizeFrame !== undefined) return;
      resizeFrame = window.requestAnimationFrame(() => {
        resizeFrame = undefined;
        resize();
      });
    });
    queueMicrotask(resize);
    observer.observe(view);
    view.addEventListener("keydown", pick.keyDown);
    const closeTools = attachContextTools(
      view,
      menus,
      (trigger) => {
        const selected = pick.capture();
        const node =
          trigger.event?.target instanceof Element
            ? trigger.event.target.closest<HTMLElement>(".pdf-sheet[data-page]")
            : null;
        return request(
          selected,
          trigger.kind === "keyboard" ||
            node === null ||
            trigger.x === undefined ||
            trigger.y === undefined
            ? undefined
            : {
                page: Number(node.dataset.page),
                anchor: pointAt(node.getBoundingClientRect(), trigger.x, trigger.y),
              },
        );
      },
      { longPress: pick.canHold },
    );
    window.addEventListener("pagehide", saveProgress);
    return () => {
      saveProgress();
      closed = true;
      if (resizeFrame !== undefined) window.cancelAnimationFrame(resizeFrame);
      observer.disconnect();
      view.removeEventListener("keydown", pick.keyDown);
      closeTools();
      window.removeEventListener("pagehide", saveProgress);
    };
  });

  createEffect(
    () => ({ requested: props.reference, source: highlight() }),
    ({ requested, source }) => {
      if (requested == null) return;
      if (source === null) {
        pick.setProblem("pdf_changed");
        return;
      }
      pick.setProblem(undefined);
      let cancelled = false;
      queueMicrotask(() => {
        if (cancelled) return;
        place({ page: source.page, offset: source.region.y });
        recordScroll();
      });
      return () => {
        cancelled = true;
      };
    },
  );

  function measure(page: number, size: Size): void {
    const prior = heights.size(page);
    const base = heights.size(0);
    const changed = prior.height / prior.width !== size.height / size.width;
    const before = changed ? readingPoint() : null;
    heights.measure(page, size);
    if (!changed) return;
    setSizeRevision((revision) => revision + 1);
    if (layout() === "spread") {
      place(before ?? { page: pageAt(), offset: 0 });
    } else if (before !== null && (page <= before.page || heights.size(0) !== base)) {
      flush();
      scroll(before);
    } else updateWindow();
  }

  function openZoom(target?: Zoomed): void {
    const chosen = pick.chosen();
    if (chosen !== undefined) {
      setZoom({
        page: chosen.page,
        anchor: {
          x: chosen.region.x + chosen.region.width / 2,
          y: chosen.region.y + chosen.region.height / 2,
        },
      });
    } else if (target !== undefined) {
      setZoom(target);
    } else {
      const middle = position((scroller?.clientHeight ?? 0) / 2);
      setZoom({ page: middle?.page ?? 1, anchor: { x: 0.5, y: middle?.offset ?? 0.5 } });
    }
  }

  function closeZoom(): void {
    setZoom(undefined);
    scroller?.focus({ preventScroll: true });
  }

  function request(selected: Picked | undefined, at?: Zoomed) {
    const source = highlight();
    const tools = pdfActions({
      lang: props.lang,
      selecting: props.onSelection !== undefined ? pick : undefined,
      selected,
      brightness: canChangeBrightness,
      magnify: () => openZoom(at),
      original: setOriginal,
      reference:
        source === null
          ? undefined
          : () => {
              place({ page: source.page, offset: source.region.y });
              recordScroll();
            },
      details: () => ({ lang: props.lang, version: props.version, width: width(), pages: count }),
    });
    const anchor = selected === undefined ? undefined : pick.anchor(selected);
    return { tools, label: props.name, anchor, restore: focusReader };
  }

  function focusReader(): void {
    scroller?.focus({ preventScroll: true });
  }

  const boxOn = (page: number): PdfRegion | undefined => {
    const box = pick.box();
    return box?.page === page && readableRegion(box.region) ? box.region : undefined;
  };

  return (
    <div
      class="pdf-view"
      data-original={String(original())}
      data-night={String(night() && !original())}
      data-layout={layout()}
      data-webgl2={String(props.graphics?.webgl2 ?? false)}
      data-webgpu={String(props.graphics?.webgpu ?? false)}
      style={{ "--pdf-tone-filter": `url(#${tone})` }}
      role="group"
      aria-label={props.name}
    >
      <PdfToneFilter id={tone} />
      <PdfReaderControls
        lang={props.lang}
        layout={layout()}
        scale={scale()}
        first={first()}
        last={Math.min(first() + turns() - 1, count)}
        total={count}
        night={night()}
        onNight={() => setNight((active) => !active)}
        onLayout={(next) => moveReading(next, next === "spread" ? 100 : scale())}
        onScale={(next) => moveReading(layout(), next)}
        onTurn={turn}
      />
      <div
        class={["pdf-scroll", { "pdf-selecting": pick.selecting() }]}
        ref={(node) => {
          scroller = node;
        }}
        tabindex="0"
        role="region"
        aria-label={props.name}
        aria-haspopup="menu"
        aria-keyshortcuts="Shift+F10 ContextMenu"
        onScroll={(event) => {
          windowed.trackScroll(event.currentTarget.scrollTop);
          updateWindow();
          recordScroll();
        }}
        onPointerDown={pick.begin}
        onPointerMove={pick.extend}
        onPointerUp={pick.end}
        onPointerCancel={pick.cancelDrag}
      >
        <div
          class="pdf-pages"
          ref={(node) => {
            pages = node;
          }}
          style={{
            "--pdf-page-width": `${String(width() || 1)}px`,
            "block-size": continuous() ? `${String(stripHeight())}px` : undefined,
          }}
        >
          <For each={windowed.pages()}>
            {(page) => {
              const size = createMemo(() => sizeOf(page));
              const inView = createMemo(() => windowed.inView(page));
              return (
                <PdfSheet
                  page={page}
                  total={count}
                  size={size()}
                  top={continuous() ? pageTop(page) : undefined}
                  box={boxOn(page)}
                  reference={highlight()?.page === page ? highlight()?.region : undefined}
                  pdf={props.pdf}
                  detailDocument={props.detailDocument}
                  width={width()}
                  lang={props.lang}
                  detail={inView() && !windowed.moving()}
                  prepare={inView() || windowed.prefetch().has(page)}
                  onReady={(pageWidth, painted) => windowed.noteReady(page, pageWidth, painted)}
                  cache={rasters}
                  measured={(size) => measure(page, size)}
                />
              );
            }}
          </For>
        </div>
      </div>
      <ContextTools controller={menus} lang={props.lang} />
      <Show when={pick.attaching()}>
        <p class="pdf-feedback" role="status" aria-busy="true">
          <Glyph name="pulse" />
          {say(props.lang, "pdf_attaching")}
        </p>
      </Show>
      <Show when={pick.problem()}>
        {(problem) => (
          <p class="pdf-feedback" role="status">
            <Glyph name="cross" />
            {say(props.lang, problem())}
          </p>
        )}
      </Show>
      <Show when={zoom()}>
        {(entry) => (
          <PdfZoom
            pdf={props.pdf}
            detailDocument={props.detailDocument}
            page={entry().page}
            anchor={entry().anchor}
            lang={props.lang}
            width={width()}
            size={sizeOf(entry().page)}
            close={closeZoom}
          />
        )}
      </Show>
    </div>
  );
}
