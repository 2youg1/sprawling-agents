<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The tree itself, one directory per question, opened the way a person
  // opens folders. A room with a run still going carries a lit dot; a
  // transcript is named for the run that wrote it and opens that run. A
  // row shows a name and nothing else until a hand is on it, when the
  // size appears.
  //
  // One instance draws one directory level and hands each open folder
  // the next instance, so every level owns one question (`Query::
  // Listing`) and one store subscription. The transcript reading goes
  // through `core/run_id` (client/Spec.lean §3-2): this file holds no grammar
  // of its own, and a name the generated `RunId` refuses is a name like
  // any other.
  import { within } from "../../core/belief/live";
  import { say } from "../../core/lang";
  import { kib } from "../../core/time";
  import { ui } from "../../ui";
  import type { Address, Entry, RunId } from "../../wire";
  import { Address as AddressSchema } from "../../wire";
  // The next level down. A self-import rather than `<svelte:self>`,
  // which the compiler marks deprecated in favour of exactly this.
  import Branch from "./tree.svelte";
  import { transcriptOf } from "./transcript";
  import { treeRowWire, type TreeMark, type TreeRowLook } from "./tree_row";
  import Row from "./tree_row.look.svelte";

  // What the tree hands back when a row is picked. Stated beside the
  // component that hands it over; the neighbours that read it take it
  // across the file boundary as a type.
  export interface Picked {
    readonly at: Address;
    readonly kind: "file" | "directory";
  }

  interface Props {
    readonly root: Address;
    readonly picked: Picked | null;
    readonly onPick: (picked: Picked) => void;
    // Where this instance reads, and how deep it stands. A caller
    // outside this file passes neither: that is the top of the tree,
    // which draws the region around the root level. The recursion below
    // passes both. A region rather than a `nav`: the tree is what the
    // building holds, read level by level, not a way between pages, and
    // its rows step in by depth where a navigation column's rows share
    // one left edge (client/Spec.lean §4-50).
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
    // Folders first, hidden last within each, the way a person's eye
    // reads a directory: rooms and plans before the machinery.
    const rank = (entry: Entry) =>
      (entry.kind === "directory" ? 0 : 2) + (entry.name.startsWith(".") ? 1 : 0);
    return [...answer.listing.entries].sort(
      (left, right) => rank(left) - rank(right) || left.name.localeCompare(right.name),
    );
  });

  // Which rows stand open. One row's openness is its own, and it lives
  // here rather than in the row: an `{#each}` iteration is not a
  // component, and this level dies with its directory either way.
  const opened = $state<Record<string, boolean>>({});

  function join(at: Address, name: string): Address {
    return AddressSchema.make(`${at}/${name}`);
  }

  // What the row is called: a transcript carries the task the run was
  // given, falling back to the id's head when nobody named one.
  function nameOf(name: string, transcript: RunId | null): string {
    if (transcript === null) return name;
    const run = $belief.runs[transcript];
    return run?.task ?? transcript.slice(0, 8);
  }

  // One row as its look draws it: the mark before the name, how loudly
  // the name is drawn, the lit dot and the size a hand reveals.
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

  // A transcript opens the run that wrote it; a folder turns over and
  // is picked; a file is picked.
  function press(entry: Entry, here: Address, transcript: RunId | null, open: boolean): void {
    if (transcript !== null) {
      u.go({ kind: "run", run: transcript });
      return;
    }
    const isDir = entry.kind === "directory";
    if (isDir) opened[here] = !open;
    onPick({ at: here, kind: isDir ? "directory" : "file" });
  }

  // Whether something is working under this row: a directory holds work
  // when a run lives at or below it, a transcript when its run has not
  // frozen.
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
