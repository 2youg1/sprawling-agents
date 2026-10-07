<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts" module>
  // The settings panel's content - the tree and one group beside it - at
  // the panel's own width, in the states the new groups have: both
  // identity cards read, an identity area that does not read, a
  // building's rules, the city's automation, the run group, and the
  // accounts group, which holds the city's own layer beside the default
  // model (client D52), and the colour page at both widths, drawn with
  // the kept override of whoever opens the gallery. The performance
  // group, and the tree opened over the performance page (its entry
  // named as the page beneath, with the process reading), close the
  // tree's states. The panel's `<dialog>` is not mounted here: a
  // modal covers every other specimen on this route, so the fixture
  // draws what the dialog holds (`views/settings/sheet.svelte`).

  import { Address, B3Hash } from "../../wire";
  import type { Answer, DocumentAnswer, IdentityAnswer, Query } from "../../wire";
  import { CITY_CONFIG, rulesAt } from "../settings/files";
  import { HALL } from "../shared/buildings";
  import { SEARCH } from "./configured";

  const VERSION = B3Hash.make("7f3a9c0e21d4b5a6978812ccde0f13a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0");

  function text(at: Address, body: string): DocumentAnswer {
    return {
      at,
      state: {
        held: {
          body: { text: { coverage: "whole", encoding: "utf8", head: { span: { start: 0, end: body.length }, text: body } } },
          bytes: body.length,
          format: "plain",
          version: VERSION,
        },
      },
    };
  }

  const STATED: IdentityAnswer = {
    stated: {
      about: "Works in Rust and Lean; prefers a short answer with the evidence first.",
      imported_from: "github.com",
      mayor: "Cat",
      mayor_text: "+++\nname = \"Cat\"\n+++\nPlans the city.\n",
      preferences_text: "+++\nuser_id = \"river-ops\"\nimported_from = \"github.com\"\n+++\nWorks in Rust and Lean.\n",
      user_id: "river-ops",
      version: "b3:5d1e",
    },
  };

  const UNREADABLE: IdentityAnswer = {
    unreadable: { document: "mayor", line: 2, why: "name must be one line of text" },
  };

  const RULES = '[write]\nallow = ["src/**", "tests/**"]\n\n[gate.exec]\nask = ["cargo publish", "git push"]\n';
  const CONFIG = '[model]\neffort = "high"\n\n[cache]\nkeep_warm = "five_minute"\n';

  function city(identity: IdentityAnswer) {
    return (query: Query): Answer | undefined => {
      if (query === "identity") return { identity };
      if (query === "preferences") return { preferences: { core: { placement: "soft_shares", priority: "normal", memory_bytes: null } } };
      if (query === "automation") {
        return {
          automation: {
            jobs: [
              { addr: HALL, cadence: { daily_at: { minute: 540 } }, goal: "", name: "morning-digest", task: "Summarise what landed overnight." },
              { addr: Address.make("lab"), cadence: { every_minutes: { minutes: 30 } }, goal: "", name: "flaky-watch", task: "Rerun the tests that failed once." },
              { addr: Address.make("lab"), cadence: { weekly_at: { minute: 4 * 1440 + 960 } }, goal: "", name: "friday-release", task: "Draft the release notes." },
            ],
            sources: [{ addr: Address.make("lab"), matches: "ci/failed/**", name: "ci-failures", starts_work: true }],
            unreadable: [],
          },
        };
      }
      if (typeof query === "object" && "config" in query) {
        return { config: { addr: HALL, effort: { effort: "high", from: "city" }, second: { domain: { max: 90, min: 31 }, from: "default", percent: 65 }, tuning: { from: "default", proxying: "except_local", timeout_ms: 600_000, account_retries: "two" }, search: SEARCH } };
      }
      if (typeof query === "object" && "document" in query) {
        if (query.document.at === CITY_CONFIG) return { document: text(CITY_CONFIG, CONFIG) };
        if (query.document.at === rulesAt(HALL)) return { document: text(rulesAt(HALL), RULES) };
      }
      return undefined;
    };
  }
</script>

<script lang="ts">
  import Sheet from "../settings/sheet.svelte";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";

  const CITY = { kind: "city" } as const;
  const MONITOR = { kind: "monitor" } as const;
  const CASES = [
    ["you", STATED, 1040, CITY],
    ["you", UNREADABLE, 1040, CITY],
    ["rules", STATED, 1040, CITY],
    ["automation", STATED, 1040, CITY],
    ["run", STATED, 1040, CITY],
    ["accounts", STATED, 1040, CITY],
    ["performance", STATED, 1040, CITY],
    ["performance", STATED, 1040, MONITOR],
    ["colours", STATED, 1040, CITY],
    ["colours", STATED, 390, CITY],
    ["you", STATED, 390, CITY],
  ] as const;
</script>

{#each CASES as [group, identity, width, beneath], index (index)}
  <Case
    label={`settings panel · ${group}${identity === UNREADABLE ? " · unreadable" : ""}${beneath === MONITOR ? " · over the monitor" : ""} at ${String(width)}`}
    {width}
  >
    <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={city(identity)}>
      <Sheet {group} {beneath} onPick={() => undefined} onClose={() => undefined} titleId={`set-fixture-${String(index)}`} />
    </Stand>
  </Case>
{/each}
