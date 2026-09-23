<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // `claude mcp add --transport http|sse <name> <url>`, as a form: the
  // three transports side by side, the url, and the headers the request
  // carries.
  //
  // Picking `stdio` moves to the command door rather than emptying this
  // one, because a stdio server has no url and a person who picked it
  // wants the other form; picking `sse` stays here and says what the
  // wire cannot spell.

  import { EMPTY } from "./draft";
  import type { Draft, Intake, Transport } from "./draft";

  export interface ByUrlProps {
    readonly intake: Intake;
    readonly onCommandDoor: () => void;
  }

  // The three transports in the order a person meets them, `stdio` last
  // because picking it leaves this door rather than filling it in.
  const TRANSPORTS: readonly Transport[] = ["http", "sse", "stdio"];

  const OVER_A_URL: Draft = { ...EMPTY, transport: "http" };
</script>

<script lang="ts">
  import { SPELLED, WHY, encode } from "./draft";
  import { say } from "../../core/lang";
  import type { Choice } from "../parts/segmented";
  import { ui } from "../../ui";
  import Button from "../parts/button.svelte";
  import Field from "../parts/field.svelte";
  import PairTable from "./pairs.svelte";
  import Segmented from "../parts/segmented.svelte";

  const { intake, onCommandDoor }: ByUrlProps = $props();

  const { lang } = ui();

  let draft = $state.raw<Draft>(OVER_A_URL);

  const encoded = $derived(encode(draft, intake.taken()));
  const reason = $derived.by((): string | null => {
    const scope = intake.why();
    if (scope !== null) return scope;
    return encoded.kind === "blocked" ? say($lang, WHY[encoded.blocker]) : null;
  });
  const transports = $derived(
    TRANSPORTS.map((each): Choice<Transport> => ({ value: each, label: say($lang, SPELLED[each]) })),
  );

  function send(): void {
    const out = encoded;
    if (out.kind === "ready" && intake.offer(out.server)) {
      draft = OVER_A_URL;
    }
  }

  function pick(transport: Transport): void {
    if (transport === "stdio") {
      onCommandDoor();
      return;
    }
    draft = { ...draft, transport };
  }
</script>

<div class="flex flex-col gap-base">
  <div class="flex flex-wrap items-center gap-snug">
    <span class="text-note text-text-quiet">{say($lang, "mcp_transport")}</span>
    <Segmented
      label={say($lang, "mcp_transport")}
      options={transports}
      held={draft.transport}
      onPick={pick}
    />
  </div>
  <div class="grid gap-base grid-cols-[repeat(auto-fit,minmax(320px,1fr))]">
    <Field
      label={say($lang, "mcp_label")}
      help={say($lang, "mcp_label_help")}
      mono
      value={draft.label}
      onInput={(label) => {
        draft = { ...draft, label };
      }}
    />
    <Field
      label={say($lang, "mcp_url")}
      mono
      value={draft.url}
      onInput={(url) => {
        draft = { ...draft, url };
      }}
    />
  </div>
  <PairTable
    caption={say($lang, "mcp_header")}
    note={say($lang, "mcp_header_help")}
    rows={draft.headers}
    onChange={(headers) => {
      draft = { ...draft, headers };
    }}
  />
  {#if reason === null}
    <Button label={say($lang, "mcp_add")} tone="primary" onPress={send} />
  {:else}
    <div class="flex min-w-0 items-center gap-base">
      <Button label={say($lang, "mcp_add")} why={reason} />
      <span class="min-w-0 text-note text-text-faint">{reason}</span>
    </div>
  {/if}
</div>
