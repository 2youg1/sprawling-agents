<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The shared parts no other fixture imports: the page frame, a sheet
  // and the shortcut sheet drawn in the flow, inked code, the uses of a
  // skill, and a question the city did not answer. Each is drawn from its
  // seat where the seat takes plain values, and from its look where the
  // seat would need a city to ask.
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Inked from "../parts/inked.svelte";
  import KbdSheet from "../parts/kbd_sheet.look.svelte";
  import { sheetOf } from "../parts/kbd";
  import Page from "../parts/page.svelte";
  import Sheet from "../parts/sheet.look.svelte";
  import Unanswered from "../parts/unanswered.look.svelte";
  import Uses from "../parts/usage_uses.svelte";
  import Case from "./case.svelte";

  const { lang } = ui();
  const still = (): void => undefined;
  // Fixture data: what a city or a person would have written.
  const WORDS = {
    held: "The words this frame holds.",
    page: "Checkout",
    note: "What the checkout building keeps.",
    section: "Recent runs",
  };
  const kept = (): (() => void) => still;
  const CODE = {
    rust: "// the total\nlet total = items.iter().sum::<u64>() + 12;",
    markdown: "# Plan\n\nFix the `total`.",
  };
  const DAY = 86_400_000;
  const AT = Date.UTC(2026, 9, 6, 9, 30);
</script>

{#snippet body()}
  <p class="text-note text-text-faint">{WORDS.held}</p>
{/snippet}

<Case label="page frame · a section with its note, and a section inside it">
  <Page title={WORDS.page} rank="section" note={WORDS.note}>
    <Page title={WORDS.section} rank="section" children={body} />
  </Page>
</Case>

<Case label="sheets · a sheet in the flow, and the shortcut sheet drawn as a specimen">
  <div class="flex flex-col gap-base">
    <Sheet wire={{ open: true, "aria-label": "sheet" }} stands="in-flow" children={body} />
    <KbdSheet
      {...sheetOf(
        { uid: "fixture-kbd", seat: "specimen", onClose: still, hold: kept },
        {
          title: say($lang, "keys_title"),
          dismiss: say($lang, "dismiss"),
          where: say($lang, "keys_where"),
          rows: [
            { key: "palette", label: "palette", marks: ["Ctrl", "K"] },
            { key: "finder", label: "find a file", marks: ["Ctrl", "P"] },
          ],
        },
      )}
    />
  </div>
</Case>

<Case label="code · inked Rust and Markdown">
  <div class="flex flex-col gap-base font-mono text-note">
    <Inked text={CODE.rust} source="total.rs" />
    <Inked text={CODE.markdown} source="Plan.md" />
  </div>
</Case>

<Case label="skill uses · never used, and used by two runs">
  <div class="flex flex-col gap-base">
    <Uses uses={[]} days={[]} />
    <Uses
      uses={[
        { run: "7f3a", resident: "checkout", at: AT, part: null, outcome: "ok" },
        { run: "8d0c", resident: null, at: AT - DAY, part: "release-notes", outcome: "failed" },
      ]}
      days={[
        { day: "2026-10-05", count: 1 },
        { day: "2026-10-06", count: 1 },
      ]}
    />
  </div>
</Case>

<Case label="unanswered · the question named, with the city's reason and a retry">
  <Unanswered
    said="The city did not answer the list of buildings."
    reason="the ledger is being compacted"
    retry={{ label: "ask again", press: still }}
  />
</Case>
