// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createSignal, Match, Show, Switch } from "solid-js";
import { EDITABLE_BYTES_MAX } from "../../core/document_windows";
import { fill, say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import { kib } from "../../core/time";
import { drawnAs, markdownFile } from "./formats/format";
import { framed } from "./formats/framed";
import { drawMath, formulaOf } from "./formats/math";
import { drawMermaid } from "./formats/mermaid";
import { diagramUrl, imageType } from "./markdown_widgets";

export type WorkspaceFormat =
  | { readonly kind: "image"; readonly mime: string }
  | { readonly kind: "docx" | "html" | "markdown" | "math" | "mermaid" | "text" };

export function workspaceFormat(name: string): WorkspaceFormat {
  const mime = name.toLowerCase().endsWith(".svg") ? "image/svg+xml" : imageType(name);
  if (mime !== null) return { kind: "image", mime };
  const known = drawnAs(name);
  if (known === "docx" || known === "html") return { kind: known };
  const lower = name.toLowerCase();
  if (markdownFile(name)) return { kind: "markdown" };
  if (/\.(?:tex|latex|math)$/u.test(lower)) return { kind: "math" };
  if (/\.(?:mmd|mermaid)$/u.test(lower)) return { kind: "mermaid" };
  return { kind: "text" };
}

interface PreviewProps {
  readonly lang: Lang;
  readonly name: string;
  readonly text: string;
  readonly bytes: Uint8Array;
  readonly format: WorkspaceFormat;
}

export function WorkspaceDocumentSurface(
  props: Omit<PreviewProps, "text"> & {
    readonly editable: boolean;
    readonly preview: string | null;
    readonly mountEditor: (node: HTMLDivElement) => void;
  },
) {
  const binaryPreview = () => props.format.kind === "docx" || props.format.kind === "image";
  return (
    <>
      <Show when={props.editable}>
        <Show when={props.bytes.length > EDITABLE_BYTES_MAX}>
          <p role="status">
            {fill(say(props.lang, "refrain_too_large"), {
              total: kib(props.bytes.length),
              through: kib(Math.min(props.bytes.length, EDITABLE_BYTES_MAX)),
            })}
          </p>
        </Show>
        <div
          class="workspace-editor-host"
          ref={props.mountEditor}
          hidden={props.preview !== null}
          data-refrain-reading-canvas
        />
      </Show>
      <Show when={binaryPreview() || props.preview !== null}>
        <WorkspacePreview
          lang={props.lang}
          name={props.name}
          text={props.preview ?? ""}
          bytes={props.bytes}
          format={props.format}
        />
      </Show>
      <Show when={!props.editable && !binaryPreview()}>
        <p role="status">{say(props.lang, "refrain_format_opaque")}</p>
      </Show>
    </>
  );
}

function WorkspacePreview(props: PreviewProps) {
  const [url, setUrl] = createSignal<string>();
  const [page, setPage] = createSignal<string>();
  const [failure, setFailure] = createSignal(false);
  let formulaBody: HTMLDivElement | undefined;

  createEffect(
    () => ({ format: props.format, text: props.text, bytes: props.bytes }),
    ({ format, text, bytes }) => {
      setFailure(false);
      setPage(undefined);
      setUrl(undefined);
      let active = true;
      let objectUrl: string | undefined;
      if (format.kind === "image") {
        objectUrl = URL.createObjectURL(new Blob([bytes.slice()], { type: format.mime }));
        setUrl(objectUrl);
      } else if (format.kind === "html") {
        setPage(framed(text, "authored"));
      } else if (format.kind === "docx") {
        void import("./formats/docx")
          .then(({ readDocx }) => readDocx(bytes))
          .then(
            (result) => {
              if (!active) return;
              if (result.kind === "drawn") setPage(framed(result.page, "built"));
              else setFailure(true);
            },
            () => {
              if (active) setFailure(true);
            },
          );
      } else if (format.kind === "mermaid") {
        void drawMermaid(text).then((svg) => {
          if (!active) return;
          if (svg === null) {
            setFailure(true);
            return;
          }
          objectUrl = diagramUrl(svg);
          setUrl(objectUrl);
        });
      } else if (format.kind === "math" && formulaBody !== undefined) {
        const wrap = formulaOf(text.trim());
        const host = formulaBody;
        host.replaceChildren();
        void drawMath(wrap?.tex ?? text, host, "display", () => active).then(
          (drawn) => {
            if (active) setFailure(!drawn);
          },
          () => {
            if (active) setFailure(true);
          },
        );
      }
      return () => {
        active = false;
        if (objectUrl !== undefined) URL.revokeObjectURL(objectUrl);
      };
    },
  );

  return (
    <div class="workspace-format-preview" data-refrain-reading-canvas>
      <Switch>
        <Match when={props.format.kind === "html" || props.format.kind === "docx"}>
          <Show when={page()}>
            <iframe title={props.name} sandbox="" referrerpolicy="no-referrer" srcdoc={page()} />
          </Show>
        </Match>
        <Match when={props.format.kind === "image" || props.format.kind === "mermaid"}>
          <Show when={url()}>
            <img alt={props.name} src={url()} onError={() => setFailure(true)} />
          </Show>
        </Match>
        <Match when={props.format.kind === "math"}>
          <div
            class="workspace-math-preview"
            ref={(node) => {
              formulaBody = node;
            }}
          />
        </Match>
      </Switch>
      <Show when={failure()}>
        <p role="status">{say(props.lang, "refrain_preview_failed")}</p>
        <Show when={props.text !== ""}>
          <pre>{props.text}</pre>
        </Show>
      </Show>
    </div>
  );
}
