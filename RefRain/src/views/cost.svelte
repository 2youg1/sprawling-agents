<script lang="ts" module>
  
  
  import type { Key } from "../core/lang";
  import type { Renewals, UnpricedCalls, UsdMicros } from "../wire";

  type Cut = "by_run" | "by_actor" | "by_segment" | "by_tool" | "by_skill";
  const CUTS: readonly Cut[] = ["by_run", "by_actor", "by_segment", "by_tool", "by_skill"];

  
  const TITLES: Record<Cut, Key> = {
    by_run: "cost_by_run",
    by_actor: "cost_by_actor",
    by_segment: "cost_by_segment",
    by_tool: "cost_by_tool",
    by_skill: "cost_by_skill",
  };
</script>

<script lang="ts">
  import { QUERIES } from "../core/asking";
  import { readAnswer } from "../core/answered";
  import { fill, say } from "../core/lang";
  import { count, usd } from "../core/time";
  import { ui } from "../ui";
  import AskMayor from "./city/ask_mayor.svelte";
  import EmptyState from "./parts/empty.svelte";
  import Page from "./parts/page.svelte";
  import Unanswered from "./parts/unanswered.svelte";
  import { costReading, renewalsSent } from "./pricing";

  interface Props {
    
    
    readonly rank?: "page" | "section" | undefined;
  }

  const { rank = "page" }: Props = $props();

  const u = ui();
  const lang = u.lang;
  const asked = u.conn.asking.ask(QUERIES.cost);

  const read = $derived(readAnswer($asked, (held) => ("cost" in held ? held.cost : undefined)));
  const answer = $derived(read.kind === "held" ? read.value : undefined);
  const reading = $derived(answer === undefined ? undefined : costReading(answer));
</script>

{#snippet unpriced(unpaid: UnpricedCalls)}
  <p class="text-note text-text-quiet">
    {fill(say($lang, "cost_unpriced"), {
      calls: count(unpaid.calls),
      tokens: count(unpaid.tokens),
    })}
  </p>
{/snippet}

{#snippet renewed(renewals: Renewals)}
  <p class="text-note text-text-quiet">
    {fill(say($lang, "cost_renewals"), {
      spent: usd(renewals.billed),
      sent: count(renewalsSent(renewals)),
      late: count(renewals.late),
    })}
  </p>
{/snippet}

{#snippet cut(rows: readonly (readonly [string, UsdMicros])[], total: UsdMicros)}
  <ul class="text-note">
    {#each [...rows].sort((a, b) => b[1] - a[1]) as [name, amount] (name)}
      <li class="settled-row py-tight">
        <div class="flex justify-between gap-base">
          <span class="truncate font-mono text-text-quiet">{name}</span>
          <span class="shrink-0 figure text-text">{usd(amount)}</span>
        </div>
        <div class="summary mt-tight h-hair overflow-hidden rounded-pill bg-track">
          <div
            class="h-full bg-accent"
            style:width="{total > 0 ? (amount / total) * 100 : 0}%"
          ></div>
        </div>
      </li>
    {/each}
  </ul>
{/snippet}

{#snippet total()}
  
  {#if answer !== undefined && reading !== undefined && reading.kind !== "idle"}
    <span class="flex items-baseline gap-snug">
      <span class="text-note text-text-faint">{say($lang, "cost_total")}</span>
      <span class="text-figure font-figure figure">
        {reading.kind === "priced" ? usd(answer.total) : say($lang, "cost_none")}
      </span>
    </span>
  {/if}
{/snippet}


<Page title={say($lang, "cost_title")} {rank} aside={total}>
  {#if read.kind === "unavailable"}
    <Unanswered query={read.query} asked={QUERIES.cost} />
  {:else if answer === undefined || reading === undefined}
    <p class="text-text-faint">…</p>
  {:else if reading.kind === "unpriced"}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
    {@render unpriced(answer.unpriced)}
  {:else if reading.kind === "idle"}
    
    <EmptyState missing="cost_empty" seat="region">
      {#snippet action()}
        <AskMayor />
      {/snippet}
    </EmptyState>
  {:else}
    {#if answer.unpriced.calls > 0}
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
      {@render unpriced(answer.unpriced)}
    {/if}
    {#if renewalsSent(answer.renewals) > 0}
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
      {@render renewed(answer.renewals)}
    {/if}
    
    <div class="grid grid-fit gap-x-gutter gap-y-wide">
      {#each CUTS as each (each)}
        <section
          class="flex min-w-0 flex-col gap-snug border-t border-edge pt-base"
          aria-label={say($lang, TITLES[each])}
        >
          <h2 class="text-note text-text-faint">{say($lang, TITLES[each])}</h2>
          {#if answer[each].length > 0}
            <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
            {@render cut(answer[each], answer.total)}
          {:else}
            <p class="text-note text-text-faint">—</p>
          {/if}
        </section>
      {/each}
    </div>
  {/if}
</Page>
