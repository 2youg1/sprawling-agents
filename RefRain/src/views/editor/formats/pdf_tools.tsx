// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { PDFDocumentProxy } from "pdfjs-dist";
import { onCleanup, onSettled, Show } from "solid-js";

import { fill, say, type Key } from "../../../core/lang";
import type { Lang } from "../../../core/lang";
import { storeValue } from "../../../core/solid_store";
import { BRIGHTNESS, wallpaperSettings } from "../../../core/wallpaper";
import Button from "../../parts/button.solid";
import type { ContextTool } from "../../parts/context_tools";
import Glyph from "../../parts/glyph.solid";
import { short } from "../reading";
import { NAME, TOOL } from "./format";
import type { PdfDetail, Size } from "./pdf";
import { readableRegion, type Point } from "./pdf_anchor";
import type { pdfPicking, Picked } from "./pdf_pick";
import { PdfPage } from "./pdf_page_view";
import { PDF_LAYOUTS, PDF_SCALE, type PdfLayout } from "./pdf_position";

interface DetailsProps {
  readonly lang: Lang;
  readonly version: string | null;
  readonly width: number;
  readonly pages: number;
}

function PdfDetails(props: DetailsProps) {
  const version = () => props.version;
  return (
    <div class="pdf-metadata">
      <p>
        {NAME.pdf} · {TOOL.pdf}
      </p>
      <p>{say(props.lang, "format_pdf_settings")}</p>
      <p>
        {props.pages === 1
          ? say(props.lang, "pdf_one_page")
          : fill(say(props.lang, "format_pages"), { n: String(props.pages) })}{" "}
        · {fill(say(props.lang, "format_width"), { width: String(props.width) })}
      </p>
      <Show when={version() !== null}>
        <p>{fill(say(props.lang, "format_version"), { version: short(version() ?? "") })}</p>
      </Show>
    </div>
  );
}

function PdfBrightness(props: { readonly lang: Lang }) {
  const settings = wallpaperSettings();
  const state = storeValue(settings);
  let previewed = false;
  onCleanup(() => {
    if (previewed && state().dirty) settings.revert();
  });
  return (
    <div class="pdf-brightness">
      <input
        aria-label={say(props.lang, "wallpaper_brightness")}
        aria-valuetext={`${String(state().look.choice.brightness)}%`}
        type="range"
        min={BRIGHTNESS.min}
        max={BRIGHTNESS.max}
        step={BRIGHTNESS.step}
        value={state().look.choice.brightness}
        disabled={state().busy}
        onInput={(event) => {
          previewed = true;
          settings.brightness(event.currentTarget.valueAsNumber);
        }}
      />
      <button
        class="pdf-control"
        type="button"
        data-control="surface"
        aria-busy={state().busy ? "true" : undefined}
        data-control-state={
          state().busy
            ? "pending"
            : state().problem === "write"
              ? "refused"
              : state().saved
                ? "success"
                : undefined
        }
        disabled={state().busy || !state().dirty}
        onClick={() => {
          void settings.apply();
        }}
      >
        <Glyph name={state().problem === "write" ? "cross" : "check"} />
        {say(props.lang, "wallpaper_apply")}
      </button>
      <Show when={state().saved}>
        <p role="status">{say(props.lang, "setup_saved")}</p>
      </Show>
      <Show when={state().problem === "write"}>
        <p role="status">{say(props.lang, "wallpaper_write_failed")}</p>
      </Show>
    </div>
  );
}

interface Actions {
  readonly lang: Lang;
  readonly selected: Picked | undefined;
  readonly selecting:
    | Pick<ReturnType<typeof pdfPicking>, "attaching" | "attach" | "selecting" | "chooseRegion">
    | undefined;
  readonly brightness: boolean;
  readonly magnify: () => void;
  readonly original: (held: boolean) => void;
  readonly reference: (() => void) | undefined;
  readonly details: () => DetailsProps;
}

export function pdfActions(actions: Actions): readonly ContextTool[] {
  const tools: ContextTool[] = [];
  const { selected, selecting, lang } = actions;
  if (selecting !== undefined) {
    if (selected !== undefined && readableRegion(selected.region))
      tools.push({
        id: "attach",
        glyph: "pin",
        label: say(lang, "pdf_attach"),
        disabled: selecting.attaching,
        run: () => {
          void selecting.attach(selected);
        },
      });
    tools.push({
      id: "region",
      glyph: "pdf-crop",
      label: say(lang, "pdf_select_region"),
      checked: selecting.selecting,
      run: selecting.chooseRegion,
    });
  }
  tools.push(
    { id: "magnify", glyph: "pdf-magnify", label: say(lang, "pdf_zoom"), run: actions.magnify },
    {
      id: "original",
      glyph: "pdf-original",
      label: say(lang, "pdf_original"),
      hold: actions.original,
    },
  );
  if (actions.brightness)
    tools.push({
      id: "brightness",
      glyph: "pdf-brightness",
      label: say(lang, "wallpaper_brightness"),
      panel: () => <PdfBrightness lang={lang} />,
    });
  if (actions.reference !== undefined)
    tools.push({
      id: "reference",
      glyph: "pdf-reference",
      label: say(lang, "pdf_reference"),
      run: actions.reference,
    });
  tools.push({
    id: "details",
    glyph: "pdf-details",
    label: say(lang, "pdf_details"),
    panel: () => <PdfDetails {...actions.details()} />,
  });
  return tools;
}

interface ZoomProps {
  readonly pdf: PDFDocumentProxy;
  readonly detailDocument: PdfDetail;
  readonly page: number;
  readonly width: number;
  readonly size: Size;
  readonly anchor: Point;
  readonly lang: Lang;
  readonly close: () => void;
}

const ZOOM = 2;

export function PdfZoom(props: ZoomProps) {
  let dialog: HTMLDialogElement | undefined;
  let scroller: HTMLDivElement | undefined;
  let drag: { pointer: number; x: number; y: number; left: number; top: number } | null = null;

  onSettled(() => {
    const target = dialog;
    const area = scroller;
    if (target === undefined || area === undefined) return;
    target.showModal();
    const leave = (event: KeyboardEvent): void => {
      if (event.key !== "Escape" || event.isComposing) return;
      event.preventDefault();
      event.stopPropagation();
      target.close();
    };
    target.addEventListener("keydown", leave);
    const frame = window.requestAnimationFrame(() => {
      area.scrollLeft = area.scrollWidth * props.anchor.x - area.clientWidth / 2;
      area.scrollTop = area.scrollHeight * props.anchor.y - area.clientHeight / 2;
    });
    return () => {
      window.cancelAnimationFrame(frame);
      target.removeEventListener("keydown", leave);
      target.close();
    };
  });

  function begin(event: PointerEvent & { currentTarget: HTMLDivElement }): void {
    if (event.button !== 0 || event.pointerType === "touch") return;
    const node = event.currentTarget;
    drag = {
      pointer: event.pointerId,
      x: event.clientX,
      y: event.clientY,
      left: node.scrollLeft,
      top: node.scrollTop,
    };
    node.setPointerCapture(event.pointerId);
  }

  function pan(event: PointerEvent & { currentTarget: HTMLDivElement }): void {
    if (drag?.pointer !== event.pointerId) return;
    event.currentTarget.scrollLeft = drag.left + drag.x - event.clientX;
    event.currentTarget.scrollTop = drag.top + drag.y - event.clientY;
  }

  return (
    <dialog
      class="pdf-zoom"
      ref={(node) => {
        dialog = node;
      }}
      aria-label={say(props.lang, "pdf_zoom")}
      onClose={() => {
        props.close();
      }}
    >
      <div
        class="pdf-zoom-scroll"
        ref={(node) => {
          scroller = node;
        }}
        role="region"
        aria-label={say(props.lang, "pdf_zoom_pan")}
        tabindex="0"
        onPointerDown={begin}
        onPointerMove={pan}
        onLostPointerCapture={() => {
          drag = null;
        }}
      >
        <div
          class="pdf-sheet"
          style={{
            width: `${String(props.width * ZOOM)}px`,
            "aspect-ratio": `${String(props.size.width)} / ${String(props.size.height)}`,
          }}
        >
          <PdfPage
            pdf={props.pdf}
            detailDocument={props.detailDocument}
            page={props.page}
            width={props.width * ZOOM}
            lang={props.lang}
            selectable={false}
          />
        </div>
      </div>
      <button
        class="pdf-control pdf-zoom-back"
        data-control="surface"
        type="button"
        onClick={() => dialog?.close()}
      >
        <Glyph name="go-back" />
        {say(props.lang, "pdf_close_zoom")}
      </button>
    </dialog>
  );
}

const LAYOUT_WORDS: Readonly<Record<PdfLayout, Key>> = {
  continuous: "pdf_layout_strip",
  spread: "pdf_layout_book",
  single: "pdf_layout_single",
};

export function PdfReaderControls(props: {
  readonly lang: Lang;
  readonly layout: PdfLayout;
  readonly scale: number;
  readonly first: number;
  readonly last: number;
  readonly total: number;
  readonly night: boolean;
  readonly onNight: () => void;
  readonly onLayout: (layout: PdfLayout) => void;
  readonly onScale: (scale: number) => void;
  readonly onTurn: (direction: number) => void;
}) {
  return (
    <div class="pdf-reader-controls">
      <label>
        {say(props.lang, "pdf_layout")}
        <select
          class="pdf-control"
          data-control="surface"
          onChange={(event) => {
            const next = PDF_LAYOUTS.find((layout) => layout === event.currentTarget.value);
            if (next !== undefined) props.onLayout(next);
          }}
        >
          {PDF_LAYOUTS.map((layout) => (
            <option value={layout} selected={layout === props.layout}>
              {say(props.lang, LAYOUT_WORDS[layout])}
            </option>
          ))}
        </select>
      </label>
      <label>
        <span>{say(props.lang, "pdf_reader_scale")}</span>
        <input
          type="range"
          min={PDF_SCALE.min}
          max={PDF_SCALE.max}
          step={PDF_SCALE.step}
          value={props.scale}
          aria-valuetext={`${String(props.scale)}%`}
          onInput={(event) => props.onScale(event.currentTarget.valueAsNumber)}
        />
        <span class="pdf-reader-scale" aria-hidden="true">
          {props.scale}%
        </span>
      </label>
      <Show when={props.layout !== "continuous"}>
        <div class="pdf-reader-turn">
          <button
            type="button"
            class="pdf-control"
            data-control="surface"
            aria-label={say(props.lang, "pdf_previous_page")}
            disabled={props.first <= 1}
            onClick={() => props.onTurn(-1)}
          >
            <Glyph name="go-back" size="sm" />
          </button>
          <span aria-live="polite">
            {fill(say(props.lang, "format_page"), {
              n:
                props.first === props.last
                  ? String(props.first)
                  : `${String(props.first)}–${String(props.last)}`,
              total: String(props.total),
            })}
          </span>
          <button
            type="button"
            class="pdf-control pdf-next-page"
            data-control="surface"
            aria-label={say(props.lang, "pdf_next_page")}
            disabled={props.last >= props.total}
            onClick={() => props.onTurn(1)}
          >
            <Glyph name="go-back" size="sm" />
          </button>
        </div>
      </Show>
      <div class="pdf-reader-night">
        <Button
          label={say(props.lang, "pdf_night")}
          glyph="moon"
          pressed={props.night}
          onPress={props.onNight}
        />
      </div>
    </div>
  );
}
