<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The building page and its sections (client-SPEC 4-50), standing in a
  // made-up city: a page at the width `<main>` has in a 1440 window, then
  // each section at the width the middle column gives it - the commits
  // with and without messages, the changes since the last checkpoint,
  // the sandbox the building has and the one it inherits, a commit asked
  // for by its oid, and a room whose runs are older than the page.
  //
  // The moments are fixed rather than read off the clock, so a picture
  // of this route compares with yesterday's.
  import type { Answer, BuildingAnswer, CommitAnswer, Entry, GitStatusAnswer, Query, SandboxLimits } from "../../wire";
  import { Address, EnvVarName, GitOid, NodeId, RunId, Seq, ServerLabel, SessionName, TimeMs, UsdMicros } from "../../wire";

  const LAB = Address.make("lab");
  const AT = 1_790_000_000_000;

  // `<main>` at a 1440 window: eleven of the shell's twelve columns.
  export const PAGE_WIDTH = 1259;
  // The building page's middle column at that window: eight columns.
  export const MIDDLE_WIDTH = 915;
  // Its right side at that window: four columns.
  const RIGHT_WIDTH = 443;

  function oid(seed: string): GitOid {
    return GitOid.make(seed.repeat(40).slice(0, 40));
  }

  function run(seed: string): RunId {
    return RunId.make(`${seed.repeat(8).slice(0, 8)}-0000-4000-8000-000000000000`);
  }

  function commit(n: number, message: string | null, actor: string, parents: readonly GitOid[]): CommitAnswer {
    return {
      actor: Address.make(actor),
      at: TimeMs.make(AT - n * 3_600_000),
      effort: n % 2 === 0 ? "high" : null,
      lineage: n === 1 ? [run(String(n)), run("e")] : [run(String(n))],
      message,
      model: "anthropic/claude-fable-5.1",
      oid: oid(String(n)),
      parents: [...parents],
      run: run(String(n)),
      seq: Seq.make(1_000 - n),
      session: n === 3 ? null : SessionName.make(`parser-${String(n)}`),
      spent: UsdMicros.make(n * 137_000),
    };
  }

  export const COMMITS: readonly CommitAnswer[] = [
    commit(1, "parser: read unicode escapes in string literals\n\nThe lexer took \\u as two characters.", "lab/parser", [oid("2")]),
    commit(2, "tests for the escape table", "lab/parser", [oid("3")]),
    commit(3, null, "lab/docs", [oid("4")]),
    commit(4, "first checkpoint of lab", "lab", []),
  ];

  const STATUS: GitStatusAnswer = {
    branch: "main",
    building: LAB,
    checkpoint: COMMITS[0] ?? null,
    drift: { ahead: 2, behind: 0 },
    files: [
      { how: "modified", lines: { counted: { added: 14, removed: 3 } }, path: "src/lexer.rs" },
      { how: "added", lines: { counted: { added: 41, removed: 0 } }, path: "tests/escapes.rs" },
      { how: { renamed: { from: "docs/old.md" } }, lines: { counted: { added: 0, removed: 0 } }, path: "docs/escapes.md" },
      { how: "deleted", lines: "binary", path: "assets/logo.png" },
    ],
  };

  export const SANDBOX: SandboxLimits = {
    shell: true,
    fuel: 4_000_000,
    mounts: [Address.make("lab/shared"), Address.make("docs")],
    env_passthrough: [EnvVarName.make("CC"), EnvVarName.make("LIB")],
    trusted: [ServerLabel.make("desktop")],
  };

  const BUILDING: BuildingAnswer = {
    addr: LAB,
    archive: [],
    blocked: [{ line: "waiting on a reviewer for the escape table", source: NodeId.make("2.1"), waiting: 1 }],
    docs: [],
    mcp: [],
    plan: [
      { item: "Read unicode escapes", leaf: false, needs: [], node: NodeId.make("1"), ready: false, share_ppb: 500_000_000, status: "in_progress" },
      { item: "Lexer takes \\u{...}", leaf: true, needs: [], node: NodeId.make("1.1"), ready: false, share_ppb: 250_000_000, status: "done" },
      { item: "Table of escapes in the docs", leaf: true, needs: [NodeId.make("1.1")], node: NodeId.make("1.2"), ready: true, share_ppb: 250_000_000, status: "not_started" },
      { item: "Review the escape table", leaf: true, needs: [NodeId.make("1")], node: NodeId.make("2.1"), ready: false, share_ppb: 0, status: "blocked" },
    ],
    problems: [],
    progress: { planned: { blocked: 1, blocked_ppb: 0, done: 1, done_ppb: 250_000_000, total: 3 } },
    rooms: ["parser", "docs"],
    sandbox: SANDBOX,
  };

  const ROOT: readonly Entry[] = [
    { kind: "directory", name: "parser" },
    { kind: "directory", name: "docs" },
    { kind: "directory", name: "src" },
    { kind: { file: { bytes: 2_310 } }, name: "Roadmap.md" },
  ];

  const ROOM: readonly Entry[] = [
    { kind: { file: { bytes: 812 } }, name: "JOB.md" },
    { kind: { file: { bytes: 48_211 } }, name: `${run("a")}.jsonl` },
    { kind: { file: { bytes: 12_004 } }, name: `${run("b")}.jsonl` },
  ];

  const FILES: readonly Entry[] = [
    { kind: { file: { bytes: 9_120 } }, name: "lexer.rs" },
    { kind: { file: { bytes: 3_406 } }, name: "escape.rs" },
  ];

  // Which listing a directory answers: the rooms hold transcripts, the
  // source directory holds files.
  function listingOf(at: Address | null | undefined): readonly Entry[] {
    if (at === LAB) return ROOT;
    return at === Address.make("lab/src") ? FILES : ROOM;
  }

  // What this made-up city answers; anything else stays unanswered, the
  // way a page that has asked and not been answered holds it.
  export function answers(query: Query): Answer | undefined {
    if (typeof query === "string") return undefined;
    if ("building_view" in query) return { building: { ...BUILDING, addr: query.building_view.addr } };
    if ("commits" in query) return { commits: { building: LAB, before: null, commits: [...COMMITS], more: false } };
    if ("git_status" in query) return { git_status: STATUS };
    if ("changes" in query) return { changes: { base: query.changes.base, head: query.changes.head ?? null, files: STATUS.files.slice(0, 2) } };
    if ("commit" in query) {
      const held = COMMITS.find((each) => each.oid === query.commit.oid);
      return held === undefined ? { unavailable: { query: "commit" } } : { commit: held };
    }
    if ("cost_of" in query) return { cost_of: { node: query.cost_of.node, spent: UsdMicros.make(312_000), runs: [[run("9"), UsdMicros.make(312_000)]] } };
    if ("listing" in query) return { listing: { at: query.listing.at ?? null, entries: [...listingOf(query.listing.at)] } };
    return undefined;
  }

  export const UNKNOWN_OID = oid("f");
  export const KNOWN_OID = oid("3");
</script>

<script lang="ts">
  import Building from "../building.svelte";
  import Commits from "../building/commits.svelte";
  import Directory from "../building/directory.svelte";
  import Opened from "../building/opened.svelte";
  import Sandbox from "../building/sandbox.svelte";
  import Status from "../building/status.svelte";
  import Whose from "../building/whose.svelte";
  import Case from "./case.svelte";
  import Pages from "./pga_pages.svelte";
  import Stand from "./stand.svelte";
</script>

<Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} {answers}>
  <Case label="building · the page, on the plan" width={PAGE_WIDTH}>
    <Building address={LAB} rank="section" />
  </Case>
  <Case label="building · commits, one without a message" width={MIDDLE_WIDTH}>
    <Commits building={LAB} />
  </Case>
  <Case label="building · changes since the last checkpoint" width={MIDDLE_WIDTH}>
    <Status building={LAB} />
  </Case>
  <Case label="building · a changed file opening on the right" width={RIGHT_WIDTH}>
    <Opened item={{ building: LAB, path: "Roadmap.md", version: null }} />
  </Case>
  <Case label="building · a commit asked for by its oid" width={MIDDLE_WIDTH}>
    <Whose oid={KNOWN_OID} />
  </Case>
  <Case label="building · an oid this city never wrote" width={MIDDLE_WIDTH}>
    <Whose oid={UNKNOWN_OID} />
  </Case>
  <Case label="building · a room whose runs are older than the page" width={MIDDLE_WIDTH}>
    <Directory at={Address.make("lab/parser")} root={LAB} onPick={() => undefined} />
  </Case>
  <Case label="building · its own sandbox" width={MIDDLE_WIDTH}>
    <Sandbox address={LAB} held={SANDBOX} known />
  </Case>
  <Case label="building · the city's sandbox holds" width={MIDDLE_WIDTH}>
    <Sandbox address={LAB} held={null} known />
  </Case>
</Stand>

<Pages />
