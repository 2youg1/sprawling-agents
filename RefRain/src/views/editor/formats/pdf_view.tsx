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
  Match,
  onSettled,
  Show,
  Switch,
} from "solid-js";

import { fill, say } from "../../../core/lang";
import type { Lang } from "../../../core/lang";
import { scopedUi } from "../../../ui";
import { attachContextTools, ContextTools, createContextTools } from "../../parts/context_tools";
import Glyph from "../../parts/glyph.solid";
import { NAME } from "./format";
import type { Opened, Size } from "./pdf";
import { pointAt, readableRegion, regionStyle } from "./pdf_anchor";
import type { PdfProgress, PdfReference, PdfRegion, PdfSelection, Point } from "./pdf_anchor";
import { PdfPage } from "./pdf_page_view";
import { PdfToneFilter } from "./pdf_palette";
import { pdfPicking, type Picked } from "./pdf_pick";
import { positionAt, scrollToPosition } from "./pdf_position";
import { pdfActions, PdfZoom } from "./pdf_tools";
import "./pdf.css";

export interface PdfProps {
  readonly bytes: Uint8Array;
  readonly version: string | null;
  readonly name: string;
  readonly lang: Lang;
  readonly progress?: PdfProgress | null | undefined;
  readonly onProgress?: ((progress: PdfProgress) => void) | undefined;
  readonly reference?: PdfReference | null | undefined;
  readonly onSelection?: ((attachment: PdfSelection) => void) | undefined;
}

interface Graphics {
  readonly webgpu: boolean;
  readonly webgl2: boolean;
}

export function PdfView(props: PdfProps) {
  const [opened, setOpened] = createSignal<Opened | null>(null);
  const [graphics, setGraphics] = createSignal<Graphics>();
  const bytes = createMemo(() => props.bytes);
  const pdf = createMemo(() => {
    const value = opened();
    return value?.kind === "open" ? value.pdf : null;
  });
  const broken = createMemo(() => {
    const value = opened();
    return value?.kind === "broken" ? value.reason : undefined;
  });

  createEffect(bytes, (from) => {
    setOpened(null);
    let gone = false;
    let close: (() => void) | undefined;
    void import("./pdf")
      .then(({ openPdf, gpuAvailability }) => {
        if (gone) return;
        const opening = openPdf(from);
        close = opening.close;
        void opening.opened.then((result) => {
          if (!gone) setOpened(result);
        });
        void gpuAvailability().then((available) => {
          if (!gone) setGraphics(available);
        });
      })
      .catch((reason: unknown) => {
        if (!gone) setOpened({ kind: "broken", reason: String(reason) });
      });
    return () => {
      gone = true;
      close?.();
    };
  });

  return (
    <Switch>
      <Match when={pdf()} keyed>
        {(document) => <PdfReading {...props} pdf={document} graphics={graphics()} />}
      </Match>
      <Match when={opened()?.kind === "locked"}>
        <p class="px-wide py-snug text-note text-text-quiet">{say(props.lang, "format_locked")}</p>
      </Match>
      <Match when={broken()}>
        {(reason) => (
          <p class="px-wide py-snug text-note text-text-quiet">
            {fill(say(props.lang, "format_broken"), { format: NAME.pdf, reason: reason() })}
          </p>
        )}
      </Match>
      <Match when={opened() === null}>
        <p class="px-wide py-snug text-note text-text-faint">
          {fill(say(props.lang, "format_reading"), { format: NAME.pdf })}
        </p>
      </Match>
    </Switch>
  );
}

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

function PdfReading(props: ReadingProps) {
  const tone = createUniqueId();
  const [sizes, setSizes] = createSignal<ReadonlyMap<number, Size>>(new Map());
  const [visible, setVisible] = createSignal<ReadonlySet<number>>(new Set());
  const [width, setWidth] = createSignal(0);
  const [original, setOriginal] = createSignal(false);
  const [zoom, setZoom] = createSignal<Zoomed>();
  let scroller: HTMLDivElement | undefined;
  let pages: HTMLDivElement | undefined;
  let surface: HTMLDivElement | undefined;
  const menus = createContextTools();
  const canChangeBrightness = scopedUi() === undefined;
  let frame = 0;
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

  function center(): PdfProgress | null {
    return scroller === undefined || pages === undefined
      ? null
      : positionAt(scroller, pages, scroller.clientHeight / 2);
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
    if (recorded !== undefined) props.onProgress?.(recorded);
  }

  function recordScroll(): void {
    if (frame || props.onProgress === undefined) return;
    frame = window.requestAnimationFrame(() => {
      frame = 0;
      if (scroller === undefined || pages === undefined) return;
      const position = positionAt(scroller, pages);
      if (position === null) return;
      nextProgress = position;
      clearTimeout(saveTimer);
      saveTimer = setTimeout(saveProgress, 250);
    });
  }

  onSettled(() => {
    const view = scroller;
    const list = pages;
    const host = surface;
    if (view === undefined || list === undefined || host === undefined) return;
    let restored = false;
    let resizeFrame: number | undefined;
    const nearby = new Set<number>();
    function readWidth(): void {
      if (view === undefined || list === undefined) return;
      const available = list.clientWidth;
      if (available === 0 || (restored && width() === available)) return;
      const before = restored ? positionAt(view, list) : (props.progress ?? null);
      const source = highlight();
      const initial = !restored;
      restored = true;
      setWidth(available);
      flush();
      const desired =
        initial && source !== null ? { page: source.page, offset: source.region.y } : before;
      if (desired !== null) scrollToPosition(view, list, desired);
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
    readWidth();
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
      window.cancelAnimationFrame(frame);
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
        if (scroller !== undefined && pages !== undefined) {
          scrollToPosition(scroller, pages, { page: source.page, offset: source.region.y });
        }
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
    const before = view === undefined || list === undefined ? null : positionAt(view, list);
    setSizes(new Map(known).set(page, size));
    if (
      view !== undefined &&
      list !== undefined &&
      before !== null &&
      (page === 1 || before.page >= page)
    ) {
      flush();
      scrollToPosition(view, list, before);
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
              if (scroller !== undefined && pages !== undefined)
                scrollToPosition(scroller, pages, { page: source.page, offset: source.region.y });
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
          ref={(node) => {
            pages = node;
          }}
          style={{ "--pdf-page-width": `${String(width() || 1)}px` }}
        >
          <For each={numbers()}>
            {(page) => {
              const size = createMemo(() => sizeOf(page));
              const active = createMemo(() => width() > 0 && visible().has(page));
              return (
                <div
                  class="pdf-sheet"
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
