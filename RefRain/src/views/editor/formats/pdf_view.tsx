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
  onSettled,
  Show,
  untrack,
} from "solid-js";

import { fill, say } from "../../../core/lang";
import { scopedUi } from "../../../ui";
import { attachContextTools, ContextTools, createContextTools } from "../../parts/context_tools";
import Glyph from "../../parts/glyph.solid";
import type { Size } from "./pdf";
import { pointAt, readableRegion, regionStyle } from "./pdf_anchor";
import type { PdfProgress, PdfRegion, Point } from "./pdf_anchor";
import { PdfPage } from "./pdf_page_view";
import type { Graphics, PdfProps } from "./pdf_loading";
import { PdfToneFilter } from "./pdf_palette";
import { pdfPicking, type Picked } from "./pdf_pick";
import {
  firstPdfPage,
  pagesPerTurn,
  positionAt,
  scrollToPosition,
  type PdfLayout,
} from "./pdf_position";
import { pdfActions, PdfReaderControls, PdfZoom } from "./pdf_tools";
import "./pdf.css";

interface ReadingProps extends PdfProps {
  readonly pdf: PDFDocumentProxy;
  readonly graphics: Graphics | undefined;
}

interface Zoomed {
  readonly page: number;
  readonly anchor: Point;
}

const FALLBACK = { width: 595, height: 842 };
const MAX_NEARBY = 5;

export function PdfReading(props: ReadingProps) {
  const tone = createUniqueId();
  const onProgress = untrack(() => props.onProgress);
  const [sizes, setSizes] = createSignal<ReadonlyMap<number, Size>>(new Map());
  const [visible, setVisible] = createSignal<ReadonlySet<number>>(new Set());
  const [width, setWidth] = createSignal(0);
  const [layout, setLayout] = createSignal<PdfLayout>("continuous");
  const [scale, setScale] = createSignal(100);
  const [pageAt, setPageAt] = createSignal(1);
  const first = () => firstPdfPage(layout(), pageAt());
  const turns = () => pagesPerTurn(layout());
  const shown = (page: number) =>
    layout() === "continuous" || (page >= first() && page < first() + turns());
  const [original, setOriginal] = createSignal(false);
  const [zoom, setZoom] = createSignal<Zoomed>();
  let scroller: HTMLDivElement | undefined;
  let pages: HTMLDivElement | undefined;
  let surface: HTMLDivElement | undefined;
  const menus = createContextTools();
  const canChangeBrightness = scopedUi() === undefined;
  let availableWidth = 0;
  let placed: { readonly point: PdfProgress; readonly top: number } | undefined;
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  let nextProgress: PdfProgress | undefined;

  const count = createMemo(() => props.pdf.numPages);
  const numbers = createMemo(() => Array.from({ length: count() }, (_, index) => index + 1));
  const sizeOf = (page: number): Size => sizes().get(page) ?? sizes().get(1) ?? FALLBACK;
  const highlight = createMemo(() => {
    const reference = props.reference;
    return reference != null &&
      reference.version === props.version &&
      readableRegion(reference.region) &&
      Number.isSafeInteger(reference.page) &&
      reference.page >= 1 &&
      reference.page <= count()
      ? reference
      : null;
  });

  function position(height = 0): PdfProgress | null {
    if (scroller === undefined || pages === undefined || availableWidth === 0) return null;
    const page = layout() === "continuous" ? undefined : pageAt();
    return positionAt(scroller, pages, height, page);
  }

  function center(): PdfProgress | null {
    return position((scroller?.clientHeight ?? 0) / 2);
  }

  function readingPoint(): PdfProgress | null {
    return placed !== undefined && scroller?.scrollTop === placed.top ? placed.point : position();
  }

  function scroll(point: PdfProgress): void {
    if (scroller === undefined || pages === undefined) return;
    scrollToPosition(scroller, pages, point);
    placed = { point, top: scroller.scrollTop };
  }

  function place(point: PdfProgress, nextLayout = layout(), nextScale = scale()): void {
    const view = scroller;
    const list = pages;
    if (view === undefined || list === undefined || availableWidth === 0) return;
    const page = Math.min(count(), Math.max(1, point.page));
    setPageAt(page);
    setLayout(nextLayout);
    setScale(nextScale);
    const columns = pagesPerTurn(nextLayout);
    const gap = Number.parseFloat(getComputedStyle(list).columnGap) || 0;
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
    const host = surface;
    if (view === undefined || list === undefined || host === undefined) return;

    let resizeFrame: number | undefined;
    const nearby = new Set<number>();
    function readWidth(): void {
      if (view === undefined || list === undefined) return;
      const style = getComputedStyle(view);
      const available =
        view.clientWidth -
        Number.parseFloat(style.paddingLeft) -
        Number.parseFloat(style.paddingRight);
      if (available <= 0 || available === availableWidth) return;
      const source = highlight();
      const desired =
        availableWidth > 0
          ? readingPoint()
          : source === null
            ? props.progress
            : { page: source.page, offset: source.region.y };
      availableWidth = available;
      place(desired ?? { page: 1, offset: 0 });
    }
    const observer = new ResizeObserver(() => {
      if (resizeFrame !== undefined) return;
      resizeFrame = window.requestAnimationFrame(() => {
        resizeFrame = undefined;
        readWidth();
      });
    });
    const intersection = new IntersectionObserver(
      (entries) => {
        if (view.clientWidth === 0) return;
        for (const entry of entries) {
          if (!(entry.target instanceof HTMLElement)) continue;
          const page = Number(entry.target.dataset.page);
          if (entry.isIntersecting) nearby.add(page);
          else nearby.delete(page);
        }
        const centre = center()?.page ?? 1;
        setVisible(
          new Set(
            [...nearby]
              .sort((a, b) => Math.abs(a - centre) - Math.abs(b - centre))
              .slice(0, MAX_NEARBY),
          ),
        );
      },
      { root: view, rootMargin: "33% 0px" },
    );
    observer.observe(view);
    for (const page of list.children) intersection.observe(page);
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
      if (resizeFrame !== undefined) window.cancelAnimationFrame(resizeFrame);
      observer.disconnect();
      intersection.disconnect();
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
      const deferred = window.requestAnimationFrame(() => {
        place({ page: source.page, offset: source.region.y });
        recordScroll();
      });
      return () => {
        window.cancelAnimationFrame(deferred);
      };
    },
  );

  function measure(page: number, size: Size): void {
    const known = sizes();
    const prior = known.get(page);
    if (prior?.width === size.width && prior.height === size.height) return;
    const view = scroller;
    const list = pages;
    const before = readingPoint();
    setSizes(new Map(known).set(page, size));
    if (
      view !== undefined &&
      list !== undefined &&
      before !== null &&
      (page === 1 || before.page >= page)
    ) {
      flush();
      scroll(before);
    }
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
      details: () => ({ lang: props.lang, version: props.version, width: width(), pages: count() }),
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
      ref={(node) => {
        surface = node;
      }}
      role="group"
      aria-label={props.name}
    >
      <PdfToneFilter id={tone} />
      <PdfReaderControls
        lang={props.lang}
        layout={layout()}
        scale={scale()}
        first={first()}
        last={Math.min(first() + turns() - 1, count())}
        total={count()}
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
        onScroll={recordScroll}
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
          style={{ "--pdf-page-width": `${String(width() || 1)}px` }}
        >
          <For each={numbers()}>
            {(page) => {
              const size = createMemo(() => sizeOf(page));
              const active = createMemo(
                () =>
                  width() > 0 && (layout() === "continuous" ? visible().has(page) : shown(page)),
              );
              return (
                <div
                  class="pdf-sheet"
                  hidden={!shown(page)}
                  data-page={String(page)}
                  style={{ "aspect-ratio": `${String(size().width)} / ${String(size().height)}` }}
                  role="group"
                  aria-label={fill(say(props.lang, "format_page"), {
                    n: String(page),
                    total: String(count()),
                  })}
                >
                  <Show when={active()}>
                    <PdfPage
                      pdf={props.pdf}
                      page={page}
                      width={width()}
                      lang={props.lang}
                      measured={(size) => {
                        measure(page, size);
                      }}
                    />
                  </Show>
                  <Show when={boxOn(page)}>
                    {(region) => (
                      <div class="pdf-region" style={regionStyle(region())} aria-hidden="true" />
                    )}
                  </Show>
                  <Show when={highlight()?.page === page ? highlight()?.region : undefined}>
                    {(region) => (
                      <div
                        class="pdf-region pdf-reference"
                        style={regionStyle(region())}
                        aria-hidden="true"
                      />
                    )}
                  </Show>
                </div>
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
