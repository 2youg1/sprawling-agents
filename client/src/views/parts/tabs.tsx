// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Several readings of one subject, one at a time: the lenses of a run,
// of the record, of the cost page.
//
// One stop on the way in and the arrows once inside - the roving
// tabindex ARIA asks for - so a keyboard crossing the page does not have
// to step through every lens to get past them.

import { For, Show, createSignal, type JSX } from "solid-js";

export interface Lens {
  readonly id: string;
  // Already in the person's language.
  readonly label: string;
  // A count or a state, usually a Badge.
  readonly mark?: JSX.Element;
}

export interface TabsProps {
  // The accessible name of the set, already in the person's language.
  readonly label: string;
  readonly lenses: readonly Lens[];
  readonly current: string;
  readonly onPick: (id: string) => void;
}

export function Tabs(props: TabsProps) {
  const [tabs, setTabs] = createSignal<readonly HTMLButtonElement[]>([]);
  const at = () => props.lenses.findIndex((lens) => lens.id === props.current);
  const move = (to: number) => {
    const lens = props.lenses[to];
    if (lens === undefined) return;
    props.onPick(lens.id);
    tabs()[to]?.focus();
  };
  const hold = (index: number, tab: HTMLButtonElement) => {
    setTabs((before) => {
      const next = [...before];
      next[index] = tab;
      return next;
    });
  };

  return (
    <div class="flex items-center gap-tight border-b border-g2" role="tablist" aria-label={props.label}>
      <For each={props.lenses}>
        {(lens, index) => (
          <button
            ref={(tab) => {
              hold(index(), tab);
            }}
            type="button"
            role="tab"
            id={`tab-${lens.id}`}
            aria-selected={lens.id === props.current}
            tabindex={lens.id === props.current ? 0 : -1}
            class={`flex items-center gap-tight border-b-2 px-base py-snug text-label transition-[opacity,transform] duration-100 ease-[cubic-bezier(0.2,0,0,1)] motion-reduce:transition-none ${
              lens.id === props.current
                ? "border-accent text-text"
                : "border-transparent text-text-quiet hover:text-text"
            }`}
            onClick={() => {
              props.onPick(lens.id);
            }}
            onKeyDown={(event) => {
              const last = props.lenses.length - 1;
              switch (event.key) {
                case "ArrowRight":
                  event.preventDefault();
                  move(at() === last ? 0 : at() + 1);
                  return;
                case "ArrowLeft":
                  event.preventDefault();
                  move(at() <= 0 ? last : at() - 1);
                  return;
                case "Home":
                  event.preventDefault();
                  move(0);
                  return;
                case "End":
                  event.preventDefault();
                  move(last);
                  return;
                default:
                  return;
              }
            }}
          >
            {lens.label}
            <Show when={lens.mark}>{(mark) => mark()}</Show>
          </button>
        )}
      </For>
    </div>
  );
}
