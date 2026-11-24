// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, onCleanup, Show } from "solid-js";
import Glyph from "../parts/glyph.solid";
import type { StripLook, TabLook } from "./strip";
import "./strip.solid.css";

function Tab(props: { readonly tab: TabLook }) {
  const wire = () => props.tab.wire;
  const close = () => props.tab.close;
  return (
    <div
      class={[
        "tab group/tab relative flex shrink-0 items-center gap-snug border-r border-edge pr-snug first:pl-wide",
        props.tab.front ? "front bg-page text-text" : "text-text-faint hover:text-text-quiet",
      ]}
    >
      <a
        role={wire().role}
        aria-selected={wire()["aria-selected"] ? "true" : "false"}
        aria-controls={wire()["aria-controls"]}
        href={wire().href}
        tabindex={wire().tabindex}
        onClick={(event) => wire().onclick(event)}
        onKeyDown={(event) => wire().onkeydown(event)}
        ref={(node) => {
          const cleanup = props.tab.hold(node);
          if (cleanup) onCleanup(cleanup);
        }}
        class="name flex h-full items-center gap-snug pl-pane whitespace-nowrap group-first/tab:pl-0"
      >
        <Show when={props.tab.terminal}>
          <Glyph name="terminal" size="sm" />
        </Show>
        <span class="truncate">{props.tab.label}</span>
        <Show when={props.tab.unsaved}>
          {(message) => (
            <>
              <span class="size-dot shrink-0 rounded-pill bg-alert" aria-hidden="true" />
              <span class="sr-only">{message()}</span>
            </>
          )}
        </Show>
      </a>
      <span class="flex size-glyph-sm shrink-0">
        <button
          data-control=""
          type={close().type}
          tabindex={close().tabindex}
          aria-label={close()["aria-label"]}
          onClick={() => close().onclick()}
          class={[
            "mark relative size-glyph-sm items-center justify-center rounded-control text-text-faint before:absolute before:-inset-tight before:content-[''] hover:bg-raised hover:text-text",
            props.tab.front ? "flex" : "hidden group-focus-within/tab:flex group-hover/tab:flex",
          ]}
        >
          <Glyph name="cross" size="sm" />
        </button>
      </span>
    </div>
  );
}

export default function StripLookView(look: StripLook) {
  return (
    <div class="strip inspect-tabs-strip flex shrink-0 items-stretch border-b border-edge pr-snug text-note">
      <div {...look.list} class="flex min-w-0 flex-1 items-stretch overflow-x-auto">
        <For each={look.tabs} keyed={(tab) => tab.key}>
          {(tab) => <Tab tab={tab()} />}
        </For>
      </div>
      <button
        data-control=""
        type={look.closeAll.type}
        aria-label={look.closeAll["aria-label"]}
        onClick={() => look.closeAll.onclick()}
        class="key my-auto ml-snug flex size-control-sm items-center justify-center rounded-control text-text-faint hover:bg-raised hover:text-text"
      >
        <Glyph name="cross" />
      </button>
    </div>
  );
}
