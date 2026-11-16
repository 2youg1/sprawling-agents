<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // One row of a building's commit list (client/Spec.lean §4-50): the commit
  // as one line, a disclosure that opens its sheet of facts and the
  // files it changed against the commit before it, and on each of those
  // files the way to take it back to this checkpoint (UC8).
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Address, CommitAnswer, GitOid } from "../../wire";
  import Changes from "../changes.svelte";
  import CommitFacts from "./commit_facts.svelte";
  import CommitLine from "./commit_line.svelte";
  import { disclosureWire } from "./disclosure";
  import Disclosure from "./disclosure.look.svelte";
  import TakeBack from "./take_back.svelte";

  interface Props {
    readonly building: Address;
    readonly commit: CommitAnswer;
    // The commit this one is diffed against: the next row down, or
    // `null` while that row's page has not arrived.
    readonly older: CommitAnswer | null;
    // Whether this is the building's first commit, with nothing older.
    readonly first: boolean;
    readonly open: boolean;
    readonly onToggle: () => void;
    readonly holds: (oid: GitOid) => boolean;
    readonly onOpen: (oid: GitOid) => void;
  }

  const { building, commit, older, first, open, onToggle, holds, onOpen }: Props = $props();

  const lang = ui().lang;
</script>

{#snippet takeBack(path: string)}
  <TakeBack {building} {path} point={commit.oid} />
{/snippet}

{#snippet line()}
  <CommitLine {commit} />
{/snippet}

<li class="border-b border-edge" id="commit-{commit.oid}">
  <Disclosure layout="commit" {open} wire={disclosureWire(open, onToggle)} cells={line} />
  {#if open}
    <div class="flex min-w-0 flex-col gap-base pt-snug pb-wide pl-wide narrow:pl-0">
      <CommitFacts {commit} {holds} {onOpen} />
      {#if older !== null}
        <Changes base={older.oid} head={commit.oid} talk={building} act={takeBack} />
      {:else}
        <p class="text-note text-text-faint">{first ? say($lang, "commits_first") : "…"}</p>
      {/if}
    </div>
  {/if}
</li>
