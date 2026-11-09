<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The panorama gauge as a strip in the talk column, outside the
  // conversation landmark (client D113): the same instrument sheet the
  // chosen-session pane used to own above a second grid row, now above
  // the scroller so the talk box never moves. Asks the same rounds
  // question the thread asks; `asking` merges them by content. The
  // run-page link lives here because desktop panorama has no separate
  // session pane; standing outside the conversation landmark keeps the
  // render talk gate from treating it as a floating bar.
  import { fill, say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { ui } from "../../ui";
  import type { Address, RoundsAnswer, RunId } from "../../wire";
  import Sheet from "../world/sheet.svelte";

  interface Props {
    readonly here: Address;
    readonly run: RunId | null;
  }

  const { here, run }: Props = $props();

  const u = ui();
  const { lang } = u;

  let rounds = $state<RoundsAnswer | undefined>(undefined);
  $effect(() => {
    const at = run;
    rounds = undefined;
    if (at === null) return;
    return u.conn.asking.ask({ rounds: { run: at } }).subscribe((held) => {
      rounds = held !== undefined && "rounds" in held ? held.rounds : undefined;
    });
  });

  const worktree = $derived(rounds?.worktree ?? null);
  const base = $derived(rounds?.opened_at ?? null);
</script>

{#if run !== null}
  <div class="shrink-0" aria-label={say($lang, "world_session")}>
    <div class="mb-snug flex items-baseline justify-between gap-pane">
      <p class="flex min-w-0 items-baseline gap-snug text-note text-text-quiet">
        {#if worktree !== null}
          <span class="min-w-0 truncate"
            >{fill(say($lang, "world_worktree"), { name: worktree })}</span
          >
        {/if}
        {#if worktree !== null && base !== null}
          <!-- wording-ok: a separator glyph between two facts, hidden from screen readers -->
          <span aria-hidden="true">·</span>
        {/if}
        {#if base !== null}
          <span class="shrink-0"
            >{fill(say($lang, "world_based_on"), { oid: base.slice(0, 7) })}</span
          >
        {/if}
      </p>
      <a
        class="shrink-0 text-note text-accent hover:text-accent-hover"
        href={toFragment({ kind: "run", run })}>{say($lang, "world_open_run")}</a
      >
    </div>
    <Sheet {here} {run} {rounds} />
  </div>
{/if}
