<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // `claude mcp add <name> -- <cmd> [args]`, as a form: a name, the
  // shell line that starts the program, and the environment that line
  // is given.
  //
  // The environment table is here because a stdio server that needs a
  // token is the ordinary case; today it can be filled in and not sent,
  // and the sentence under it says which half of that is the city's
  // fault.

  import type { Draft, Intake } from "./draft";

  export interface ByCommandProps {
    readonly intake: Intake;
  }
</script>

<script lang="ts">
  import { EMPTY, WHY, encode } from "./draft";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Button from "../parts/button.svelte";
  import Field from "../parts/field.svelte";
  import PairTable from "./pairs.svelte";

  const { intake }: ByCommandProps = $props();

  const { lang } = ui();

  let draft = $state.raw<Draft>(EMPTY);

  const encoded = $derived(encode(draft, intake.taken()));
  const reason = $derived.by((): string | null => {
    const scope = intake.why();
    if (scope !== null) return scope;
    return encoded.kind === "blocked" ? say($lang, WHY[encoded.blocker]) : null;
  });

  function send(): void {
    const out = encoded;
    if (out.kind === "ready" && intake.offer(out.server)) {
      draft = EMPTY;
    }
  }
</script>

<div class="flex flex-col gap-base">
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
      label={say($lang, "mcp_command")}
      help={say($lang, "mcp_command_help")}
      mono
      value={draft.command}
      onInput={(command) => {
        draft = { ...draft, command };
      }}
    />
  </div>
  <PairTable
    caption={say($lang, "mcp_env")}
    note={say($lang, "mcp_env_help")}
    rows={draft.env}
    onChange={(env) => {
      draft = { ...draft, env };
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
