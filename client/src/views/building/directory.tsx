// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A directory is one of three things, and the page says which. A
// **room** has held work, so it is read as the runs that worked there.
// The **governance** directory holds the rules a building runs under,
// so it is read as those documents. Anything else is a directory, read
// as its files. Treating all three as rooms is what put "no runs" under
// every folder in the tree.
//
// The kind is decided from the listing itself rather than from a name
// the city does not promise: a room is where a job was written down or
// a transcript was left.

import { For, Show, createMemo } from "solid-js";

import type { Doing } from "../../core/belief";
import { roomOf, toFragment } from "../../core/route";
import { clock, kib, usd } from "../../core/time";
import type { Address, Entry } from "../../wire";
import { Address as AddressSchema } from "../../wire";
import { useLang, useSay, useUi } from "../../ui";
import { EmptyState } from "../parts/empty";
import { Path } from "../parts/path";
import type { Picked } from "./tree";

// The directory a building keeps its own rules in.
const GOVERNANCE = ".sprawling";

type Kind = "room" | "governance" | "ordinary";

export interface DirectoryProps {
  readonly at: Address;
  readonly root: Address;
  readonly onPick: (picked: Picked) => void;
}

function kindOf(at: Address, entries: readonly Entry[]): Kind {
  if (roomOf(at) === GOVERNANCE) return "governance";
  const held = entries.some((entry) => entry.name === "JOB.md" || entry.name.endsWith(".jsonl"));
  return held ? "room" : "ordinary";
}

function posture(say: ReturnType<typeof useSay>, doing: Doing): string {
  switch (doing.kind) {
    case "thinking":
      return say("run_doing_thinking");
    case "calling":
      return say("run_doing_calling");
    case "waiting":
      return say("talk_waiting_you");
    case "frozen":
      switch (doing.completion) {
        case "done":
          return say("outcome_done");
        case "limit":
          return say("outcome_limit");
        case "cancelled":
          return say("outcome_cancelled");
        case null:
        default:
          return say("outcome_frozen");
      }
  }
}

function Files(props: DirectoryProps & { readonly entries: readonly Entry[] }) {
  return (
    <ul class="text-note">
      <For each={props.entries}>
        {(entry) => {
          const here = () => AddressSchema.make(`${props.at}/${entry.name}`);
          return (
            <li class="flex items-center gap-base border-b border-g1 py-snug">
              <Path
                path={here()}
                base={props.root}
                onOpen={() => {
                  props.onPick({ at: here(), kind: entry.kind === "directory" ? "directory" : "file" });
                }}
              />
              <span class="flex-1" />
              <Show when={entry.kind !== "directory" ? entry.kind : undefined}>
                {(kind) => <span class="shrink-0 font-mono text-text-disabled">{kib(kind().file.bytes)}</span>}
              </Show>
            </li>
          );
        }}
      </For>
    </ul>
  );
}

function Runs(props: { readonly at: Address }) {
  const ui = useUi();
  const say = useSay();
  const lang = useLang();
  const cost = ui.conn.asking.ask("cost_view");
  const spent = (run: string) => {
    const held = cost();
    if (held === undefined || !("cost" in held)) return null;
    return held.cost.by_run.find(([name]) => name === run)?.[1] ?? null;
  };
  const runs = createMemo(() =>
    Object.values(ui.conn.belief.runs)
      .filter((run) => run.addr === props.at)
      .sort((a, b) => (b.started ?? 0) - (a.started ?? 0)),
  );
  return (
    <Show
      when={runs().length > 0}
      fallback={
        <EmptyState
          text={say("dir_room_empty")}
          action={
            <a
              href={toFragment({ kind: "talk", address: props.at })}
              class="rounded-control bg-g2 px-base py-tight text-label hover:bg-g3"
            >
              {say("bld_open_talk")}
            </a>
          }
        />
      }
    >
      <ul class="text-note">
        <For each={runs()}>
          {(run) => (
            <li class="border-b border-g1 py-snug">
              <a href={toFragment({ kind: "run", run: run.run })} class="flex items-center gap-base hover:text-text">
                <span class={`inline-block size-dot shrink-0 rounded-pill ${run.doing.kind === "frozen" ? "bg-g4" : "bg-accent"}`} />
                <span class="min-w-0 flex-1 truncate text-text-quiet">{run.task ?? run.run}</span>
                <span class="shrink-0 text-text-faint">{posture(say, run.doing)}</span>
                <Show when={spent(run.run)}>{(usdMicros) => <span class="shrink-0 text-text-disabled">{usd(usdMicros())}</span>}</Show>
                <Show when={run.started}>{(at) => <span class="shrink-0 text-text-disabled">{clock(lang(), at())}</span>}</Show>
              </a>
            </li>
          )}
        </For>
      </ul>
    </Show>
  );
}

export function Directory(props: DirectoryProps) {
  const ui = useUi();
  const say = useSay();
  const listing = createMemo(() => ui.conn.asking.ask({ listing: { at: props.at } }));
  const entries = createMemo(() => {
    const held = listing()();
    return held !== undefined && "listing" in held ? held.listing.entries : undefined;
  });

  return (
    <Show when={entries()} fallback={<p class="text-text-disabled">…</p>}>
      {(held) => {
        const kind = () => kindOf(props.at, held());
        return (
          <div>
            <div class="mb-base flex flex-wrap items-baseline gap-base">
              <h2 class="text-heading font-heading">{roomOf(props.at)}</h2>
              <span class="text-note text-text-faint">{say(`dir_${kind()}`)}</span>
              <span class="flex-1" />
              <Show when={kind() === "room"}>
                <a
                  href={toFragment({ kind: "talk", address: props.at })}
                  class="rounded-control bg-g2 px-base py-tight text-label hover:bg-g3"
                >
                  {say("bld_open_talk")}
                </a>
              </Show>
            </div>
            <Show when={kind() === "governance"}>
              <p class="mb-base text-note text-text-quiet">{say("dir_governance_what")}</p>
            </Show>
            <Show when={kind() === "room"} fallback={
              <Show when={held().length > 0} fallback={<EmptyState text={say("dir_empty")} />}>
                <Files at={props.at} root={props.root} onPick={props.onPick} entries={held()} />
              </Show>
            }>
              <Runs at={props.at} />
            </Show>
          </div>
        );
      }}
    </Show>
  );
}
