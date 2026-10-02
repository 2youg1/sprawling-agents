<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The city, cost and registry pages and the desktop allowlist, each at
  // the width `<main>` has in a 1440 window, standing in a made-up city
  // (client/Spec.lean §4-50): a city with three buildings and one picked, a
  // city that has spent across every cut, and a registry with three
  // assets.
  import type { Answer, CityAnswer, CostAnswer, Query, RegistryLine } from "../../wire";
  import { Address, NodeId, TimeMs, Tokens, UsdMicros } from "../../wire";
  import { PAGE_WIDTH } from "./pga.svelte";

  const AT = 1_790_000_000_000;

  function usd(dollars: number): UsdMicros {
    return UsdMicros.make(Math.round(dollars * 1_000_000));
  }

  const CITY: CityAnswer = {
    active: 2,
    frozen: 31,
    halted: [],
    proved: null,
    pursuits: [{ addr: Address.make("lab"), goal: "read every unicode escape the spec names", state: "running", verdict: { kind: "work", next: NodeId.make("1.2") } }],
    runs: [],
    buildings: [
      { addr: Address.make("hall"), blocked: [], problems: [], progress: { unplanned: { steps: 3, budget: { tokens: Tokens.make(120_000), usd: usd(0.84) } } }, ready: 0 },
      {
        addr: Address.make("lab"),
        blocked: [{ line: "waiting on a reviewer", source: NodeId.make("2.1"), waiting: 1 }],
        problems: [],
        progress: { planned: { blocked: 1, blocked_ppb: 0, done: 1, done_ppb: 250_000_000, total: 3 } },
        ready: 1,
      },
      { addr: Address.make("docs"), blocked: [], problems: [], progress: { planned: { blocked: 0, blocked_ppb: 0, done: 4, done_ppb: 1_000_000_000, total: 4 } }, ready: 0 },
    ],
  };

  const COST: CostAnswer = {
    total: usd(18.42),
    unpriced: { calls: 3, tokens: 41_200 },
    by_run: [["a1f0c2d9", usd(6.1)], ["7be410aa", usd(4.75)], ["c09d33e1", usd(2.2)]],
    by_actor: [["lab/parser", usd(9.4)], ["hall/mayor", usd(5.1)], ["docs/tidy", usd(2.9)]],
    by_segment: [["model", usd(16.9)], ["tools", usd(1.52)]],
    by_tool: [["exec", usd(0.9)], ["read", usd(0.4)], ["edit", usd(0.22)]],
    by_skill: [],
  };

  const ASSETS: readonly RegistryLine[] = [
    { addr: Address.make("lab"), at: TimeMs.make(AT - 3_600_000), kind: "screenshot", subject: "the settings page at 2560 px, forced colours" },
    { addr: Address.make("hall/mayor"), at: TimeMs.make(AT - 7_200_000), kind: "transcript", subject: "what the mayor was asked on the first day" },
    { addr: Address.make("docs"), at: TimeMs.make(AT), kind: "release", subject: "sprawling 0.0.8, windows-x86_64" },
  ];

  // The four page questions this made-up city answers, by name; any
  // other question stays unanswered.
  const ANSWERS: Readonly<Partial<Record<string, Answer>>> = {
    city_view: { city: CITY },
    metrics: { metrics: { approvals_waiting: 1, buildings: 3, discards_outstanding: 0, events: 48_211, runs_active: 2, runs_frozen: 31, signals_waiting: 0 } },
    cost_view: { cost: COST },
    registry_view: { registry: { assets: [...ASSETS] } },
  };

  function answers(query: Query): Answer | undefined {
    return typeof query === "string" ? ANSWERS[query] : undefined;
  }
</script>

<script lang="ts">
  import City from "../city.svelte";
  import Cost from "../cost.svelte";
  import Desktop from "../desktop.svelte";
  import Registry from "../registry.svelte";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";
</script>

<Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} {answers}>
  <Case label="city · the page, on the building table" width={PAGE_WIDTH}>
    <City rank="section" />
  </Case>
  <Case label="cost · the page, five cuts of one total" width={PAGE_WIDTH}>
    <Cost rank="section" />
  </Case>
  <Case label="registry · the page, three assets" width={PAGE_WIDTH}>
    <Registry rank="section" />
  </Case>
  <Case label="desktop · a building with no allowlist yet">
    <Desktop addr={Address.make("lab")} />
  </Case>
</Stand>
