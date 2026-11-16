<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import { within } from "../../core/belief/live";
  import { say } from "../../core/lang";
  import { kib } from "../../core/time";
  import { ui } from "../../ui";
  import type { Address, Entry, RunId } from "../../wire";
  import { Address as AddressSchema } from "../../wire";

  import Branch from "./tree.svelte";
  import { transcriptOf } from "./transcript";
  import { treeRowWire, type TreeMark, type TreeRowLook } from "./tree_row";
  import Row from "./tree_row.look.svelte";

  export interface Picked {
    readonly at: Address;
    readonly kind: "file" | "directory";
  }

  interface Props {
    readonly root: Address;
    readonly picked: Picked | null;
    readonly onPick: (picked: Picked) => void;

    readonly at?: Address | undefined;
    readonly depth?: number | undefined;
  }

  const { root, picked, onPick, at, depth }: Props = $props();

  const u = ui();
  const lang = u.lang;
  const belief = u.conn.belief;

  const dir = $derived(at ?? root);
  const nesting = $derived(depth ?? 0);

  const listing = $derived(u.conn.asking.ask({ listing: { at: dir } }));
  const entries = $derived.by(() => {
    const answer = $listing;
    if (answer === undefined || !("listing" in answer)) return undefined;

    const rank = (entry: Entry) =>
      (entry.kind === "directory" ? 0 : 2) + (entry.name.startsWith(".") ? 1 : 0);
    return [...answer.listing.entries].sort(
      (left, right) => rank(left) - rank(right) || left.name.localeCompare(right.name),
    );
  });

  const opened = $state<Record<string, boolean>>({});

  function join(at: Address, name: string): Address {
    return AddressSchema.make(`${at}/${name}`);
  }

  function nameOf(name: string, transcript: RunId | null): string {
    if (transcript === null) return name;
    const run = $belief.runs[transcript];
    return run?.task ?? transcript.slice(0, 8);
  }

  function rowOf(entry: Entry, here: Address, open: boolean): TreeRowLook {
    const transcript = transcriptOf(entry.name);
    const mark: TreeMark =
      entry.kind === "directory"
        ? { kind: "directory", open }
        : transcript === null
          ? { kind: "file" }
          : { kind: "transcript", hint: say($lang, "tree_transcript") };
    return {
      name: nameOf(entry.name, transcript),
      mark,
      tone: picked?.at === here ? "picked" : entry.name.startsWith(".") ? "hidden" : "plain",
      coded: transcript !== null,
      live: lit(entry.kind === "directory", here, transcript) ? say($lang, "tree_live") : undefined,
      size: entry.kind === "directory" ? undefined : kib(entry.kind.file.bytes),
      wire: treeRowWire(mark, () => {
        press(entry, here, transcript, open);
      }),
    };
  }

  function press(entry: Entry, here: Address, transcript: RunId | null, open: boolean): void {
    if (transcript !== null) {
      u.go({ kind: "run", run: transcript });
      return;
    }
    const isDir = entry.kind === "directory";
    if (isDir) opened[here] = !open;
    onPick({ at: here, kind: isDir ? "directory" : "file" });
  }

  function lit(isDir: boolean, here: Address, transcript: RunId | null): boolean {
    if (isDir) {
      return $belief.live.some((run) => within(run, here));
    }
    if (transcript === null) return false;
    const run = $belief.runs[transcript];
    return run !== undefined && run.doing.kind !== "frozen";
  }
</script>

{#if at === undefined}
  <section aria-label={say($lang, "bld_tree")} class="text-note">
    <Branch {root} {picked} {onPick} at={root} depth={0} />
  </section>
{:else}
  <ul class={nesting === 0 ? "" : "ml-base border-l border-edge pl-tight"}>
    {#if entries === undefined}
      <li class="h-step pl-wide text-note leading-none text-text-faint">…</li>
    {:else if entries.length === 0}
      <li class="h-step pl-wide text-note leading-none text-text-faint">
        {say($lang, "tree_empty")}
      </li>
    {:else}
      {#each entries as entry (entry.name)}
        {@const here = join(dir, entry.name)}
        {@const isDir = entry.kind === "directory"}
        {@const hidden = entry.name.startsWith(".")}
        {@const openNow = opened[here] ?? (nesting === 0 && !hidden)}
        <li>
          <Row {...rowOf(entry, here, openNow)} />
          {#if isDir && openNow}
            <Branch {root} {picked} {onPick} at={here} depth={nesting + 1} />
          {/if}
        </li>
      {/each}
    {/if}
  </ul>
{/if}
