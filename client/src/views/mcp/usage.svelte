<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // Every tool server of the city and what its tools were used for, from
  // the ledger (`Query::McpUsage`, wire D33): a server some building
  // names and nobody has called shows that it was never used; calls the
  // ledger cannot tie to a server any more stand last, under "removed",
  // with the whole tool name. Which tools a server offers now is the
  // handshake's question, which the servers list above asks.
  import type { DayCount, McpServerUsage, McpUse } from "../../wire";
  import type { UseRow } from "../parts/usage_uses.svelte";

  interface ToolLine {
    readonly tool: string;
    readonly uses: readonly UseRow[];
    readonly days: readonly DayCount[];
  }

  interface ServerLine {
    readonly server: string | null;
    readonly configured: boolean;
    readonly tools: readonly ToolLine[];
  }

  function rows(uses: readonly McpUse[]): UseRow[] {
    return uses.map((one) => ({ run: one.run, resident: one.resident ?? null, at: one.at, part: null, outcome: one.outcome }));
  }

  function lineOf(server: McpServerUsage): ServerLine {
    return {
      server: server.server ?? null,
      configured: server.configured,
      tools: server.tools.map((tool) => ({ tool: tool.tool, uses: rows(tool.uses), days: tool.per_day })),
    };
  }
</script>

<script lang="ts">
  import { readAnswer } from "../../core/answered";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Query } from "../../wire";
  import EmptyState from "../parts/empty.svelte";
  import Unanswered from "../parts/unanswered.svelte";
  import UsageExport from "../parts/usage_export.svelte";
  import UsageUses from "../parts/usage_uses.svelte";

  const u = ui();
  const lang = u.lang;
  const question: Query = { mcp_usage: { server: null } };
  const asked = u.conn.asking.ask(question);
  const read = $derived(readAnswer($asked, (held) => ("mcp_usage" in held ? held.mcp_usage.servers.map(lineOf) : undefined)));
</script>

<section class="flex min-w-0 flex-col gap-base">
  <div class="flex flex-wrap items-baseline justify-between gap-base">
    <h2 class="text-note text-text-faint">{say($lang, "usage_mcp_title")}</h2>
    <UsageExport what="mcp" />
  </div>
  {#if read.kind === "unavailable"}
    <Unanswered query={read.query} asked={question} />
  {:else if read.kind === "asking"}
    <p class="text-text-faint">…</p>
  {:else if read.value.length === 0}
    <EmptyState missing="usage_mcp_none" seat="region" />
  {:else}
    <ul class="flex flex-col">
      {#each read.value as server (server.server ?? "")}
        <li class="flex min-w-0 flex-col gap-tight border-b border-edge py-snug">
          <span class="flex items-baseline gap-base text-note">
            <span class="font-mono text-text-quiet">{server.server ?? say($lang, "usage_removed")}</span>
            {#if server.server !== null && !server.configured}
              <span class="text-text-faint">{say($lang, "usage_unconfigured")}</span>
            {/if}
          </span>
          {#if server.tools.length === 0}
            <p class="text-note text-text-faint">{say($lang, "usage_never")}</p>
          {/if}
          {#each server.tools as tool (tool.tool)}
            <div class="flex min-w-0 flex-col gap-tight pl-base">
              <span class="font-mono text-note text-text-quiet">{tool.tool}</span>
              <UsageUses uses={tool.uses} days={tool.days} />
            </div>
          {/each}
        </li>
      {/each}
    </ul>
  {/if}
</section>
