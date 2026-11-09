<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The unified catalog look. All behaviour and wording arrives from
  // `catalog.ts`; this file places the existing controls and plain-text versions.
  // The RULES.toml editor arrives as a snippet from the seat when the
  // models or rules pane is open.
  import type { Snippet } from "svelte";

  import Button from "../parts/button.svelte";
  import Segmented from "../parts/segmented.svelte";
  import type { CatalogLook } from "./catalog";

  interface Props extends CatalogLook {
    readonly rules?: Snippet | undefined;
  }

  const look: Props = $props();
</script>

<div class="flex min-w-0 flex-col gap-wide">
  <div class="flex min-w-0 flex-col gap-snug">
    {#if look.scopes.length > 0}
      <span class="text-note text-text-quiet">{look.scopeLabel}</span>
      <Segmented
        label={look.scopeLabel}
        options={look.scopes}
        held={look.scope}
        onPick={look.onScope}
      />
    {/if}
    <Segmented label={look.panesLabel} options={look.panes} held={look.pane} onPick={look.onPane} />
    <p class="text-note">
      <span class="figure text-text-quiet">{look.meter}</span>
      {#if look.delta !== undefined}
        <span class="figure text-text">· {look.delta}</span>
      {/if}
    </p>
    <div class="flex min-w-0 flex-wrap items-center gap-base">
      <Button
        label={look.unused.label}
        tone={look.unused.pressed ? "secondary" : "quiet"}
        onPress={look.unused.onPress}
      />
      {#if look.unused.pressed && look.groups.length > 0}
        <Button label={look.unused.offLabel} tone="quiet" onPress={look.unused.onOff} />
      {/if}
      {#if look.copy !== undefined && look.copy.options.length > 0}
        <select
          class="h-control max-w-full rounded-control border border-edge-input bg-raised px-snug text-note text-text"
          aria-label={look.copy.label}
          value=""
          onchange={(event) => {
            const picked = look.copy?.options.find((addr) => addr === event.currentTarget.value);
            if (picked !== undefined) look.copy?.onPick(picked);
          }}
        >
          <option value="" disabled selected>{look.copy.label}</option>
          {#each look.copy.options as addr (addr)}<option value={addr}>{addr}</option>{/each}
        </select>
      {/if}
    </div>
    {#if look.openSessions !== undefined}
      <p class="text-note text-text-faint">
        {look.openSessions.text}
        <a class="text-accent underline-offset-2 hover:underline" href={look.openSessions.href}>
          {look.openSessions.label}
        </a>
      </p>
    {/if}
  </div>
  {#if look.rulesNote !== undefined}
    <p class="text-note text-text-quiet">{look.rulesNote}</p>
  {/if}
  {#if look.rules !== undefined}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
    {@render look.rules()}
  {:else if look.empty !== undefined}
    <p class="text-note text-text-faint">{look.empty}</p>
  {:else}
    <ul class="flex min-w-0 flex-col gap-snug">
      {#each look.groups as group (group.key)}
        <li class="rounded-panel bg-raised p-snug">
          <details open={group.open} class="flex min-w-0 flex-col gap-snug">
            <summary
              class="flex min-w-0 cursor-pointer flex-wrap items-baseline gap-base text-note"
            >
              <span class="font-mono text-text">{group.title}</span>
              <span class="min-w-0 flex-1 text-text-faint">{group.summary}</span>
            </summary>
            <div class="flex min-w-0 flex-col gap-snug pt-snug">
              {#if group.rows.length > 0}
                <Segmented
                  label={group.summary}
                  options={group.choices}
                  held={group.held}
                  onPick={group.onPick}
                />
              {/if}
              <ul class="flex min-w-0 flex-col">
                {#each group.rows as row (row.key)}
                  <li class="flex min-w-0 flex-col border-t border-edge py-snug text-note">
                    <div class="flex min-w-0 flex-wrap items-center gap-x-base gap-y-tight">
                      <span class="flex min-w-0 grow basis-[16rem] flex-col gap-tight">
                        <span class="flex min-w-0 items-baseline gap-base">
                          <span class="shrink-0 font-mono text-text-quiet">{row.name}</span>
                          <span class="min-w-0 truncate text-text-faint">{row.hint}</span>
                          {#if row.change !== undefined}
                            <span class="shrink-0 text-alert">{row.change}</span>
                          {/if}
                        </span>
                        <span class="min-w-0 text-text-faint">{row.facts}</span>
                      </span>
                      <Segmented
                        label={row.label}
                        options={row.choices}
                        held={row.held}
                        onPick={row.onPick}
                      />
                    </div>
                    {#if row.actions.length > 0}
                      <div class="flex flex-wrap gap-base pt-snug">
                        {#each row.actions as action (action.key)}
                          <Button
                            label={action.label}
                            tone="quiet"
                            {...action.why === undefined ? {} : { why: action.why }}
                            onPress={action.onPress}
                          />
                        {/each}
                      </div>
                    {/if}
                    {#if row.comparison !== undefined}
                      <details class="pt-snug">
                        <summary class="cursor-pointer text-text-quiet">{row.change}</summary>
                        {#if row.comparison.audit !== undefined}
                          <p class="pt-tight text-text-faint">{row.comparison.audit}</p>
                        {/if}
                        <div class="grid min-w-0 grid-cols-2 gap-base pt-snug narrow:grid-cols-1">
                          <pre
                            class="min-w-0 overflow-auto whitespace-pre-wrap break-words rounded-card bg-page p-snug text-text-quiet">{row
                              .comparison.pinned}</pre>
                          <pre
                            class="min-w-0 overflow-auto whitespace-pre-wrap break-words rounded-card bg-page p-snug text-text">{row
                              .comparison.current}</pre>
                        </div>
                      </details>
                    {/if}
                  </li>
                {/each}
              </ul>
            </div>
          </details>
        </li>
      {/each}
    </ul>
    <span class="flex flex-wrap items-center gap-base">
      <Button
        label={look.save.label}
        tone="primary"
        {...look.save.why === undefined ? {} : { why: look.save.why }}
        onPress={look.save.onPress}
      />
    </span>
  {/if}
</div>
