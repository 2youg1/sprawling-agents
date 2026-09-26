<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import { fill, say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { clock } from "../../core/time";
  import type { RunBelief } from "../../core/belief";
  import { ui } from "../../ui";
  import { motherName } from "./forking";
  import type { Boundary, ForkPlan } from "./forking";
  import Thread from "./thread.svelte";

  interface Props {
    // Everything before the boundary, oldest first. Folded to one line
    // and opened in place: this is the same conversation, earlier -
    // neither a dialog nor a route is the right weight for a scroll
    // (ux B7).
    readonly earlier: readonly RunBelief[];
    readonly who: string;
    readonly boundary: Boundary | null;
    readonly onFork: (plan: ForkPlan) => void;
    readonly onRetry: (task: string) => void;
  }

  const { earlier, who, boundary, onFork, onRetry }: Props = $props();

  const { lang, conn } = ui();
  const belief = conn.belief;

  let open = $state(false);

  // The count is the folded runs themselves: what expands is one thread
  // per run, so the number a person reads is the number of things the
  // line is holding down.
  const holding = $derived(fill(say($lang, "session_previous"), { n: String(earlier.length) }));

  const line = $derived.by((): string | null => {
    if (boundary === null) return null;
    if (boundary.kind === "forked") {
      // The mother is looked up in the whole city, not only among the
      // runs folded above: a branch cut from another room's run, or from
      // one outside this stretch, still names whom it came from.
      const task = $belief.runs[boundary.mother]?.task ?? null;
      const named = fill(say($lang, "session_forked_divider"), {
        turn: String(boundary.turn),
        at: (task === null ? null : motherName(task)) ?? say($lang, "fork_mother"),
      });
      return boundary.at === null ? named : `${named} · ${clock($lang, boundary.at)}`;
    }
    // A boundary this page watched open knows its minute; one found
    // here after a reload says only that the stretch is new, which is
    // the part that is true in every case.
    return boundary.at === null ? null : fill(say($lang, "session_new_divider"), { at: clock($lang, boundary.at) });
  });

  const href = $derived.by((): string | null => {
    if (boundary?.kind !== "forked") return null;
    return toFragment({ kind: "run", run: boundary.mother });
  });
</script>

{#if earlier.length > 0}
  {#if open}
    <div class="fade">
      {#each earlier as run (run.run)}
        <Thread {run} {who} {onFork} {onRetry} />
      {/each}
    </div>
  {/if}
  <div class="my-snug flex items-center gap-base text-note text-text-faint">
    <span class="h-px flex-1 bg-raised"></span>
    <button
      type="button"
      class="rounded-control px-tight hover:bg-chrome hover:text-text-quiet"
      aria-expanded={open}
      onclick={() => {
        open = !open;
      }}
    >
      {holding} · {open ? say($lang, "session_collapse") : say($lang, "session_expand")}
    </button>
    <span class="h-px flex-1 bg-raised"></span>
  </div>
{/if}
{#if line !== null}
  <div class="mt-tight flex items-center gap-base text-note text-text-faint">
    <span class="h-px flex-1 bg-raised"></span>
    {#if href !== null}
      <a {href} class="hover:text-text-quiet">{line}</a>
    {:else}
      <span>{line}</span>
    {/if}
    <span class="h-px flex-1 bg-raised"></span>
  </div>
{/if}
