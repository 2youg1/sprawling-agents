<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The two usage panels - every skill and what runs read of it, every
  // tool server and what its tools were called for - each in three
  // cities: one whose ledger holds nothing to count, one with the few
  // rows a city has after a week, and one long enough that names, rows
  // and day counts have to hold their shape at the width the panel is
  // drawn in. The skill panel stands under the skills page at the
  // page's width; the MCP panel stands in the right column of the MCP
  // page, so its cases are drawn at that column's width.
  //
  // The answers are a fixed script - counted runs, counted days, one
  // minute apart - rather than a random one, so the render gate sees
  // the same table every time.

  import type { Answer, DayCount, McpServerUsage, McpUse, Query, SkillAudit, SkillUsageLine, SkillUse } from "../../wire";
  import { Address, B3Hash, RunId, Seq, TimeMs } from "../../wire";

  const START = 1_790_000_000_000;
  const MINUTE = 60_000;
  const DAY = 24 * 60 * MINUTE;

  function digest(n: number): B3Hash {
    return B3Hash.make(`${n.toString(16).padStart(8, "0")}${"af3c1d2e4b5a6978".repeat(3)}0c1d2e3f`);
  }

  function run(n: number): RunId {
    return RunId.make(`0199c0de-0000-4000-8000-${n.toString(16).padStart(12, "0")}`);
  }

  // A day as the city writes it, a UTC calendar date.
  function dayOf(n: number): string {
    return new Date(START + n * DAY).toISOString().slice(0, 10);
  }

  // `count` uses spread over `days` days, one minute apart within a day,
  // with every seventh one refused and every eleventh unrecorded, and
  // the per-day counts that fold them.
  function spread(count: number, days: number): { readonly at: readonly number[]; readonly perDay: readonly DayCount[] } {
    const at = Array.from({ length: count }, (_, n) => START + (n % days) * DAY + Math.floor(n / days) * MINUTE).sort((a, b) => a - b);
    const perDay = Array.from({ length: Math.min(count, days) }, (_, n) => ({
      day: dayOf(n),
      count: at.filter((moment) => moment >= START + n * DAY && moment < START + (n + 1) * DAY).length,
    }));
    return { at, perDay };
  }

  function outcome(n: number): "ok" | "failed" | "unknown" {
    return n % 11 === 10 ? "unknown" : n % 7 === 6 ? "failed" : "ok";
  }

  const RESIDENTS = [Address.make("release/ledger"), Address.make("memory/gate"), null] as const;

  function skill(name: string, count: number, days: number, audit: SkillAudit, part: string): SkillUsageLine {
    const { at, perDay } = spread(count, days);
    const uses: SkillUse[] = at.map((moment, n) => ({
      at: TimeMs.make(moment),
      digest: digest(name.length),
      outcome: outcome(n),
      part,
      resident: RESIDENTS[n % RESIDENTS.length] ?? null,
      run: run(n + 1),
      seq: Seq.make(1_000 + n),
    }));
    return {
      name,
      held: [{ audit, digest: digest(name.length), shelf: { library: Address.make(`.sprawling/library/engineering/${name}.md`) } }],
      per_day: perDay,
      uses,
      versions: count === 0 ? [] : [{ at: TimeMs.make(START), digest: digest(name.length), run: run(1), seq: Seq.make(1_000) }],
    };
  }

  function tool(name: string, count: number, days: number): McpServerUsage["tools"][number] {
    const { at, perDay } = spread(count, days);
    const uses: McpUse[] = at.map((moment, n) => ({
      at: TimeMs.make(moment),
      outcome: outcome(n),
      resident: RESIDENTS[n % RESIDENTS.length] ?? null,
      run: run(n + 1),
      seq: Seq.make(2_000 + n),
    }));
    return { tool: name, uses, per_day: perDay };
  }

  const PASSED: SkillAudit = { state: "audited", at: Seq.make(900), verdict: "pass" };
  const WARNED: SkillAudit = { state: "audited", at: Seq.make(901), verdict: "warn" };
  const STALE: SkillAudit = { state: "stale", audited: digest(1) };
  const UNAUDITED: SkillAudit = { state: "unaudited" };

  // A week: two skills read, one whose content changed since its audit,
  // and one on the shelf nobody has read.
  const SKILLS_TYPICAL: readonly SkillUsageLine[] = [
    skill("diagnosing-bugs", 9, 4, PASSED, "SKILL.md"),
    skill("resolving-merge-conflicts", 3, 2, STALE, "SKILL.md"),
    skill("kiln-firing", 0, 1, UNAUDITED, "SKILL.md"),
  ];

  // Long names, a skill read every day for a month, every audit state,
  // a copy no shelf holds any more, and a long list of skills nobody
  // has read.
  const SKILLS_LONG: readonly SkillUsageLine[] = [
    skill("apostle-opusmethodology-karatani-origins", 240, 30, WARNED, "references/kernels-in-japanese.md"),
    skill("rust-coverage-meaningful-tests", 61, 21, PASSED, "SKILL.md"),
    { ...skill("workflow-authoring", 12, 6, STALE, "SKILL.md"), held: [] },
    skill("diagnosing-bugs", 9, 4, { state: "audited", at: Seq.make(902), verdict: "fail" }, "SKILL.md"),
    ...[
      "authority-review",
      "blast-radius",
      "codebase-design",
      "domain-modeling",
      "grilling",
      "lieflat-charts",
      "m05-type-driven",
      "m13-domain-error",
      "m15-anti-pattern",
      "rust-api-design",
      "rust-cargo-build",
      "rust-documentation",
      "rust-hardening",
      "rust-module-layout",
      "rust-semver",
      "rust-workspace",
    ].map((name) => skill(name, 0, 1, UNAUDITED, "SKILL.md")),
  ];

  // A week: one server with two tools in use, one a building names and
  // nobody has called.
  const SERVERS_TYPICAL: readonly McpServerUsage[] = [
    { server: "github", configured: true, tools: [tool("create_issue", 4, 3), tool("search_code", 17, 5)] },
    { server: "filesystem", configured: true, tools: [] },
  ];

  // Long server and tool names, a server no building names any more,
  // a tool called every day for a month, and the calls the ledger can
  // no longer tie to a server.
  const SERVERS_LONG: readonly McpServerUsage[] = [
    {
      server: "composio-github-enterprise-cloud-integration",
      configured: true,
      tools: [
        tool("github_create_pull_request_review_comment_reply", 180, 30),
        tool("github_list_repository_collaborators_with_permissions", 44, 14),
        tool("search_code", 3, 2),
      ],
    },
    { server: "postgres-analytics-read-replica", configured: false, tools: [tool("query", 27, 9)] },
    { server: "filesystem", configured: true, tools: [] },
    { server: "browser-automation", configured: true, tools: [] },
    { server: null, configured: false, tools: [tool("mcp__retired-wiki__page_history_with_revisions", 6, 3)] },
  ];

  function skills(lines: readonly SkillUsageLine[]): (query: Query) => Answer | undefined {
    return (query) => (typeof query === "object" && "skill_usage" in query ? { skill_usage: { skills: lines } } : undefined);
  }

  function servers(lines: readonly McpServerUsage[]): (query: Query) => Answer | undefined {
    return (query) => (typeof query === "object" && "mcp_usage" in query ? { mcp_usage: { servers: lines } } : undefined);
  }

  // The right column of the MCP page at a 1440 window.
  const MCP_COLUMN = 520;
</script>

<script lang="ts">
  import McpUsage from "../mcp/usage.svelte";
  import SkillUsage from "../setup/skill_usage.svelte";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";
</script>

<Case label="skill usage · nothing on a shelf, nothing read">
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={skills([])}>
    <SkillUsage />
  </Stand>
</Case>

<Case label="skill usage · a week: read, audit stale, never read">
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={skills(SKILLS_TYPICAL)}>
    <SkillUsage />
  </Stand>
</Case>

<Case label="skill usage · long names, a month of reads, many never read">
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={skills(SKILLS_LONG)}>
    <SkillUsage />
  </Stand>
</Case>

<Case label="mcp usage · no server configured or called" width={MCP_COLUMN}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={servers([])}>
    <McpUsage />
  </Stand>
</Case>

<Case label="mcp usage · a week: two tools called, one server never" width={MCP_COLUMN}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={servers(SERVERS_TYPICAL)}>
    <McpUsage />
  </Stand>
</Case>

<Case label="mcp usage · long names, unconfigured and removed servers" width={MCP_COLUMN}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={servers(SERVERS_LONG)}>
    <McpUsage />
  </Stand>
</Case>
