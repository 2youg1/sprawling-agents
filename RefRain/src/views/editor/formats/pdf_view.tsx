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
import type { PdfDetail, Size } from "./pdf";
import { pointAt, readableRegion, unit } from "./pdf_anchor";
import type { PdfProgress, PdfRegion, Point } from "./pdf_anchor";
import { PdfSheet } from "./pdf_page_view";
import type { Graphics, PdfProps } from "./pdf_loading";
import { PdfToneFilter } from "./pdf_palette";
import { pdfPicking, type Picked } from "./pdf_pick";
import { pdfWindow } from "./pdf_window";
import { PdfRasterCache } from "./pdf_raster_cache";
import {
  firstPdfPage,
  numbered,
  pagesPerTurn,
  PdfPageHeights,
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
  const [zoom, setZoom] = createSignal<Zoomed>();
  let scroller: HTMLDivElement | undefined;
  let pages: HTMLDivElement | undefined;
  const menus = createContextTools();
  const canChangeBrightness = scopedUi() === undefined;
  let availableWidth = 0;
  let viewportHeight = 0;
  let paddingTop = 0;
  let gap = 0;
  let placed: { readonly point: PdfProgress; readonly top: number } | undefined;
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  let nextProgress: PdfProgress | undefined;

  const sizeOf = (page: number): Size => {
    sizeRevision();
    return heights.size(page);
  };
  const pageTop = (page: number): number => {
    sizeRevision();
    return heights.top(page, width(), gap);
  };
  const stripHeight = () => pageTop(count + 1) - gap;
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
    if (scroller === undefined || pages === undefined || availableWidth === 0) return null;
    const top = scroller.scrollTop + height;
    if (continuous()) return heights.position(top - paddingTop, width(), gap);
    const page = pageAt();
    const node = numbered(pages, page);
    if (node === null || node.offsetHeight === 0) return null;
    return { page, offset: unit((top - node.offsetTop) / node.offsetHeight) };
  }

  function center(): PdfProgress | null {
    return position((scroller?.clientHeight ?? 0) / 2);
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
        paddingTop + pageTop(page) + unit(offset) * heights.height(page, width());
    else {
      const node = numbered(pages, page);
      if (node !== null) scroller.scrollTop = node.offsetTop + unit(offset) * node.offsetHeight;
    }
    placed = { point, top: scroller.scrollTop };
    windowed.placed(scroller.scrollTop);
    updateWindow();
  }

  function updateWindow(): void {
    if (scroller === undefined || availableWidth === 0 || !continuous()) return;
    const top = scroller.scrollTop - paddingTop;
    windowed.show(
      heights.pageAt(top, width(), gap),
      heights.pageAt(top + viewportHeight, width(), gap),
    );
  }

  function place(point: PdfProgress, nextLayout = layout(), nextScale = scale()): void {
    const view = scroller;
    const list = pages;
    if (view === undefined || list === undefined || availableWidth === 0) return;
    const page = Math.min(count, Math.max(1, point.page));
    const columns = pagesPerTurn(nextLayout);
    if (nextLayout !== "continuous") {
      const start = firstPdfPage(nextLayout, page);
      pick.keepVisible((selected) => selected >= start && selected < start + columns);
    }
    windowed.settle();
    setPageAt(page);
    setLayout(nextLayout);
    setScale(nextScale);
    setWidth((Math.max(1, (availableWidth - (columns - 1) * gap) / columns) * nextScale) / 100);
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
    centeredPage: () => center()?.page ?? 1,
    version: () => props.version,
    attach: () => props.onSelection,
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
    function readWidth(): void {
      if (view === undefined || list === undefined) return;
      const style = getComputedStyle(view);
      viewportHeight = view.clientHeight;
      const available =
        view.clientWidth -
        Number.parseFloat(style.paddingLeft) -
        Number.parseFloat(style.paddingRight);
      if (available <= 0) return;
      if (available === availableWidth) {
        updateWindow();
        return;
      }
      const source = highlight();
      const desired =
        availableWidth > 0
          ? readingPoint()
          : source === null
            ? props.progress
            : { page: source.page, offset: source.region.y };
      availableWidth = available;
      paddingTop = Number.parseFloat(style.paddingTop);
      gap = Number.parseFloat(getComputedStyle(list).rowGap) || 0;
      place(desired ?? { page: 1, offset: 0 });
    }
    const observer = new ResizeObserver(() => {
      if (resizeFrame !== undefined) return;
      resizeFrame = window.requestAnimationFrame(() => {
        resizeFrame = undefined;
        readWidth();
      });
    });
    let closed = false;
    queueMicrotask(() => {
      if (!closed) readWidth();
    });
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
    if (before !== null && (page <= before.page || heights.size(0) !== base)) {
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
      const middle = center();
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
        onLayout={(next) => moveReading(next, scale())}
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
          data-layout={layout()}
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
