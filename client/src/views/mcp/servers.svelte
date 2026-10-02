<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // `claude mcp list`, as a page: one line per server with its
  // transport, where it stands and how many tools it offers, and a row
  // that opens on the tool list with each tool's input schema.
  //
  // A server nobody has reached yet says so rather than looking
  // healthy. The three states come from the city's own handshake, so
  // what a person reads here and what a model is given cannot disagree.
  //
  // **The row is `parts/row`'s** (client/Spec.lean §7 names this seat): the
  // list draws `RowList` and each server draws a `Row`, so a keyboard
  // walks the list and the remove control stays separately reachable.
  // What the row cannot hold - the opened tool list - is the one row
  // this file draws for itself, which is the door `Row` leaves open for
  // a list that holds more than rows.
  //
  // Removing a server is the one press on this page that cannot be
  // undone from here, so it is confirmed through `parts/dialog` before
  // anything is sent (ux A10): the cancelling answer is written first
  // and lands under the hand.

  import type { McpServer, McpServerHealth, McpState } from "../../wire";

  export interface ServersProps {
    readonly servers: readonly McpServer[];
    readonly health: readonly McpServerHealth[];
    readonly onCheck: () => void;
    readonly onRemove: (label: string) => void;
  }

  // The one place a transport becomes the line a person reads.
  function targetOf(server: McpServer): string {
    const transport = server.transport;
    if ("stdio" in transport) return [transport.stdio.command, ...transport.stdio.args].join(" ");
    if ("http" in transport) return transport.http.url;
    return transport.sse.url;
  }

  function transportWord(server: McpServer): "mcp_kind_stdio" | "mcp_kind_http" | "mcp_kind_sse" {
    const transport = server.transport;
    if ("stdio" in transport) return "mcp_kind_stdio";
    if ("http" in transport) return "mcp_kind_http";
    return "mcp_kind_sse";
  }

  // The name/value rows a server is configured with, whichever of the
  // two tables this transport keeps them in.
  function pairsOf(server: McpServer): readonly (readonly [string, string])[] {
    const transport = server.transport;
    if ("stdio" in transport) return transport.stdio.env;
    if ("http" in transport) return transport.http.headers;
    return transport.sse.headers;
  }

  function pairCaption(server: McpServer): "mcp_env" | "mcp_header" {
    return "stdio" in server.transport ? "mcp_env" : "mcp_header";
  }

  // How many tools the server listed, or an em dash where nobody has
  // asked: "none" and "not asked" are different facts about a server.
  function toolCount(state: McpState | undefined): string {
    if (state === undefined) return "—";
    return "connected" in state ? String(state.connected.tools.length) : "—";
  }
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Badge from "../parts/badge.svelte";
  import Button from "../parts/button.svelte";
  import Dialog from "../parts/dialog.svelte";
  import EmptyState from "../parts/empty.svelte";
  import { recoveryWords } from "../parts/notice_title";
  import Row, { RowList } from "../parts/row.svelte";

  const { servers, health, onCheck, onRemove }: ServersProps = $props();

  const { lang } = ui();

  // Which server's tool list is open, and which one is being confirmed
  // for removal. Two questions with one live answer each; the second is
  // emptied before the command leaves, so a dialog never survives its
  // own answer.
  let open = $state<string | null>(null);
  let confirming = $state<string | null>(null);

  const stateOf = (label: string): McpState | undefined =>
    health.find((line) => line.label === label)?.state;
</script>

{#if servers.length === 0}
  <EmptyState missing="mcp_none" seat="region" />
{:else}
  <div class="flex flex-col gap-tight">
    <div class="flex justify-end">
      <Button label={say($lang, "mcp_check")} tone="quiet" onPress={onCheck} />
    </div>
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression, @typescript-eslint/no-unsafe-call (a snippet call is the render itself; the typechecker types a snippet exported from a .svelte module as unresolvable) -->
    {@render RowList({ label: say($lang, "mcp_servers"), rows: serverRows })}
  </div>
{/if}

{#snippet serverRows()}
  {#each servers as server (server.label)}
    <Row primary={server.label} secondary={targetOf(server)}>
      {#snippet status()}
        <span class="flex items-center gap-tight text-text-faint">
          <Badge text={say($lang, transportWord(server))} />
          <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
          {@render stateBadge(stateOf(server.label))}
          {say($lang, "mcp_tools")}
          <span class="text-text-faint">{toolCount(stateOf(server.label))}</span>
        </span>
      {/snippet}
      {#snippet actions()}
        <Button
          label={say($lang, "mcp_expand")}
          tone="quiet"
          onPress={() => {
            open = open === server.label ? null : server.label;
          }}
        />
        <Button
          label={say($lang, "setup_remove")}
          tone="destructive"
          onPress={() => {
            confirming = server.label;
          }}
        />
      {/snippet}
    </Row>
    {#if open === server.label}
      <li class="flex flex-col gap-tight border-b border-edge px-base py-snug text-note">
        <div class="flex min-w-0 items-baseline gap-snug">
          <span class="shrink-0 text-text-quiet">{say($lang, "mcp_target")}</span>
          <span class="min-w-0 wrap-anywhere font-mono text-text">{targetOf(server)}</span>
        </div>
        {#if pairsOf(server).length > 0}
          <div class="flex min-w-0 flex-col gap-tight">
            <span class="text-text-quiet">{say($lang, pairCaption(server))}</span>
            {#each pairsOf(server) as pair, at (at)}
              <span class="min-w-0 wrap-anywhere font-mono text-text">
                <!-- wording-ok: the "=" is punctuation in type, not a word; the two values are the city's own. -->
                {pair[0]}<span class="text-text-faint">&nbsp;=&nbsp;</span>{pair[1]}
              </span>
            {/each}
          </div>
        {/if}
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
        {@render standing(stateOf(server.label))}
      </li>
    {/if}
  {/each}
{/snippet}

{#snippet stateBadge(state: McpState | undefined)}
  {#if state === undefined}
    <Badge text={say($lang, "mcp_state_unknown")} dot />
  {:else if "connected" in state}
    <Badge text={say($lang, "mcp_state_connected")} dot weight="live" />
  {:else if "authenticating" in state}
    <Badge text={say($lang, "mcp_state_auth")} dot weight="live" />
  {:else if "failed" in state}
    <Badge text={say($lang, "mcp_state_failed")} dot weight="alert" />
  {/if}
{/snippet}

{#snippet standing(state: McpState | undefined)}
  {#if state === undefined}
    <p class="text-text-faint">{say($lang, "mcp_state_unknown")}</p>
  {:else if "authenticating" in state}
    <p class="text-text-faint">{state.authenticating.recovery}</p>
  {:else if "failed" in state}
    <!-- The three-part refusal: what failed and on what, as the city
    wrote them, and what a person can do about it in the reader's own
    language, by the code (`parts/notice_title.ts`). -->
    <p class="text-text-faint">
      {state.failed.refusal.action}: {state.failed.refusal.subject} — {recoveryWords(
        $lang,
        state.failed.refusal.code,
        state.failed.refusal.recovery,
      )}
    </p>
  {:else if "connected" in state}
    <ul class="flex flex-col gap-tight">
      {#each state.connected.tools as tool (tool.name)}
        <li class="flex min-w-0 flex-col gap-tight">
          <span class="font-mono text-text">{tool.name}</span>
          <span class="text-text-faint">{tool.disclosure}</span>
          <pre
            class="min-w-0 overflow-x-auto rounded-control bg-raised px-base py-snug font-mono text-text-quiet"
          >{JSON.stringify(tool.input_schema, null, 2)}</pre>
        </li>
      {/each}
    </ul>
  {/if}
{/snippet}

<Dialog
  open={confirming !== null}
  title={say($lang, "part_remove_server")}
  detail={say($lang, "part_remove_server_detail")}
  confirmLabel={say($lang, "setup_remove")}
  cancelLabel={say($lang, "part_cancel")}
  destructive
  onConfirm={() => {
    const label = confirming;
    confirming = null;
    if (label !== null) onRemove(label);
  }}
  onCancel={() => {
    confirming = null;
  }}
/>
