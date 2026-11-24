// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { CheatsheetLook } from "../parts/cheatsheet";
import Inked from "../parts/inked.solid";
import ModalSheet from "../parts/modal_sheet";
import Page from "../parts/page.solid";
import Unanswered from "../parts/unanswered.look.solid";
import Uses from "../parts/usage_uses.solid";
import { GalleryCase } from "./case";
const still = (): void => undefined;
const WORDS = {
  held: "The words this frame holds.",
  page: "Checkout",
  note: "What the checkout building keeps.",
  section: "Recent runs",
  sheet: "sheet",
};
const CODE = {
  rust: "// the total\nlet total = items.iter().sum::<u64>() + 12;",
  markdown: "# Plan\n\nFix the `total`.",
};
const DAY = 86_400_000;
const AT = Date.UTC(2026, 9, 6, 9, 30);

const SHORTCUTS = [
  { key: "palette", label: "palette", marks: ["Ctrl", "/"] },
  { key: "tier.cycle", label: "layers", marks: ["Ctrl", "\\"] },
] as const;

function Body() {
  return <p class="text-note text-text-faint">{WORDS.held}</p>;
}

export default function PartsLookCases() {
  const lang = storeValue(ui().lang);
  return (
    <>
      <GalleryCase label="page frame · a section with its note, and a section inside it">
        <Page title={WORDS.page} rank="section" note={WORDS.note}>
          <Page title={WORDS.section} rank="section">
            <Body />
          </Page>
        </Page>
      </GalleryCase>

      <GalleryCase label="sheets · a sheet in the flow, and the shortcut sheet drawn as a specimen">
        <div class="flex flex-col gap-base">
          <ModalSheet open stands="in-flow" label={WORDS.sheet} onCancel={still}>
            <Body />
          </ModalSheet>
          <CheatsheetLook
            seat="specimen"
            title={say(lang(), "keys_title")}
            dismiss={say(lang(), "dismiss")}
            where={say(lang(), "keys_where")}
            rows={SHORTCUTS}
            onClose={still}
          />
        </div>
      </GalleryCase>

      <GalleryCase label="code · inked Rust and Markdown">
        <div class="flex flex-col gap-base font-mono text-note">
          <Inked text={CODE.rust} source="total.rs" />
          <Inked text={CODE.markdown} source="Plan.md" />
        </div>
      </GalleryCase>

      <GalleryCase label="skill uses · never used, and used by two runs">
        <div class="flex flex-col gap-base">
          <Uses uses={[]} days={[]} />
          <Uses
            uses={[
              { run: "7f3a", resident: "checkout", at: AT, part: null, outcome: "ok" },
              {
                run: "8d0c",
                resident: null,
                at: AT - DAY,
                part: "release-notes",
                outcome: "failed",
              },
            ]}
            days={[
              { day: "2026-10-05", count: 1 },
              { day: "2026-10-06", count: 1 },
            ]}
          />
        </div>
      </GalleryCase>

      <GalleryCase label="unanswered · the question named, with the city's reason and a retry">
        <Unanswered
          said="The city did not answer the list of buildings."
          reason="the ledger is being compacted"
          retry={{ label: "ask again", press: still }}
        />
      </GalleryCase>
    </>
  );
}
