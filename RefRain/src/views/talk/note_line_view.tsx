// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, For, Show } from "solid-js";
import { fill, say } from "../../core/lang";
import { toFragment } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { clock } from "../../core/time";
import { ui } from "../../ui";
import type { Address, HandbackNote, Note, ReplyEnded, RunId, Turn } from "../../wire";
import { FailureCard } from "./failed_view";
import type { ForkEntry, ForkPlan } from "./forking";
import { kindSaid } from "./inbox";
import { called, residentAt } from "./naming";
import {
  handbackLook,
  named,
  replyWaitLook,
  stamped,
  type Ink,
  type LetterLook,
  type NoteLook,
  type Piece,
} from "./note";
import Person from "./person_view";
import { refusedLook, unreadableLook } from "./refused";
import { ticker } from "./timing";

const INK: Readonly<Record<Ink, string>> = {
  faint: "text-text-faint",
  quiet: "text-text-quiet",
  accent: "text-accent",
  alert: "text-alert",
};

function draw(piece: Piece) {
  return piece.kind === "link" ? (
    <a
      href={piece.href}
      class={[
        "text-text-quiet transition-colors ease-leave hover:text-text hover:ease-arrive",
        piece.face === "mono" && "font-mono underline",
      ]}
    >
      {piece.text}
    </a>
  ) : (
    <span class={INK[piece.ink]}>{piece.text}</span>
  );
}

export function NoteRow(props: NoteLook) {
  return (
    <div class="flex flex-wrap items-baseline gap-snug text-note text-text-faint" role={props.role}>
      <For each={props.pieces} keyed={false}>
        {(piece) => draw(piece())}
      </For>
    </div>
  );
}

export function LetterLookView(props: LetterLook) {
  return (
    <article
      class="mr-auto max-w-5/6 rounded-card border border-edge-panel px-pane py-base"
      aria-label={props.label}
    >
      <div class="mb-tight">
        <NoteRow role={undefined} pieces={props.head} />
      </div>
      <div class="text-body leading-read whitespace-pre-wrap wrap-anywhere">{props.said}</div>
    </article>
  );
}

interface Author {
  readonly from: string | null | undefined;
  readonly session: RunId | null;
  readonly t: number;
}

function Handback(props: Author & { readonly handback: HandbackNote }) {
  const lang = storeValue(ui().lang);
  const address = createMemo(() => residentAt(props.from));
  const look = createMemo(() => {
    const room = address();
    const child = room === null ? (props.from ?? "") : called(room, null, lang());
    const link =
      props.session !== null
        ? toFragment({ kind: "run", run: props.session })
        : room !== null
          ? toFragment({ kind: "talk", address: room })
          : null;
    return handbackLook(lang(), named(child, link), props.handback, clock(lang(), props.t));
  });
  return (
    <div class="my-snug">
      <NoteRow {...look()} />
    </div>
  );
}

export function LetterNote(
  props: Author & { readonly kind: string | null; readonly said: string },
) {
  const lang = storeValue(ui().lang);
  const head = createMemo((): Piece[] => {
    const sender = residentAt(props.from);
    return [
      {
        kind: "words",
        text: props.kind === null ? say(lang(), "talk_letter") : kindSaid(lang(), props.kind),
        ink: "faint",
      },
      ...(sender !== null
        ? [named(called(sender, null, lang()), toFragment({ kind: "talk", address: sender }))]
        : props.from == null
          ? []
          : [named(props.from, null)]),
      ...(props.session === null
        ? []
        : [
            named(
              say(lang(), "talk_letter_session"),
              toFragment({ kind: "run", run: props.session }),
            ),
          ]),
      stamped(clock(lang(), props.t)),
    ];
  });
  return (
    <div class="my-pane">
      <LetterLookView label={say(lang(), "talk_letter")} head={head()} said={props.said} />
    </div>
  );
}

function ReplyWait(props: {
  readonly on: Address;
  readonly until: number;
  readonly ended: ReplyEnded | null | undefined;
}) {
  const u = ui();
  const lang = storeValue(u.lang);
  const tick = storeValue(ticker(u.now));
  const look = createMemo(() =>
    replyWaitLook(
      lang(),
      named(called(props.on, null, lang()), toFragment({ kind: "talk", address: props.on })),
      props.until - tick(),
      props.ended,
    ),
  );
  return (
    <div class="my-snug">
      <NoteRow {...look()} />
    </div>
  );
}

export default function NoteLine(props: {
  readonly note: Note;
  readonly turn: Turn;
  readonly run: RunId;
  readonly onFork?: ((plan: ForkPlan) => void) | undefined;
  readonly onHover: (entry: ForkEntry | null) => void;
}) {
  const lang = storeValue(ui().lang);
  function row(note: Note) {
    if ("arrived" in note) {
      const arrival = note.arrived;
      if (arrival.handback != null)
        return (
          <Handback
            from={arrival.from}
            handback={arrival.handback}
            session={arrival.session ?? null}
            t={arrival.t}
          />
        );
      if (arrival.by === "resident")
        return (
          <LetterNote
            from={arrival.from}
            said={arrival.said ?? ""}
            kind={arrival.kind ?? null}
            session={arrival.session ?? null}
            t={arrival.t}
          />
        );
      return (
        <Person
          text={arrival.said ?? ""}
          label={say(lang(), "talk_you")}
          at={arrival.t}
          entry={{ kind: "message", turn: props.turn, text: arrival.said ?? "" }}
          run={props.run}
          onFork={props.onFork}
          onHover={props.onHover}
        />
      );
    }
    if ("awaiting_reply" in note)
      return (
        <ReplyWait
          on={note.awaiting_reply.on}
          until={note.awaiting_reply.until}
          ended={note.awaiting_reply.ended}
        />
      );
    if ("refused" in note)
      return (
        <div class="my-pane">
          <FailureCard {...refusedLook(lang(), note.refused.error)} />
        </div>
      );
    if ("waiting" in note)
      return (
        <div class="my-snug">
          <NoteRow
            role={undefined}
            pieces={[{ kind: "words", text: say(lang(), "talk_waiting_you"), ink: "alert" }]}
          />
        </div>
      );
    if ("discarded" in note)
      return (
        <div class="my-snug">
          <NoteRow
            role={undefined}
            pieces={[
              {
                kind: "words",
                text: fill(say(lang(), "talk_discarded"), { n: String(note.discarded.count) }),
                ink: "faint",
              },
            ]}
          />
        </div>
      );
    if ("unreadable" in note)
      return (
        <div class="my-pane">
          <FailureCard {...unreadableLook(lang(), note.unreadable.at, note.unreadable.cause)} />
        </div>
      );
    return null;
  }
  return (
    <Show when={props.note} keyed>
      {row}
    </Show>
  );
}
