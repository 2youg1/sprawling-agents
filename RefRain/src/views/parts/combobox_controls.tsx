// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Show } from "solid-js";
import Glyph from "./glyph.solid";

import type { ComboboxLook, OptionLook } from "./combobox";
import "./combobox.css";

export interface BoxRefs {
  readonly root: (node: HTMLDivElement) => void;
  readonly trigger: (node: HTMLButtonElement) => void;
  readonly layer: (node: HTMLDivElement) => void;
  readonly search: (node: HTMLInputElement) => void;
  readonly list: (node: HTMLUListElement) => void;
}

function OptionRow(props: { readonly option: OptionLook }) {
  return (
    <li
      class={[
        "relative flex cursor-pointer items-baseline justify-between gap-snug rounded-control px-base py-snug text-body",
        props.option.chosen
          ? "refrain-combobox-applied applied wash-strong text-text"
          : props.option.cursor
            ? "bg-raised-hover text-text"
            : "text-text-quiet",
        props.option.cursor && "refrain-combobox-cursor cursor",
      ]}
      id={props.option.wire.id}
      role={props.option.wire.role}
      aria-selected={props.option.wire["aria-selected"] ? "true" : "false"}
      onMouseEnter={() => props.option.wire.onmouseenter()}
      onClick={() => props.option.wire.onclick()}
    >
      <span class="truncate" style={{ "font-family": props.option.face }}>
        {props.option.label}
      </span>
      <Show when={props.option.note !== undefined}>
        <span class="shrink-0 text-note text-text-faint">{props.option.note}</span>
      </Show>
    </li>
  );
}

export default function ComboboxControls(props: ComboboxLook & { readonly holds: BoxRefs }) {
  return (
    <div
      class="relative flex w-full min-w-0 flex-col gap-tight"
      ref={props.holds.root}
      onFocusOut={(event) => props.root.onfocusout(event)}
    >
      <button
        class="flex h-control w-full min-w-0 items-center justify-between gap-snug rounded-control border border-edge-input bg-raised px-base text-body text-text hover:bg-raised-hover"
        data-control=""
        ref={props.holds.trigger}
        type={props.trigger.type}
        aria-label={props.trigger["aria-label"]}
        aria-haspopup="listbox"
        aria-expanded={props.open ? "true" : "false"}
        aria-controls={props.open ? props.search["aria-controls"] : undefined}
        onClick={() => props.trigger.onclick()}
        onKeyDown={(event) => props.trigger.onkeydown(event)}
      >
        <span
          class={["truncate", props.unset && "text-text-faint"]}
          style={{ "font-family": props.faceStyle }}
        >
          {props.face}
        </span>
        <Glyph name="chevron" size="sm" class={["shrink-0", props.open && "rotate-90"]} />
      </button>
      <Show when={props.layer !== undefined}>
        <div
          ref={props.holds.layer}
          class={[
            "refrain-combobox-layer layer absolute left-0 z-(--layer-popover) w-full flex-col rounded-panel border border-edge-panel bg-raised p-tight shadow-float",
            props.side === "below" ? "top-full mt-tight" : "bottom-full mb-tight",
            props.open && "open",
          ]}
          data-side={props.side}
          inert={props.layer?.inert}
        >
          <input
            class="mb-tight h-control w-full rounded-control bg-raised px-base text-body text-text placeholder:text-text-faint"
            ref={props.holds.search}
            role={props.search.role}
            aria-label={props.search["aria-label"]}
            aria-expanded={props.search["aria-expanded"] ? "true" : "false"}
            aria-controls={props.search["aria-controls"]}
            aria-activedescendant={props.search["aria-activedescendant"]}
            placeholder={props.search.placeholder}
            value={props.search.value}
            onInput={(event) => props.search.oninput(event)}
            onKeyDown={(event) => props.search.onkeydown(event)}
          />
          <ul
            class="max-h-output overflow-y-auto"
            ref={props.holds.list}
            id={props.list.id}
            role={props.list.role}
            aria-label={props.list["aria-label"]}
            onMouseDown={(event) => props.list.onmousedown(event)}
          >
            <For
              each={props.options}
              keyed={(option) => option.key}
              fallback={
                <li role="presentation" class="px-base py-snug text-note text-text-faint">
                  {props.empty}
                </li>
              }
            >
              {(option) => <OptionRow option={option()} />}
            </For>
          </ul>
        </div>
      </Show>
    </div>
  );
}
