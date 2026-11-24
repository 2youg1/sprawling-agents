// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { Delivered } from "../talk/delivery_view";
import { FailureCard as Card } from "../talk/failed_view";
import Head from "../talk/head_view";
import { named, replyWaitLook } from "../talk/note";
import { LetterNote, NoteRow } from "../talk/note_line_view";
import { failedLook } from "../talk/refused";
import Saying from "../talk/saying_view";
import TurnLine from "../talk/turn";
import { GalleryCase } from "./case";

import type { AxError, Note, Turn } from "../../wire";
import { Address, RunId, Seq, TimeMs } from "../../wire";

const RUN = RunId.make("0199c0de-0000-4000-8000-0000000a0be5");
const CHILD = RunId.make("0199c0de-0000-4000-8000-0000000c41d0");
const AT = TimeMs.make(1_767_000_000_000);

const OVERFLOW: AxError = {
  action: "call the model",
  code: "E_PROVIDER",
  nearby: [],
  provider: { kind: "overflow", status: 400 },
  recovery: "the request is longer than the model's context window; start a new session with /new",
  retry: "no",
  subject: "gallery/model",
};

const BUSY: AxError = {
  action: "write src/lib.rs",
  code: "E_WORKTREE_BUSY",
  nearby: [],
  recovery: "another run holds this worktree; wait for it to finish",
  retry: "yes",
  subject: "shop/main",
};

const NOTES: readonly Note[] = [
  { arrived: { at: Seq.make(2), by: "user", said: "Keep the old name as an alias.", t: AT } },
  { refused: { at: Seq.make(3), error: OVERFLOW } },
  { refused: { at: Seq.make(4), error: BUSY } },
  { waiting: { at: Seq.make(5), t: AT } },
  { discarded: { at: Seq.make(6), count: 3 } },
  { unreadable: { at: Seq.make(7), cause: "the record names a frame this build does not know" } },
  {
    arrived: {
      at: Seq.make(8),
      by: "resident",
      from: "shop/tests",
      handback: { finished: { verified_by: "cargo nextest" } },
      session: CHILD,
      t: AT,
    },
  },
  {
    arrived: {
      at: Seq.make(9),
      by: "resident",
      from: "shop/docs",
      handback: { stopped: { because: "the plan was refused" } },
      t: AT,
    },
  },
  {
    awaiting_reply: {
      at: Seq.make(10),
      ended: { by: "timeout", t: AT },
      on: Address.make("shop/tests"),
      t: AT,
      until: AT,
    },
  },
  {
    arrived: {
      at: Seq.make(11),
      by: "resident",
      from: "shop/tests",
      kind: "thread",
      said: "The alias is covered; two tests read it.",
      session: CHILD,
      t: AT,
    },
  },
];

const TURN: Turn = {
  calls: [],
  notes: NOTES,
  number: 1,
  opened: Seq.make(1),
  said: "The rename is done, and the old name stays as an alias.",
  t: AT,
  timing: "measured",
};

export default function AgentNotes() {
  const lang = storeValue(ui().lang);
  const ignore = (): void => undefined;
  return (
    <>
      <GalleryCase
        label="agent messages · every note a turn comes to, a line or a shape apart"
        width={760}
      >
        <div class="px-wide">
          <TurnLine
            turn={TURN}
            run={RUN}
            who="shop/main"
            model={null}
            live="frozen"
            doing={undefined}
            showEmpty={false}
            ceiling={null}
            whole
            onHover={ignore}
          />
        </div>
      </GalleryCase>

      <GalleryCase
        label="agent messages · a reply still arriving with motion off, the caret lit"
        width={760}
      >
        <div class="px-wide" data-motion="off">
          <Saying
            who="shop/main"
            text="The rename is done. Next I will read the two callers that still"
          />
        </div>
      </GalleryCase>

      <GalleryCase
        label="agent messages · a head, a letter, a wait, an echo and a failed message, seated alone"
        width={760}
      >
        <div class="flex flex-col gap-snug px-wide">
          <Head
            who="shop/main"
            at={AT}
            model="gallery/model"
            ttft={{ unit: "ms", n: 412 }}
            tps={61}
            rhythm={null}
          />
          <LetterNote
            from="shop/tests"
            said="Two tests read the alias."
            kind="mention"
            session={CHILD}
            t={AT}
          />
          <NoteRow {...replyWaitLook(lang(), named("shop/tests", null), 95_000, null)} />
          <Delivered
            delivery={{
              kind: "unknown",
              words: "Keep the old name as an alias.",
              refusal: null,
              mark: 0,
            }}
          />
          <Card
            {...failedLook(lang(), say(lang(), "talk_failed_model"), BUSY, {
              settings: "offered",
              onRetry: ignore,
            })}
          />
        </div>
      </GalleryCase>
    </>
  );
}
