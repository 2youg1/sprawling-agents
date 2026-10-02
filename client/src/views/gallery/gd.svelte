<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts" module>
  // Keys and search (client/Spec.lean §4-62): the file finder's content with a
  // building's files found shallow first, the same with a walk the city
  // cut short, and tool lines drawing their keys at the row's end. The
  // finder's `<dialog>` is not mounted here: a modal covers every other
  // specimen on this route, so the fixture draws what the dialog holds.
  // A line draws its keys while it holds the focus or while the right
  // side shows its call; neither can stand on this route without moving
  // the focus or opening the right side of every other fixture, so the
  // keys of the lines here are drawn as they are then.

  import { Address, RunId, Seq, TimeMs } from "../../wire";
  import type { Answer, Call, FindAnswer, Query } from "../../wire";

  const LAB = Address.make("lab");
  const RUN = RunId.make("0199c0de-7700-4000-8000-0000000000d4");

  const FOUND: readonly Address[] = [
    "Roadmap.md",
    "notes/roadmap-review.md",
    "notes/2026/roadmap-q3.md",
    "docs/archive/old-roadmap.txt",
  ].map((path) => Address.make(path));

  function finding(walked: FindAnswer["walked"]): (query: Query) => Answer | undefined {
    return (query) =>
      typeof query === "object" && "find" in query
        ? { find: { under: query.find.under, text: query.find.text, paths: [...FOUND], walked } }
        : undefined;
  }

  function call(at: number, tool: string, subject: string, took: number): Call {
    const called = 1_790_000_000_000 + at * 1000;
    return {
      tool,
      subject,
      arguments: null,
      outcome: "answered",
      at: Seq.make(at),
      output: null,
      called: TimeMs.make(called),
      answered: TimeMs.make(called + took),
      timing: "measured",
      effect: "read",
      render: "generic",
    };
  }

  const CALLS: readonly Call[] = [
    call(41, "read", "crates/city/src/document.rs", 31),
    call(42, "search", "edit_against", 140),
    call(43, "exec", "cargo nextest -p city", 3_412),
  ];
</script>

<script lang="ts">
  import CallLine from "../talk/call_line.svelte";
  import Search from "../finder/search.svelte";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";
</script>

{#each ["whole", "cut"] as const as walked (walked)}
  <Case label={`file finder · lab · walked ${walked}`} width={640}>
    <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={finding(walked)}>
      <div class="flex flex-col gap-snug rounded-panel bg-raised p-snug">
        <Search under={LAB} titleId={`gd-finder-${walked}`} onClose={() => undefined} />
      </div>
    </Stand>
  </Case>
{/each}

<Case label="tool lines · keys at the row's end">
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]}>
    <div data-thread class="flex flex-col [&_[data-line-keys]]:inline-flex">
      {#each CALLS as each (each.at)}
        <CallLine call={each} run={RUN} />
      {/each}
    </div>
  </Stand>
</Case>
