// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The tree itself, one directory per question, opened the way a person
// opens folders. A room with a run still going carries a lit dot; a
// transcript is named for the run that wrote it and opens that run.

import { For, Show, createMemo, createSignal, untrack } from "solid-js";

import { Option } from "effect";

import { RunId } from "../../core/run_id";
import { kib } from "../../core/time";
import type { Address, Entry } from "../../wire";
import { Address as AddressSchema } from "../../wire";
import { useSay, useUi } from "../../ui";

export interface Picked {
  readonly at: Address;
  readonly kind: "file" | "directory";
}

export interface TreeProps {
  readonly root: Address;
  readonly picked: Picked | null;
  readonly onPick: (picked: Picked) => void;
}

function join(at: Address, name: string): Address {
  return AddressSchema.make(`${at}/${name}`);
}

function Node(props: {
  readonly at: Address;
  readonly entry: Entry;
  readonly depth: number;
  readonly picked: Picked | null;
  readonly onPick: (picked: Picked) => void;
  readonly openAtStart: boolean;
}) {
  const ui = useUi();
  const say = useSay();
  const [open, setOpen] = createSignal(untrack(() => props.openAtStart));
  const here = () => join(props.at, props.entry.name);
  const isDir = () => props.entry.kind === "directory";
  const chosen = () => props.picked?.at === here();
  const transcriptRun = createMemo(() => {
    const stem = props.entry.name.replace(/\.jsonl$/, "");
    if (stem === props.entry.name) return null;
    return Option.getOrNull(RunId.option(stem));
  });
  const live = createMemo(() =>
    isDir()
      ? Object.values(ui.conn.belief.runs).some(
          (run) => run.doing.kind !== "frozen" && run.addr !== null && (run.addr === here() || run.addr.startsWith(`${here()}/`)),
        )
      : false,
  );
  const pad = () => `${String(props.depth * 14 + 8)}px`;

  return (
    <li>
      <button
        type="button"
        class={`flex w-full items-center gap-snug rounded-control py-tight pr-snug text-left text-note hover:bg-g1 ${chosen() ? "bg-g2 text-text" : "text-text-quiet"}`}
        style={{ "padding-left": pad() }}
        onClick={() => {
          if (isDir()) setOpen((held) => !held);
          props.onPick({ at: here(), kind: isDir() ? "directory" : "file" });
        }}
        aria-expanded={isDir() ? open() : undefined}
      >
        <span class="inline-block w-base text-text-disabled">{isDir() ? (open() ? "▾" : "▸") : ""}</span>
        <span class={`truncate ${props.entry.name.startsWith(".") ? "text-text-faint" : ""}`}>
          {props.entry.name}
        </span>
        <Show when={live()}>
          <span class="inline-block size-dot rounded-pill bg-accent" title={say("tree_live")} />
        </Show>
        <span class="flex-1" />
        <Show when={props.entry.kind !== "directory" ? props.entry.kind : undefined}>
          {(kind) => <span class="text-text-disabled">{kib(kind().file.bytes)}</span>}
        </Show>
        <Show when={transcriptRun()}>
          {(run) => (
              <a
                href={`#/run/${run()}`}
                class="rounded-pill bg-g2 px-snug text-text-quiet hover:bg-g3"
                onClick={(event) => {
                  event.stopPropagation();
                }}
              >
                {say("tree_transcript")}
              </a>
          )}
        </Show>
      </button>
      <Show when={isDir() && open()}>
        <Branch at={here()} depth={props.depth + 1} picked={props.picked} onPick={props.onPick} />
      </Show>
    </li>
  );
}

function Branch(props: {
  readonly at: Address;
  readonly depth: number;
  readonly picked: Picked | null;
  readonly onPick: (picked: Picked) => void;
}) {
  const ui = useUi();
  const say = useSay();
  const listing = createMemo(() => ui.conn.asking.ask({ listing: { at: props.at } }));
  const entries = createMemo(() => {
    const answer = listing()();
    return answer !== undefined && "listing" in answer ? answer.listing.entries : undefined;
  });
  return (
    <ul>
      <Show when={entries()} fallback={<li class="py-tight pl-wide text-note text-text-disabled">…</li>}>
        {(held) => (
          <Show when={held().length > 0} fallback={<li class="py-tight pl-wide text-note text-text-disabled">{say("tree_empty")}</li>}>
            <For each={held()}>
              {(entry) => (
                <Node
                  at={props.at}
                  entry={entry}
                  depth={props.depth}
                  picked={props.picked}
                  onPick={props.onPick}
                  openAtStart={props.depth === 0 && entry.kind === "directory" && !entry.name.startsWith(".")}
                />
              )}
            </For>
          </Show>
        )}
      </Show>
    </ul>
  );
}

export function Tree(props: TreeProps) {
  const say = useSay();
  return (
    <nav aria-label={say("bld_tree")} class="text-note">
      <Branch at={props.root} depth={0} picked={props.picked} onPick={props.onPick} />
    </nav>
  );
}
