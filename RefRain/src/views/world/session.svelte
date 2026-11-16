<script lang="ts">
  import type { Snippet } from "svelte";

  import { fill, say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { ui } from "../../ui";
  import type { Address, RoundsAnswer, RunId } from "../../wire";
  import Empty from "../parts/empty.svelte";
  import { sessionRun } from "./chosen.svelte";
  import Sheet from "./sheet.svelte";

  interface Props {
    readonly here: Address;

    readonly shown?: RunId | null | undefined;

    readonly title?: string | undefined;

    readonly head: Snippet;
  }

  const { here, shown, title, head }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;

  const run = $derived(shown === undefined ? sessionRun($belief, here) : shown);

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

<section
  class="flex min-h-0 flex-1 flex-col overflow-hidden"
  aria-label={say($lang, "world_session")}
>
  {@render head()}
  {#if title !== undefined}
    <h3 class="truncate text-heading font-heading text-text">{title}</h3>
  {/if}

  <div class="flex shrink-0 items-baseline justify-between gap-pane">
    <p class="flex min-w-0 items-baseline gap-snug text-note text-text-quiet">
      {#if worktree !== null}
        <span class="min-w-0 truncate"
          >{fill(say($lang, "world_worktree"), { name: worktree })}</span
        >
      {/if}
      {#if worktree !== null && base !== null}
        <span aria-hidden="true">·</span>
      {/if}
      {#if base !== null}
        <span class="shrink-0">{fill(say($lang, "world_based_on"), { oid: base.slice(0, 7) })}</span
        >
      {/if}
    </p>
    {#if run !== null}
      <a
        class="shrink-0 text-note text-accent hover:text-accent-hover"
        href={toFragment({ kind: "run", run })}>{say($lang, "world_open_run")}</a
      >
    {/if}
  </div>
  {#if run === null}
    <div class="mt-snug border-t border-edge">
      <Empty missing="world_no_session" seat="inset" />
    </div>
  {:else}
    <div class="mt-snug flex min-h-0 flex-1 flex-col">
      <Sheet {here} {run} {rounds} />
    </div>
  {/if}
</section>
