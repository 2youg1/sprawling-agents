// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { For, Show, createEffect, createMemo, createSignal, createUniqueId } from "solid-js";
import {
  landing,
  panelId,
  panelStop,
  REACHABLE,
  tabId,
  tabStop,
  type Lens,
} from "./tab_navigation";
import "./tabs.css";

export interface NativeTabsProps {
  readonly label: string;
  readonly lenses: readonly Lens[];
  readonly current: string;
  readonly onPick: (id: string) => void;
  readonly panel: (lens: Lens) => JSX.Element;
  readonly mark?: ((lens: Lens) => JSX.Element) | undefined;
}

export default function Tabs(props: NativeTabsProps) {
  const uid = createUniqueId();
  const chosen = createMemo(() => props.lenses.findIndex((lens) => lens.id === props.current));
  const [reachable, setReachable] = createSignal(true);
  createEffect(
    () => props.current,
    (id) => {
      const host = document.getElementById(panelId(uid, id));
      if (host === null) return;
      let frame = 0;
      const measure = (): void => {
        frame = 0;
        setReachable(host.querySelector(REACHABLE) !== null);
      };
      const observer = new MutationObserver(() => {
        if (frame === 0) frame = requestAnimationFrame(measure);
      });
      observer.observe(host, {
        childList: true,
        subtree: true,
        attributes: true,
        attributeFilter: ["disabled", "href", "tabindex", "contenteditable", "type"],
      });
      frame = requestAnimationFrame(measure);
      return () => {
        observer.disconnect();
        cancelAnimationFrame(frame);
      };
    },
  );
  function travel(event: KeyboardEvent): void {
    const at = landing(props.lenses.length, chosen(), event.key);
    const lens = at === undefined ? undefined : props.lenses[at];
    if (lens === undefined) return;
    event.preventDefault();
    props.onPick(lens.id);
    document.getElementById(tabId(uid, lens.id))?.focus({ preventScroll: true });
  }
  return (
    <>
      <div
        role="tablist"
        aria-label={props.label}
        class="flex flex-wrap items-center gap-tight border-b border-edge"
      >
        <For each={props.lenses} keyed={(item) => item.id}>
          {(item, index) => (
            <button
              type="button"
              data-control="surface"
              class="refrain-tab flex items-center gap-tight px-base py-snug text-label"
              id={tabId(uid, item().id)}
              role="tab"
              aria-controls={panelId(uid, item().id)}
              aria-selected={item().id === props.current ? "true" : "false"}
              tabindex={tabStop(index(), chosen())}
              onClick={() => {
                props.onPick(item().id);
              }}
              onKeyDown={travel}
            >
              {item().label}
              {props.mark?.(item())}
            </button>
          )}
        </For>
      </div>
      <For each={props.lenses} keyed={(item) => item.id}>
        {(item) => (
          <div
            role="tabpanel"
            id={panelId(uid, item().id)}
            aria-labelledby={tabId(uid, item().id)}
            hidden={item().id !== props.current}
            tabindex={panelStop(item().id === props.current, reachable())}
          >
            <Show when={item().id === props.current}>{props.panel(item())}</Show>
          </div>
        )}
      </For>
    </>
  );
}
