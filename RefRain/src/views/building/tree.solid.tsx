// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, For, Show } from "solid-js";

import { within } from "../../core/belief/live";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { kib } from "../../core/time";
import { ui } from "../../ui";
import type { Address, Entry, RunId } from "../../wire";
import { Address as AddressSchema } from "../../wire";

import { transcriptOf } from "./transcript";
import { treeRowWire, type TreeMark, type TreeRowLook } from "./tree_row";
import Row from "./tree_row.look.solid";

export interface Picked {
  readonly at: Address;
  readonly kind: "file" | "directory";
}

export interface BuildingTreeProps {
  readonly root: Address;
  readonly picked: Picked | null;
  readonly onPick: (picked: Picked) => void;
  readonly at?: Address | undefined;
  readonly depth?: number | undefined;
}

function rank(entry: Entry): number {
  return (entry.kind === "directory" ? 0 : 2) + (entry.name.startsWith(".") ? 1 : 0);
}

function Branch(props: BuildingTreeProps & { readonly at: Address }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const depth = () => props.depth ?? 0;
  const listing = storeValue(() => u.conn.asking.ask({ listing: { at: props.at } }));
  const entries = createMemo(() => {
    const answer = listing();
    if (answer === undefined || !("listing" in answer)) return undefined;
    return [...answer.listing.entries].sort(
      (left, right) => rank(left) - rank(right) || left.name.localeCompare(right.name),
    );
  });
  const [opened, setOpened] = createSignal<Readonly<Record<string, boolean>>>({});

  function press(entry: Entry, here: Address, transcript: RunId | null, open: boolean): void {
    if (transcript !== null) {
      u.go({ kind: "run", run: transcript });
      return;
    }
    const isDir = entry.kind === "directory";
    if (isDir) setOpened((old) => ({ ...old, [here]: !open }));
    props.onPick({ at: here, kind: isDir ? "directory" : "file" });
  }

  function rowOf(entry: Entry, here: Address, open: boolean): TreeRowLook {
    const transcript = transcriptOf(entry.name);
    const isDir = entry.kind === "directory";
    const mark: TreeMark = isDir
      ? { kind: "directory", open }
      : transcript === null
        ? { kind: "file" }
        : { kind: "transcript", hint: say(lang(), "tree_transcript") };
    const run = transcript === null ? undefined : belief().runs[transcript];
    const live = isDir
      ? belief().live.some((target) => within(target, here))
      : run !== undefined && run.doing.kind !== "frozen";
    return {
      name: transcript === null ? entry.name : (run?.task ?? transcript.slice(0, 8)),
      mark,
      tone: props.picked?.at === here ? "picked" : entry.name.startsWith(".") ? "hidden" : "plain",
      coded: transcript !== null,
      live: live ? say(lang(), "tree_live") : undefined,
      size: entry.kind === "directory" ? undefined : kib(entry.kind.file.bytes),
      wire: treeRowWire(mark, () => press(entry, here, transcript, open)),
    };
  }

  return (
    <ul class={depth() === 0 ? "" : "ml-base border-l border-edge pl-tight"}>
      <Show
        when={entries()}
        fallback={<li class="h-step pl-wide text-note leading-none text-text-faint">{"…"}</li>}
      >
        {(rows) => (
          <For
            each={rows()}
            keyed={(entry) => entry.name}
            fallback={
              <li class="h-step pl-wide text-note leading-none text-text-faint">
                {say(lang(), "tree_empty")}
              </li>
            }
          >
            {(entry) => {
              const here = createMemo(() => AddressSchema.make(`${props.at}/${entry().name}`));
              const open = () =>
                opened()[here()] ?? (depth() === 0 && !entry().name.startsWith("."));
              return (
                <li>
                  <Row {...rowOf(entry(), here(), open())} />
                  <Show when={entry().kind === "directory" && open()}>
                    <Branch
                      at={here()}
                      root={props.root}
                      picked={props.picked}
                      onPick={props.onPick}
                      depth={depth() + 1}
                    />
                  </Show>
                </li>
              );
            }}
          </For>
        )}
      </Show>
    </ul>
  );
}

export default function BuildingTree(props: BuildingTreeProps) {
  const lang = storeValue(ui().lang);
  return (
    <Show
      when={props.at}
      fallback={
        <section aria-label={say(lang(), "bld_tree")} class="text-note">
          <Branch
            at={props.root}
            root={props.root}
            picked={props.picked}
            onPick={props.onPick}
            depth={0}
          />
        </section>
      }
    >
      {(at) => <Branch {...props} at={at()} />}
    </Show>
  );
}
