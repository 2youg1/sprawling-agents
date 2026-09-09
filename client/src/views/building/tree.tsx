// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The tree itself, one directory per question, opened the way a person
// opens folders. A room with a run still going carries a lit dot; a
// transcript is named for the run that wrote it and opens that run. A
// row shows a name and nothing else until a hand is on it, when the
// size appears.

import { For, Show, createMemo, createSignal, untrack } from "solid-js";

import { Option } from "effect";

import { RunId } from "../../core/run_id";
import { kib } from "../../core/time";
import type { Address, Entry } from "../../wire";
import { Address as AddressSchema } from "../../wire";
import { useGo, useSay, useUi } from "../../ui";

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

function Chevron(props: { readonly open: boolean }) {
  return (
    <svg
      viewBox="0 0 10 10"
      class={`size-dot shrink-0 text-text-disabled transition-transform ${props.open ? "rotate-90" : ""}`}
      fill="none"
      stroke="currentColor"
      stroke-width="1.4"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <path d="M3.5 2l3 3-3 3" />
    </svg>
  );
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
  const go = useGo();
  const [open, setOpen] = createSignal(untrack(() => props.openAtStart));
  const here = () => join(props.at, props.entry.name);
  const isDir = () => props.entry.kind === "directory";
  const chosen = () => props.picked?.at === here();
  const hidden = () => props.entry.name.startsWith(".");
  const transcriptRun = createMemo(() => {
    const stem = props.entry.name.replace(/\.jsonl$/, "");
    if (stem === props.entry.name) return null;
    return Option.getOrNull(RunId.option(stem));
  });
  const run = createMemo(() => {
    const id = transcriptRun();
    return id === null ? undefined : ui.conn.belief.runs[id];
  });
  const live = createMemo(() =>
    isDir()
      ? Object.values(ui.conn.belief.runs).some(
          (r) => r.doing.kind !== "frozen" && r.addr !== null && (r.addr === here() || r.addr.startsWith(`${here()}/`)),
        )
      : run()?.doing.kind !== undefined && run()?.doing.kind !== "frozen",
  );
  const shown = () => {
    const id = transcriptRun();
    return id === null ? props.entry.name : (run()?.task ?? id.slice(0, 8));
  };
  const pick = () => {
    const id = transcriptRun();
    if (id !== null) {
      go({ kind: "run", run: id });
      return;
    }
    if (isDir()) setOpen((held) => !held);
    props.onPick({ at: here(), kind: isDir() ? "directory" : "file" });
  };

  return (
    <li>
      <button
        type="button"
        class={`group/row flex h-step w-full items-center gap-tight rounded-control pl-tight pr-snug text-left text-note leading-none hover:bg-g1 ${
          chosen() ? "bg-g2 text-text" : hidden() ? "text-text-disabled" : "text-text-quiet"
        }`}
        onClick={pick}
        aria-expanded={isDir() ? open() : undefined}
        title={transcriptRun() === null ? undefined : say("tree_transcript")}
      >
        <span class="flex w-base shrink-0 justify-center">
          <Show when={isDir()} fallback={<Show when={transcriptRun()}><span class="text-text-disabled">↗</span></Show>}>
            <Chevron open={open()} />
          </Show>
        </span>
        <span class={`truncate ${transcriptRun() === null ? "" : "font-mono text-text-faint"}`}>{shown()}</span>
        <Show when={live()}>
          <span class="ml-tight inline-block size-dot shrink-0 rounded-pill bg-accent" title={say("tree_live")} />
        </Show>
        <span class="flex-1" />
        <Show when={props.entry.kind !== "directory" ? props.entry.kind : undefined}>
          {(kind) => (
            <span class="hidden shrink-0 whitespace-nowrap font-mono text-text-disabled group-hover/row:inline">
              {kib(kind().file.bytes)}
            </span>
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
    if (answer === undefined || !("listing" in answer)) return undefined;
    // Folders first, hidden last within each, the way a person's eye
    // reads a directory: rooms and plans before the machinery.
    const rank = (entry: Entry) => (entry.kind === "directory" ? 0 : 2) + (entry.name.startsWith(".") ? 1 : 0);
    return [...answer.listing.entries].sort((a, b) => rank(a) - rank(b) || a.name.localeCompare(b.name));
  });
  return (
    <ul class={props.depth === 0 ? "" : "ml-base border-l border-g2 pl-tight"}>
      <Show when={entries()} fallback={<li class="h-step pl-wide text-note leading-none text-text-disabled">…</li>}>
        {(held) => (
          <Show when={held().length > 0} fallback={<li class="h-step pl-wide text-note leading-none text-text-disabled">{say("tree_empty")}</li>}>
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
