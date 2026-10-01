<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The world layer's commits pane (client-SPEC 7K): the city's commits
  // across every session, newest first, each a short oid, its message and
  // the room that made it. Commits made in the room the conversation is
  // in are drawn at full strength and the rest a step quieter, so the
  // chosen session's work stands out of everyone's.
  //
  // It asks the first page of the city-wide commits question in the one
  // spelling `asking.ts` gives it. The lanes, the B3 and the files a
  // chosen commit opens into are drawn when the wire carries them (7K,
  // current state); each row today links to the run that made it.
  import { readAnswer } from "../../core/answered";
  import { commitsQuery } from "../../core/asking";
  import { say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import Unanswered from "../parts/unanswered.svelte";

  interface Props {
    readonly here: Address;
  }

  const { here }: Props = $props();

  const u = ui();
  const { lang } = u;
  const commits = u.conn.asking.ask(commitsQuery(null, null));
  const read = $derived(readAnswer($commits, (held) => ("commits" in held ? held.commits : undefined)));

  // An oid as git prints it short.
  const SHORT = 7;
</script>

<section class="flex min-h-0 flex-col overflow-hidden" aria-label={say($lang, "world_commits")}>
  <h2 class="flex h-control shrink-0 items-center text-note text-text-faint">{say($lang, "world_commits")}</h2>
  <div class="min-h-0 flex-1 overflow-y-auto">
    {#if read.kind === "unavailable"}
      <Unanswered query={read.query} asked={commitsQuery(null, null)} />
    {:else if read.kind === "held"}
      <ul>
        {#each read.value.commits as commit (commit.oid)}
          <li>
            <a
              href={toFragment({ kind: "run", run: commit.run })}
              class={[
                "-mx-snug grid h-bar grid-cols-[12px_8ch_minmax(0,1fr)] items-center gap-x-base rounded-card px-snug text-note hover:wash",
                commit.actor === here ? "text-text" : "text-text-faint",
              ]}
            >
              <span
                class={["size-dot rounded-pill", commit.actor === here ? "bg-accent" : "bg-mark"]}
                aria-hidden="true"
              ></span>
              <span class="figure text-text-quiet">{commit.oid.slice(0, SHORT)}</span>
              <span class="truncate">
                {commit.message ?? ""}
                <span class="ml-snug text-text-faint">{commit.actor}</span>
              </span>
            </a>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</section>
