// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createSignal, createUniqueId } from "solid-js";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import RefRain from "../editor/document_view";
import { cardLookOf } from "../editor/proposals_card";
import Card, { ProposalBody as Body } from "../editor/proposals_card_view";
import Proposals from "../editor/proposals_view";
import Letter from "../inspect/letter.solid";
import Mailbox, { MailboxColumn as Column } from "../mailbox/mailbox";
import { GalleryCase } from "./case";
import Stand from "./stand.solid";

import { Address, B3Hash, RunId, Seq, TimeMs } from "../../wire";
import type {
  Answer,
  AxError,
  Call,
  DocumentState,
  EventRecord,
  ProposalCard,
  ProposalsAnswer,
  Query,
  RoundsAnswer,
  Slice,
} from "../../wire";
import { decideProposals } from "../../core/commands";
import { editOf, rejectionOf, retaken } from "../editor/proposals";
import type { Deciding, Edit } from "../editor/proposals";

const SHOP = Address.make("shop");
const PATH = "notes/plan.md";
const DOC = Address.make(`shop/${PATH}`);
const EAST = Address.make("shop/east");
const RUN = RunId.make("0199c0de-0000-4c3d-8e4f-0000000000a1");

const V0 = B3Hash.make("07c41e09".repeat(8));
const V1 = B3Hash.make("3c1f0a9e".repeat(8));

const HEAD = "# Plan\n\n";
const BEFORE = "The reader opens a version. It edits in place. It saves the bytes.";
const TEXT = `${HEAD}${BEFORE}\n\nDrafts are kept for a while.\n`;

function bytes(text: string): number {
  return new TextEncoder().encode(text).length;
}

function slice(kind: Slice["kind"], text: string, trail: string): Slice {
  return { kind, lead: "", text, trail };
}

const CURRENT: ProposalCard = {
  id: B3Hash.make("a1".repeat(32)),
  run: RUN,
  baseline: V1,
  span: { start: bytes(HEAD), end: bytes(HEAD) + bytes(BEFORE) },
  slices: [
    slice("same", "The reader opens a version.", " "),
    slice("delete", "It edits in place.", " "),
    slice("insert", "It edits in the browser and keeps a draft per version.", " "),
    slice("delete", "It saves the bytes.", ""),
    slice("insert", "It saves the bytes of the version it was made on.", ""),
  ],
  offered: Seq.make(9),
};

const STALE: ProposalCard = {
  id: B3Hash.make("b2".repeat(32)),
  run: RUN,
  baseline: V0,
  span: { start: 77, end: 105 },
  slices: [
    slice("delete", "Drafts are kept for a while.", ""),
    slice("insert", "Drafts are kept until the save lands.", ""),
  ],
};

const PROPOSALS: ProposalsAnswer = { doc: DOC, version: V1, open: [CURRENT, STALE] };

const DOCUMENT: DocumentState = {
  held: {
    version: V1,
    format: "markdown",
    bytes: bytes(TEXT),
    body: {
      text: {
        encoding: "utf8",
        coverage: "whole",
        head: { span: { start: 0, end: bytes(TEXT) }, text: TEXT },
      },
    },
  },
};

const NOW = Date.now();

function record(
  seq: number,
  kind: EventRecord["kind"],
  data: Record<string, unknown>,
): EventRecord {
  return {
    addr: EAST,
    data,
    kind,
    prev: B3Hash.make("0".repeat(64)),
    run: RUN,
    seq: Seq.make(seq),
    t: TimeMs.make(NOW - (10 - seq) * 60_000),
    v: 1,
    who: EAST,
  };
}

const RECORDS: readonly EventRecord[] = [
  record(1, "run_started", { task: "tighten the plan's wording" }),
  record(2, "model_called", { model: "claude-opus-4" }),
  record(3, "tool_called", { name: "read", subject: PATH }),
  record(4, "proposal_offered", {
    doc: DOC,
    baseline: V0,
    start: 77,
    end: 105,
    before: "",
    after: "",
  }),
  record(5, "proposal_offered", {
    doc: DOC,
    baseline: V1,
    start: 8,
    end: 75,
    before: "",
    after: "",
  }),
];

const READ: Call = {
  tool: "read",
  subject: PATH,
  arguments: null,
  outcome: "answered",
  at: Seq.make(3),
  output: null,
  called: TimeMs.make(NOW - 8 * 60_000),
  answered: TimeMs.make(NOW - 8 * 60_000 + 40),
  timing: "measured",
};
function round(
  number: number,
  opened: number,
  said: string,
  calls: readonly Call[],
): RoundsAnswer["turns"][number] {
  return {
    calls,
    notes: [],
    number,
    opened: Seq.make(opened),
    said,
    t: TimeMs.make(NOW - (10 - number) * 60_000),
    timing: "measured",
  };
}
const ROUNDS: RoundsAnswer = {
  run: RUN,
  turns: [
    round(1, 2, "I read the plan before changing anything.", [READ]),
    round(2, 6, "The reader section says nothing about drafts; that is the gap.", []),
    round(3, 8, "I offered one card on the reader's three sentences.", []),
    round(4, 12, "The card is waiting for the User; nothing else in the plan needs a change.", []),
  ],
};

const OPEN = [
  { doc: DOC, id: CURRENT.id, at: TimeMs.make(NOW - 5 * 60_000) },
  { doc: DOC, id: STALE.id, at: TimeMs.make(NOW - 6 * 60_000) },
];

const EDITED: Edit = retaken(retaken(editOf(CURRENT), 1, { taken: false }), 2, {
  text: "It edits in the browser, one draft per version.",
});

const SENT: Deciding = { kind: "sent", command: decideProposals(DOC, [rejectionOf(CURRENT)]) };

const REFUSED: AxError = {
  action: "decide proposals",
  code: "E_VERSION_CONFLICT",
  gate: null,
  nearby: [],
  recovery:
    "The plan moved to a new version while this card was open; read the card again before deciding.",
  retry: "no",
  subject: DOC,
};

function answering(query: Query): Answer | undefined {
  if (query === "open_proposals") return { open_proposals: { open: OPEN } };
  if (typeof query !== "object") return undefined;
  if ("rounds" in query) return query.rounds.run === RUN ? { rounds: ROUNDS } : undefined;
  if ("proposals" in query) return query.proposals === DOC ? { proposals: PROPOSALS } : undefined;
  if ("document" in query)
    return query.document.at === DOC ? { document: { at: DOC, state: DOCUMENT } } : undefined;
  if ("sessions" in query)
    return { sessions: { room: query.sessions.room, sessions: [], earlier: 0 } };
  return undefined;
}
const LIVE = { kind: "live", city: "sprawling" } as const;
const ignore = (): void => undefined;
const hands = { retake: ignore };
const current = { kind: "current" } as const;

export default function ProposalsGallery() {
  return (
    <Stand
      link={LIVE}
      unread={[]}
      waiting={[]}
      answers={answering}
      records={RECORDS}
      children={() => <ProposalCases />}
    />
  );
}

function ProposalCases() {
  const lang = storeValue(ui().lang);
  const unshowable = (card: ProposalCard): string | undefined =>
    card.id === STALE.id ? say(lang(), "proposal_show_why") : undefined;
  const [editedHost, setEditedHost] = createSignal<HTMLElement>();
  createEffect(editedHost, (host) => {
    if (host === undefined) return;
    const edit = say(lang(), "proposal_edit");
    [...host.querySelectorAll("button")].find((each) => each.textContent.trim() === edit)?.click();
  });
  return (
    <>
      {" "}
      <GalleryCase label="proposal card · a change to accept, edit or reject">
        <Card doc={DOC} card={CURRENT} version={V1} />
      </GalleryCase>
      <GalleryCase label="proposal card · made on a version the document has left">
        <Card doc={DOC} card={STALE} version={V1} />
      </GalleryCase>
      <GalleryCase label="proposal card · edited before it is accepted">
        <div ref={setEditedHost}>
          <Card doc={DOC} card={CURRENT} version={V1} />
        </div>
      </GalleryCase>
      <GalleryCase label="proposal card body · a removal left out and an inserted sentence rewritten">
        <Body
          {...cardLookOf(
            CURRENT,
            { standing: current, deciding: { kind: "idle" }, edit: EDITED },
            lang(),
            hands,
          )}
        />
      </GalleryCase>
      <GalleryCase label="proposal card body · a decision on its way to the city">
        <Body
          {...cardLookOf(CURRENT, { standing: current, deciding: SENT, edit: null }, lang(), hands)}
        />
      </GalleryCase>
      <GalleryCase label="proposal card body · a decision the city refused, its recovery on the card">
        <Body
          {...cardLookOf(
            CURRENT,
            {
              standing: current,
              deciding: { kind: "refused", error: REFUSED },
              edit: null,
            },
            lang(),
            hands,
          )}
        />
      </GalleryCase>
      <GalleryCase
        label="proposal cards · the band, one card shown in the text and one not"
        width={600}
      >
        <div class="flex h-[640px] flex-col overflow-hidden bg-page">
          <Proposals doc={DOC} unshowable={unshowable} onShow={ignore} />
        </div>
      </GalleryCase>
      <GalleryCase label="proposal cards · above the document in RefRain" width={600}>
        <div class="flex h-[640px] flex-col overflow-hidden bg-page">
          <RefRain building={SHOP} path={PATH} version={null} />
        </div>
      </GalleryCase>
      <GalleryCase label="mailbox · proposal cards in the deciding section" width={440}>
        <div class="flex h-[960px] flex-col bg-raised">
          <Column onClose={ignore} />
        </div>
      </GalleryCase>
      <GalleryCase
        label="letter · a proposal card opened on the right side, read as its diff"
        width={600}
      >
        <div class="flex h-[640px] flex-col overflow-hidden bg-page">
          <Letter doc={DOC} card={CURRENT.id} />
        </div>
      </GalleryCase>
      <GalleryCase
        label="letter · the same card read as the turns around the one that offered it"
        width={600}
      >
        <div class="flex h-[640px] flex-col overflow-hidden bg-page">
          <Letter doc={DOC} card={CURRENT.id} opening="talk" />
        </div>
      </GalleryCase>
      <GalleryCase
        label="letter · a card whose offering line was not found, read as the whole conversation"
        width={600}
      >
        <div class="flex h-[640px] flex-col overflow-hidden bg-page">
          <Letter doc={DOC} card={STALE.id} opening="talk" />
        </div>
      </GalleryCase>
      <GalleryCase
        label="letter · the same card read as the whole text it was written against"
        width={600}
      >
        <div class="flex h-[640px] flex-col overflow-hidden bg-page">
          <Letter doc={DOC} card={CURRENT.id} opening="before" />
        </div>
      </GalleryCase>
      <GalleryCase
        label="letter · the same card read beside the whole text as it stands now"
        width={600}
      >
        <div class="flex h-[640px] flex-col overflow-hidden bg-page">
          <Letter doc={DOC} card={CURRENT.id} opening="now" />
        </div>
      </GalleryCase>
      <GalleryCase label="mailbox key · two proposal cards waiting">
        <Mailbox asked={0} keyId={createUniqueId()} hint={(words) => words} />
      </GalleryCase>
    </>
  );
}
