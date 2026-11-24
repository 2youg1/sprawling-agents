// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type {
  Answer,
  DayCount,
  McpServerUsage,
  McpUse,
  Query,
  SkillAudit,
  SkillUsageLine,
  SkillUse,
} from "../../wire";
import { Address, B3Hash, RunId, Seq, TimeMs } from "../../wire";
import McpUsage from "../mcp/usage.solid";
import SkillUsage from "../setup/skill_usage_view";
import { GalleryCase as Case } from "./case";
import Stand from "./stand.solid";

export default function Used() {
  const START = 1_790_000_000_000;
  const MINUTE = 60_000;
  const DAY = 24 * 60 * MINUTE;

  function digest(n: number): B3Hash {
    return B3Hash.make(`${n.toString(16).padStart(8, "0")}${"af3c1d2e4b5a6978".repeat(3)}0c1d2e3f`);
  }

  function run(n: number): RunId {
    return RunId.make(`0199c0de-0000-4000-8000-${n.toString(16).padStart(12, "0")}`);
  }

  function dayOf(n: number): string {
    return new Date(START + n * DAY).toISOString().slice(0, 10);
  }

  function spread(
    count: number,
    days: number,
  ): { readonly at: readonly number[]; readonly perDay: readonly DayCount[] } {
    const at = Array.from(
      { length: count },
      (_, n) => START + (n % days) * DAY + Math.floor(n / days) * MINUTE,
    ).sort((a, b) => a - b);
    const perDay = Array.from({ length: Math.min(count, days) }, (_, n) => ({
      day: dayOf(n),
      count: at.filter((moment) => moment >= START + n * DAY && moment < START + (n + 1) * DAY)
        .length,
    }));
    return { at, perDay };
  }

  function outcome(n: number): "ok" | "failed" | "unknown" {
    return n % 11 === 10 ? "unknown" : n % 7 === 6 ? "failed" : "ok";
  }

  const RESIDENTS = [Address.make("release/ledger"), Address.make("memory/gate"), null] as const;

  function skill(
    name: string,
    count: number,
    days: number,
    audit: SkillAudit,
    part: string,
  ): SkillUsageLine {
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
      held: [
        {
          audit,
          digest: digest(name.length),
          shelf: "library",
        },
      ],
      per_day: perDay,
      uses,
      versions:
        count === 0
          ? []
          : [
              {
                at: TimeMs.make(START),
                digest: digest(name.length),
                run: run(1),
                seq: Seq.make(1_000),
              },
            ],
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

  const SKILLS_TYPICAL: readonly SkillUsageLine[] = [
    skill("diagnosing-bugs", 9, 4, PASSED, "SKILL.md"),
    skill("resolving-merge-conflicts", 3, 2, STALE, "SKILL.md"),
    skill("kiln-firing", 0, 1, UNAUDITED, "SKILL.md"),
  ];

  const SKILLS_LONG: readonly SkillUsageLine[] = [
    skill(
      "apostle-opusmethodology-karatani-origins",
      240,
      30,
      WARNED,
      "references/kernels-in-japanese.md",
    ),
    skill("rust-coverage-meaningful-tests", 61, 21, PASSED, "SKILL.md"),
    { ...skill("workflow-authoring", 12, 6, STALE, "SKILL.md"), held: [] },
    skill(
      "diagnosing-bugs",
      9,
      4,
      { state: "audited", at: Seq.make(902), verdict: "fail" },
      "SKILL.md",
    ),
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

  const SERVERS_TYPICAL: readonly McpServerUsage[] = [
    {
      server: "github",
      configured: true,
      tools: [tool("create_issue", 4, 3), tool("search_code", 17, 5)],
    },
    { server: "filesystem", configured: true, tools: [] },
  ];

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
    {
      server: "postgres-analytics-read-replica",
      configured: false,
      tools: [tool("query", 27, 9)],
    },
    { server: "filesystem", configured: true, tools: [] },
    { server: "browser-automation", configured: true, tools: [] },
    {
      server: null,
      configured: false,
      tools: [tool("mcp__retired-wiki__page_history_with_revisions", 6, 3)],
    },
  ];

  function skills(lines: readonly SkillUsageLine[]): (query: Query) => Answer | undefined {
    return (query) =>
      typeof query === "object" && "skill_usage" in query
        ? { skill_usage: { skills: lines } }
        : undefined;
  }

  function servers(lines: readonly McpServerUsage[]): (query: Query) => Answer | undefined {
    return (query) =>
      typeof query === "object" && "mcp_usage" in query
        ? { mcp_usage: { servers: lines } }
        : undefined;
  }

  const MCP_COLUMN = 520;

  return (
    <>
      <Case label="skill usage · nothing on a shelf, nothing read">
        <Stand
          link={{ kind: "live", city: "sprawling" }}
          unread={[]}
          waiting={[]}
          answers={skills([])}
          children={() => <SkillUsage />}
        />
      </Case>

      <Case label="skill usage · a week: read, audit stale, never read">
        <Stand
          link={{ kind: "live", city: "sprawling" }}
          unread={[]}
          waiting={[]}
          answers={skills(SKILLS_TYPICAL)}
          children={() => <SkillUsage />}
        />
      </Case>

      <Case label="skill usage · long names, a month of reads, many never read">
        <Stand
          link={{ kind: "live", city: "sprawling" }}
          unread={[]}
          waiting={[]}
          answers={skills(SKILLS_LONG)}
          children={() => <SkillUsage />}
        />
      </Case>

      <Case label="mcp usage · no server configured or called" width={MCP_COLUMN}>
        <Stand
          link={{ kind: "live", city: "sprawling" }}
          unread={[]}
          waiting={[]}
          answers={servers([])}
          children={() => <McpUsage />}
        />
      </Case>

      <Case label="mcp usage · a week: two tools called, one server never" width={MCP_COLUMN}>
        <Stand
          link={{ kind: "live", city: "sprawling" }}
          unread={[]}
          waiting={[]}
          answers={servers(SERVERS_TYPICAL)}
          children={() => <McpUsage />}
        />
      </Case>

      <Case label="mcp usage · long names, unconfigured and removed servers" width={MCP_COLUMN}>
        <Stand
          link={{ kind: "live", city: "sprawling" }}
          unread={[]}
          waiting={[]}
          answers={servers(SERVERS_LONG)}
          children={() => <McpUsage />}
        />
      </Case>
    </>
  );
}
