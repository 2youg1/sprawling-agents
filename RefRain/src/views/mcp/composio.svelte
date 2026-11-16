<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { Intake } from "./draft";

  export interface ComposioProps {
    readonly intake: Intake;
  }

  const COMPOSIO_BASE = "https://backend.composio.dev/v3/mcp/";

  const KEY_HEADER = "x-api-key";
</script>

<script lang="ts">
  import { get } from "svelte/store";

  import type { ToolkitLine, ToolkitSlug } from "../../wire";
  import { QUERIES } from "../../core/asking";
  import { enrol } from "../../core/enrol";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Badge from "../parts/badge.svelte";
  import Button from "../parts/button.svelte";
  import Field from "../parts/field.svelte";
  import Unanswered from "../parts/unanswered.svelte";
  import { recoveryWords } from "../parts/notice_title";
  import { EMPTY, WHY, encode } from "./draft";
  import { consentUrl, shelfOf } from "./shelf.svelte";

  const { intake }: ComposioProps = $props();

  const u = ui();
  const { lang } = u;
  const shelf = shelfOf();
  const read = $derived(shelf.answer());
  const answer = $derived(read.kind === "held" ? read.value : undefined);

  let key = $state("");
  let reference = $state<string | null>(null);
  let refused = $state(false);
  let pressed = $state<ToolkitSlug | null>(null);
  let tab: Window | null = null;
  let routed: ToolkitSlug | null = null;

  let serverId = $state("");
  let userId = $state("");

  function store(): void {
    const value = key.trim();
    if (value === "") return;
    refused = false;
    void enrol({
      origin: u.origin,
      token: u.credential(),
      realm: "mcp",
      name: "composio",
      value,
      lang: get(lang),
    }).then((outcome) => {
      if (outcome.kind === "stored") {
        reference = outcome.reference;
        key = "";
        shelf.recheck();
        return;
      }
      refused = true;
    });
  }

  $effect(() => {
    const waiting = pressed;
    const rows = shelf.rows();
    if (waiting === null || routed === waiting) return;
    const row = rows.find((line) => line.slug === waiting);
    const url = row === undefined ? null : consentUrl(row);
    if (url === null) return;
    const held = tab;
    if (held !== null && !held.closed) held.location.href = url;
    routed = waiting;
  });

  function press(line: ToolkitLine): void {
    routed = null;
    tab = window.open("", "_blank", "noopener");
    pressed = line.slug;
    shelf.connect(line.slug);
  }

  const byIdUrl = $derived.by((): string => {
    const user = userId.trim();
    const id = serverId.trim();
    return `${COMPOSIO_BASE}${id}${user === "" ? "" : `?user_id=${encodeURIComponent(user)}`}`;
  });
  const byIdEncoded = $derived(
    encode(
      {
        ...EMPTY,
        label: serverId.trim(),
        transport: "http",
        url: serverId.trim() === "" ? "" : byIdUrl,
        headers: reference === null ? [] : [{ name: KEY_HEADER, value: reference }],
      },
      intake.taken(),
    ),
  );
  const byIdReason = $derived.by((): string | null => {
    const scope = intake.why();
    if (scope !== null) return scope;
    if (reference === null) return say($lang, "mcp_key_first");
    return byIdEncoded.kind === "blocked" ? say($lang, WHY[byIdEncoded.blocker]) : null;
  });

  function byIdSend(): void {
    const out = byIdEncoded;
    if (out.kind === "ready" && intake.offer(out.server)) {
      serverId = "";
      userId = "";
    }
  }
</script>

<div class="flex flex-col gap-base">
  <div class="flex flex-wrap items-end gap-base">
    <div class="min-w-0 flex-[1_1_16ch]">
      <Field
        label={say($lang, "mcp_api_key")}
        kind="password"
        mono
        value={key}
        onInput={(typed) => {
          key = typed;
        }}
      />
    </div>
    <Button label={say($lang, "mcp_key_store")} tone="secondary" onPress={store} />
  </div>
  <a
    href="https://platform.composio.dev"
    target="_blank"
    rel="noopener noreferrer"
    class="w-fit text-note text-text-faint transition-colors ease-leave hover:text-text-quiet hover:ease-arrive"
  >
    {say($lang, "mcp_open_composio")}
  </a>
  {#if reference !== null}
    <div class="flex min-w-0 items-center gap-snug text-note">
      <Badge text={say($lang, "mcp_key_stored")} weight="live" dot />
      <span class="min-w-0 truncate font-mono text-text-faint">{reference}</span>
    </div>
  {/if}
  {#if refused}
    <p class="text-note text-alert" role="alert">
      {say($lang, "link_refused")}
    </p>
  {/if}

  {#if read.kind === "unavailable"}
    <Unanswered query={read.query} asked={QUERIES.toolkits} />
  {:else if answer === undefined}
    <p class="text-note text-text-faint">{say($lang, "mcp_shelf_asking")}</p>
  {:else if answer === "unenrolled"}
    <p class="text-note text-text-faint">{say($lang, "mcp_shelf_needs_key")}</p>
  {:else if "refused" in answer}
    <div class="flex flex-col gap-tight" role="alert">
      <p class="text-note text-alert">{answer.refused.refusal.subject}</p>
      <p class="text-note text-text-faint">
        {recoveryWords($lang, answer.refused.refusal.code, answer.refused.refusal.recovery)}
      </p>
      <Button label={say($lang, "mcp_shelf_again")} tone="quiet" onPress={shelf.recheck} />
    </div>
  {:else if "shelf" in answer}
    <ul class="flex flex-col gap-tight" aria-label={say($lang, "mcp_toolkits")}>
      {#each answer.shelf.toolkits as line (line.slug)}
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
        {@render toolkit(line)}
      {/each}
    </ul>
  {/if}

  <details class="text-note">
    <summary
      class="cursor-pointer text-text-faint transition-colors ease-leave hover:text-text-quiet hover:ease-arrive"
    >
      {say($lang, "mcp_by_id")}
    </summary>
    <div class="mt-base flex flex-col gap-base">
      <div class="grid gap-base grid-cols-[repeat(auto-fit,minmax(320px,1fr))]">
        <Field
          label={say($lang, "mcp_server_id")}
          mono
          value={serverId}
          onInput={(typed) => {
            serverId = typed;
          }}
        />
        <Field
          label={say($lang, "mcp_user_id")}
          mono
          value={userId}
          onInput={(typed) => {
            userId = typed;
          }}
        />
      </div>
      {#if byIdReason === null}
        <Button label={say($lang, "mcp_add")} tone="secondary" onPress={byIdSend} />
      {:else}
        <div class="flex min-w-0 items-center gap-base">
          <Button label={say($lang, "mcp_add")} why={byIdReason} />
          <span class="min-w-0 text-note text-text-faint">{byIdReason}</span>
        </div>
      {/if}
    </div>
  </details>
</div>

{#snippet toolkit(line: ToolkitLine)}
  <li class="rounded-card bg-chrome">
    <div class="flex min-w-0 items-center gap-base px-base py-snug text-note">
      <span class="w-figure shrink-0 truncate text-text">{line.name}</span>
      <span class="min-w-0 flex-1 truncate font-mono text-text-faint">{line.slug}</span>
      <Badge text={line.auth} />
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
      {@render action(line)}
    </div>
  </li>
{/snippet}

{#snippet action(line: ToolkitLine)}
  {#if line.standing === "absent"}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
    {@render again(say($lang, "mcp_connect"), line)}
  {:else if "connected" in line.standing}
    <Badge text={line.standing.connected.alias} weight="live" dot />
  {:else if "awaiting" in line.standing}
    <a
      href={line.standing.awaiting.consent_url}
      target="_blank"
      rel="noopener noreferrer"
      class="text-note text-accent hover:underline"
      onclick={() => {
        shelf.recheck();
      }}
    >
      {say($lang, "mcp_consent_finish")}
    </a>
  {:else if "refused" in line.standing}
    <div class="flex min-w-0 items-center gap-snug">
      <span class="min-w-0 truncate text-note text-alert">
        {recoveryWords(
          $lang,
          line.standing.refused.refusal.code,
          line.standing.refused.refusal.recovery,
        )}
      </span>
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
      {@render again(say($lang, "mcp_connect_again"), line)}
    </div>
  {/if}
{/snippet}

{#snippet again(label: string, line: ToolkitLine)}
  <Button
    {label}
    tone="secondary"
    onPress={() => {
      press(line);
    }}
  />
{/snippet}
