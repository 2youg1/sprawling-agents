// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import {
  createEffect,
  createMemo,
  createSignal,
  For,
  Match,
  Show,
  Switch,
  untrack,
} from "solid-js";
import { decideProposals } from "../../core/commands";
import { fill, say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address, B3Hash, Command, ProposalCard } from "../../wire";
import type { Choice } from "../parts/decide_model";
import Decide from "../parts/decide_view";
import {
  advance,
  editOf,
  refusesCard,
  rejectionOf,
  retaken,
  standingOf,
  verdictsOf,
  type Deciding,
  type Edit,
  type Happened,
  type Take,
} from "./proposals";
import {
  cardLookOf,
  refusalsOf,
  type CardLook,
  type PieceLook,
  type RowLook,
} from "./proposals_card";
import { short } from "./reading";
import "./proposals_card.css";

export interface ProposalCardViewProps {
  readonly doc: Address;
  readonly card: ProposalCard;
  readonly version: B3Hash | null;
  readonly lead?: JSX.Element;
}

function EditRow(props: { readonly row: RowLook }) {
  const queued = () => (props.row.kind === "kept" ? null : props.row);
  return (
    <li class="flex min-w-0 items-start gap-snug">
      <span class="proposal-box">
        <Show when={queued()}>
          {(row) => (
            <input
              class="size-glyph-sm"
              type="checkbox"
              checked={row().take.checked}
              aria-label={row().take["aria-label"]}
              onChange={(event) => {
                row().take.onchange(event);
              }}
            />
          )}
        </Show>
      </span>
      <Switch>
        <Match when={props.row.kind === "kept" ? props.row : null}>
          {(row) => <span class="proposal-sentence text-text-faint">{row().text}</span>}
        </Match>
        <Match when={props.row.kind === "struck" ? props.row : null}>
          {(row) => (
            <del class={["proposal-sentence proposal-removed", !row().taken && "proposal-untaken"]}>
              {row().text}
            </del>
          )}
        </Match>
        <Match when={props.row.kind === "written" ? props.row : null}>
          {(row) => (
            <textarea
              class={[
                "proposal-sentence proposal-added proposal-field",
                !row().taken && "proposal-untaken",
              ]}
              rows={row().words.rows}
              aria-label={row().words["aria-label"]}
              value={row().words.value}
              onInput={(event) => {
                row().words.oninput(event);
              }}
            />
          )}
        </Match>
      </Switch>
    </li>
  );
}

function DiffPiece(props: { readonly piece: PieceLook }) {
  return (
    <Switch
      fallback={
        <span class="text-text-quiet">
          {props.piece.lead}
          {props.piece.text}
          {props.piece.trail}
        </span>
      }
    >
      <Match when={props.piece.kind === "delete"}>
        {props.piece.lead}
        <del class="proposal-removed">
          <span class="sr-only">{props.piece.said}</span>
          {props.piece.text}
        </del>
        {props.piece.trail}
      </Match>
      <Match when={props.piece.kind === "insert"}>
        {props.piece.lead}
        <ins class={["proposal-added", props.piece.abuts && "proposal-abuts"]}>
          <span class="sr-only">{props.piece.said}</span>
          {props.piece.text}
        </ins>
        {props.piece.trail}
      </Match>
    </Switch>
  );
}

export function ProposalBody(props: CardLook & { readonly lead?: JSX.Element }) {
  const editRows = createMemo(() => (props.body.kind === "edit" ? props.body.rows : null));
  const pieces = createMemo(() => (props.body.kind === "diff" ? props.body.pieces : []));
  return (
    <div class="flex min-w-0 flex-col gap-snug">
      {props.lead}
      <Show when={props.stale}>{(stale) => <p class="text-note text-alert">{stale()}</p>}</Show>
      <p role={props.region.role} class="text-note empty:hidden">
        <Show when={props.status}>
          {(status) => (
            <span class={status().tone === "alert" ? "text-alert" : "text-text-faint"}>
              {status().text}
            </span>
          )}
        </Show>
      </p>
      <Show
        when={editRows()}
        fallback={
          <p class="proposal-run">
            <For each={pieces()} keyed={(piece) => piece.key}>
              {(piece) => <DiffPiece piece={piece()} />}
            </For>
          </p>
        }
      >
        {(rows) => (
          <ol class="flex flex-col gap-snug">
            <For each={rows()} keyed={(row) => row.key}>
              {(row) => <EditRow row={row()} />}
            </For>
          </ol>
        )}
      </Show>
    </div>
  );
}

export default function Card(props: ProposalCardViewProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const connection = storeValue(u.conn.state);
  const [deciding, setDeciding] = createSignal<Deciding>({ kind: "idle" });
  const [edit, setEdit] = createSignal<Edit | null>(null);
  const [editing, setEditing] = createSignal(false);
  const held = createMemo(() => ({
    standing: standingOf(props.card, props.version),
    deciding: deciding(),
    edit: editing() ? edit() : null,
  }));
  const asker = createMemo(() =>
    fill(say(lang(), "proposal_from"), {
      who: belief().runs[props.card.run]?.addr ?? props.card.run.slice(0, 8),
    }),
  );

  function happen(happened: Happened): void {
    setDeciding((current) => advance(current, happened));
  }
  function send(command: Command): void {
    if (u.send(command)) happen({ kind: "sent", command });
  }
  function accept(): void {
    send(
      decideProposals(props.doc, [
        { proposal: props.card.id, verdicts: verdictsOf(props.card, held().edit) },
      ]),
    );
  }
  function reject(): void {
    send(decideProposals(props.doc, [rejectionOf(props.card)]));
  }
  function toggleEditing(): void {
    if (edit() === null) setEdit(editOf(props.card));
    setEditing((open) => !open);
  }
  function retake(place: number, change: Partial<Take>): void {
    setEdit((old) => retaken(old ?? editOf(props.card), place, change));
  }

  const refusals = createMemo(() => refusalsOf(held(), lang()));
  const choices = createMemo<Choice[]>(() => [
    {
      answer: "yes",
      label: say(lang(), editing() ? "proposal_accept_edited" : "proposal_accept"),
      why: refusals().accept,
      onPress: accept,
    },
    {
      answer: "edit",
      label: say(lang(), editing() ? "proposal_back" : "proposal_edit"),
      why: refusals().edit,
      onPress: toggleEditing,
    },
    {
      answer: "no",
      label: say(lang(), "proposal_reject"),
      why: refusals().reject,
      onPress: reject,
    },
  ]);
  const look = createMemo(() => cardLookOf(props.card, held(), lang(), { retake }));

  let last = untrack(() => belief().refusal);
  createEffect(
    () => belief().refusal,
    (error) => {
      if (error === null || error === last) return;
      last = error;
      const state = deciding();
      if (state.kind !== "sent" && state.kind !== "pending") return;
      const matches =
        error.request == null
          ? refusesCard(error, props.doc, props.card.id)
          : "decide_proposals" in state.command &&
            error.request === state.command.decide_proposals.idem;
      if (!matches) return;
      happen({ kind: "refusal", error });
      if (error.code === "E_VERSION_CONFLICT") u.conn.asking.refresh({ proposals: props.doc });
    },
  );
  let live = untrack(() => connection().kind === "live");
  createEffect(
    () => connection().kind === "live",
    (now) => {
      if (now === live) return;
      live = now;
      if (!now) {
        happen({ kind: "lost" });
        return;
      }
      const state = deciding();
      if (state.kind === "pending" && u.conn.command(state.command)) happen({ kind: "relinked" });
    },
  );

  return (
    <Decide
      kind="proposal"
      asker={asker()}
      at={fill(say(lang(), "proposal_on"), { version: short(props.card.baseline) })}
      choices={choices()}
    >
      <ProposalBody {...look()} lead={props.lead} />
    </Decide>
  );
}
