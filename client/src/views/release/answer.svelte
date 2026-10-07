<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // One answer about this release, drawn: where this binary stands,
  // what each registry the city asked said (`reading.ts`), and the
  // command that updates it through the channel that installed it, with
  // a copy key. Separate from the press in `release.svelte`, so the
  // gallery draws every state from a fixture.
  //
  // Nothing here updates anything: the command is the city's
  // `update.command`, printed for a User to run, because the channel
  // that installed the binary owns updating it.

  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { RegistryNewest, ReleaseAnswer, UpdateHint } from "../../wire";
  import Copy from "../parts/copy.svelte";
  import { confirmedCommand, registryLineOf, updateOf } from "./reading";

  interface Props {
    readonly answer: ReleaseAnswer;
  }

  const { answer }: Props = $props();
  const { lang } = ui();

  const stands = $derived("stands" in answer ? answer.stands : null);
  const unreleased = $derived("unreleased" in answer ? answer.unreleased : null);
  let selection = $state<{ readonly answer: ReleaseAnswer | null; readonly command: string | null }>({ answer: null, command: null });
  const unconfirmed = $derived("unconfirmed" in answer ? answer.unconfirmed : null);
  const refused = $derived("refused" in answer ? answer.refused : null);
  const newest = $derived(stands === null ? null : stands.newest);
</script>

{#snippet registries(lines: readonly RegistryNewest[])}
  <dl class="grid grid-cols-[minmax(0,8rem)_minmax(0,1fr)] gap-x-base gap-y-tight text-note">
    {#each lines as each (each.registry)}
      {@const line = registryLineOf(each)}
      <dt class="text-text-faint">{say($lang, line.registry)}</dt>
      <dd class="min-w-0 break-words">
        {#if line.newest !== null}
          <span class="font-mono text-text">{line.newest.version}</span>
          <span class="text-text-faint">{line.newest.released}</span>
        {/if}
        {#if line.reason !== null}
          <span class="text-text-quiet">{say($lang, line.reason.key)}</span>
          {#if line.reason.said !== null}
            <span class="font-mono text-text-faint">{line.reason.said}</span>
          {/if}
        {/if}
      </dd>
    {/each}
  </dl>
{/snippet}

{#snippet update(hint: UpdateHint)}
  {@const how = updateOf(hint)}
  {@const command = how.command ?? (selection.answer === answer ? confirmedCommand(hint, selection.command) : null)}
  <div class="flex min-w-0 flex-col gap-tight">
    <span class="text-note text-text-quiet">{say($lang, how.channel)}</span>
    {#if hint.alternatives.length > 0}
      <select aria-label={say($lang, "release_channel_confirm")} class="h-control min-w-0 rounded-control bg-raised px-snug text-note text-text" value={selection.answer === answer ? selection.command ?? "" : ""} onchange={(event) => { selection = { answer, command: confirmedCommand(hint, event.currentTarget.value) }; }}>
        <option value="">{say($lang, "release_channel_confirm")}</option>
        {#each hint.alternatives as choice (choice)}<option value={choice}>{choice}</option>{/each}
      </select>
    {/if}
    {#if command !== null}
      <div class="flex min-w-0 items-start gap-tight">
        <code class="min-w-0 flex-1 break-words rounded-control bg-chrome px-snug py-tight font-mono text-note text-text">{command}</code>
        <Copy text={command} />
      </div>
    {/if}
  </div>
{/snippet}

<div class="flex min-w-0 flex-col gap-snug">
  {#if refused !== null}
    <p class="text-note text-alert">
      {say($lang, "release_refused_registries")}
      <code class="font-mono text-note text-text-quiet">{refused.refusal.recovery}</code>
    </p>
  {/if}
  {#if unconfirmed !== null}
    <p class="text-note text-text">{fill(say($lang, "release_mine"), { version: unconfirmed.mine.version, released: unconfirmed.mine.released })}</p>
    <p class="text-note text-text-quiet">{say($lang, "release_origin_unconfirmed")}</p>
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself) -->
    {@render registries(unconfirmed.registries)}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself) -->
    {@render update(unconfirmed.update)}
  {/if}
  {#if unreleased !== null}
    <p class="text-note text-text-quiet">{say($lang, "release_source")}</p>
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
    {@render registries(unreleased.registries)}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
    {@render update(unreleased.update)}
  {/if}
  {#if stands !== null}
    <p class="text-note text-text">
      {fill(say($lang, stands.mine.released.length === 0 ? "release_mine_undated" : "release_mine"), { version: stands.mine.version, released: stands.mine.released })}
    </p>
    {#if stands.verdict === "current"}
      <p class="text-note text-accent">{say($lang, "release_current")}</p>
    {:else if stands.verdict === "ahead"}
      <p class="text-note text-text-quiet">{fill(say($lang, "release_ahead"), { version: newest?.version ?? "" })}</p>
    {:else if stands.verdict === "behind"}
      <p class="text-note text-alert">
        {fill(say($lang, newest?.released.length === 0 ? "release_behind_undated" : "release_behind"), { version: newest?.version ?? "", released: newest?.released ?? "" })}
      </p>
    {/if}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
    {@render registries(stands.registries)}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
    {@render update(stands.update)}
  {/if}
</div>
