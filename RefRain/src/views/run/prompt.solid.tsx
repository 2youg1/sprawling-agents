// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { For, Show, createMemo, createSignal } from "solid-js";
import { say } from "../../core/lang";
import { buildingOf } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { RunId } from "../../wire";
import Copy from "../parts/copy.solid";
import Glyph from "../parts/glyph.solid";
import Path from "../parts/path.solid";
import { lookOf, type PromptLook, type SegmentLook, type ToolsLook } from "./prompt";

const FOLD =
  "flex min-h-control-sm min-w-0 flex-1 items-center gap-base rounded-control px-snug text-left transition-colors still:transition-none ease-leave hover:wash hover:ease-arrive";
const TURN = "shrink-0 text-text-faint transition-transform still:transition-none";

function Fold(props: {
  readonly item: SegmentLook | ToolsLook;
  readonly note?: string | undefined;
}) {
  return (
    <button
      type="button"
      data-control="surface"
      aria-expanded={props.item.open ? "true" : "false"}
      onClick={() => props.item.fold.onclick()}
      class={FOLD}
    >
      <Glyph
        name="chevron"
        size="sm"
        class={`${TURN} ${props.item.open ? "rotate-90 ease-arrive" : "ease-leave"}`}
      />
      <span class="shrink-0 text-text-quiet">{props.item.slot}</span>
      <span class="figure shrink-0 text-text-faint">{props.item.size}</span>
      <Show when={props.note}>
        <span class="truncate text-alert">{props.note}</span>
      </Show>
    </button>
  );
}

type PaintCopy = (segment: SegmentLook, text: string) => JSX.Element;

function Segment(props: {
  readonly item: SegmentLook;
  readonly paintCopy?: PaintCopy | undefined;
}) {
  const lang = storeValue(ui().lang);
  return (
    <li class="border-b border-edge">
      <div class="flex items-center gap-base py-tight text-note">
        <Fold item={props.item} note={props.item.gone} />
        <Show when={props.item.copyText !== undefined}>
          {props.paintCopy?.(props.item, props.item.copyText ?? "") ?? (
            <Copy text={props.item.copyText ?? ""} note={say(lang(), "run_prompt_copy")} />
          )}
        </Show>
      </div>
      <Show when={props.item.open}>
        <div class="flex flex-col gap-snug pb-base pl-wide">
          <Show when={props.item.sources.length > 0}>
            <p class="flex flex-wrap items-baseline gap-snug text-note text-text-faint">
              <span>{props.item.sourcesLabel}</span>
              <For each={props.item.sources} keyed={(item) => item.key}>
                {(item) => (
                  <>
                    <Path path={item().path} onOpen={item().onOpen} />
                    <Show when={item().dropped}>
                      <span class="text-alert">{item().dropped}</span>
                    </Show>
                  </>
                )}
              </For>
            </p>
          </Show>
          <pre class="overflow-x-auto whitespace-pre-wrap break-words rounded-control border border-edge bg-page p-base font-mono text-note text-text-quiet">
            {props.item.text}
          </pre>
        </div>
      </Show>
    </li>
  );
}

function Tools(props: { readonly item: ToolsLook }) {
  return (
    <li class="border-b border-edge">
      <div class="flex items-center gap-base py-tight text-note">
        <Fold item={props.item} />
      </div>
      <Show when={props.item.open}>
        <ul class="flex flex-col pb-base pl-wide text-note">
          <For each={props.item.rows} keyed={(item) => item.key}>
            {(item) => (
              <li class="flex min-w-0 items-baseline gap-base">
                <span class="shrink-0 font-mono text-text-quiet">{item().name}</span>
                <span class="min-w-0 truncate text-text-faint">{item().facts}</span>
              </li>
            )}
          </For>
        </ul>
      </Show>
    </li>
  );
}

export function PromptContent(props: {
  readonly look: PromptLook;
  readonly paintCopy?: PaintCopy | undefined;
}) {
  return (
    <Show when={props.look.segments} fallback={<p class="text-text-faint">…</p>}>
      {(rows) => (
        <Show when={rows().length > 0} fallback={<p class="text-text-faint">{props.look.none}</p>}>
          <ul>
            <Show when={props.look.tools}>{(item) => <Tools item={item()} />}</Show>
            <For each={rows()} keyed={(item) => item.key}>
              {(item) => <Segment item={item()} paintCopy={props.paintCopy} />}
            </For>
          </ul>
        </Show>
      )}
    </Show>
  );
}

export default function Prompt(props: { readonly run: RunId }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const asked = storeValue(() => u.conn.asking.ask({ prefix: { run: props.run } }));
  const [open, setOpen] = createSignal<ReadonlySet<string>>(new Set());
  const sent = createMemo(() => {
    const held = asked();
    return held === undefined
      ? undefined
      : "prefix" in held
        ? held.prefix
        : { segments: [], tools: null };
  });
  const look = createMemo(() =>
    lookOf(sent(), { open: open() }, lang(), {
      toggle: (hash) => {
        setOpen((held) => {
          const next = new Set(held);
          if (!next.delete(hash)) next.add(hash);
          return next;
        });
      },
      visit: (source) => {
        u.go({ kind: "building", address: buildingOf(source) });
      },
    }),
  );
  return <PromptContent look={look()} />;
}
