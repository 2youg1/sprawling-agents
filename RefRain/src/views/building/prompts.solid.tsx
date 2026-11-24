// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, Match, Switch } from "solid-js";
import { readAnswer } from "../../core/answered";
import { heldIn, heldWithin } from "../../core/belief/rooms";
import { putPrompt } from "../../core/commands";
import { fill, say } from "../../core/lang";
import { toFragment } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { Address as AddressSchema, type Address, type PromptStep, type Query } from "../../wire";
import { openDocument } from "../inspect/open";
import Dialog from "../parts/dialog_view";
import Unanswered from "../parts/unanswered.solid";
import {
  actionName,
  actionWord,
  cardsOf,
  slotsOf,
  type CardAction,
  type CardLook,
  type PromptsLook,
} from "./prompts";
import Look from "./prompts.look.solid";

interface Props {
  readonly building: Address;
  readonly rooms: readonly string[];
}

export default function Prompts(props: Props) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const [picked, setPicked] = createSignal<Address | null>(null);
  const [asking, setAsking] = createSignal<{
    readonly card: CardLook;
    readonly step: PromptStep;
  } | null>(null);
  const room = createMemo(
    () => picked() ?? heldWithin(belief(), props.building).at(-1)?.addr ?? props.building,
  );
  const last = createMemo(() => heldIn(belief(), room()).at(-1)?.run);
  const layersAsked = createMemo<Query>(() => ({ layers: { addr: room() } }));
  const layers = storeValue(() => u.conn.asking.ask(layersAsked()));
  const read = createMemo(() =>
    readAnswer(layers(), (answer) => ("layers" in answer ? answer.layers : undefined)),
  );
  const unavailable = () => {
    const answer = read();
    return answer.kind === "unavailable" ? answer : undefined;
  };
  const sent = storeValue(() => {
    const run = last();
    return run === undefined ? undefined : u.conn.asking.ask({ prefix: { run } });
  });

  function act(card: CardLook, action: CardAction): void {
    if (action.kind === "open") {
      openDocument({ building: props.building, path: action.path, version: null });
    } else if (action.step === "own") {
      u.send(putPrompt(card.at, card.base, action.step));
    } else {
      setAsking({ card, step: action.step });
    }
  }

  const choices = () =>
    [
      props.building,
      ...props.rooms.map((name) => AddressSchema.make(`${props.building}/${name}`)),
    ].map((each) => ({
      label: each,
      current: each === room(),
      onPress: () => {
        setPicked(each);
      },
    }));
  const look = createMemo((): PromptsLook | null => {
    const answer = read();
    if (answer.kind !== "held") return null;
    const prefix = sent();
    const run = last();
    const l = lang();
    return {
      rooms: choices(),
      roomsLabel: say(l, "prompts_rooms"),
      heading: fill(say(l, run === undefined ? "prompts_no_run" : "prompts_last_run"), {
        room: room(),
      }),
      slots: prefix !== undefined && "prefix" in prefix ? slotsOf(prefix.prefix, l) : [],
      run:
        run === undefined
          ? undefined
          : {
              label: say(l, "prompts_open_run"),
              href: toFragment({ kind: "run", run, lens: "prompt" }),
            },
      note: say(l, "prompts_next_run"),
      documentsLabel: say(l, "prompts_documents"),
      cards: cardsOf(answer.value.documents, props.building, l),
      actionLabel: (action) => actionWord(action, l),
      actionName: (card, action) => actionName(card, action, l),
      onAction: act,
    };
  });

  const dialog = () => {
    const held = asking();
    if (held === null) return { title: "", detail: "", confirmLabel: "" };
    const inherits = held.step === "inherit";
    return {
      title: fill(say(lang(), inherits ? "prompts_inherit_title" : "prompts_restore_title"), {
        at: held.card.at,
      }),
      detail: say(lang(), inherits ? "prompts_inherit_detail" : "prompts_restore_detail"),
      confirmLabel: actionWord(held.step, lang()),
    };
  };
  return (
    <>
      <Switch fallback={<p class="text-text-faint">…</p>}>
        <Match when={look()}>{(view) => <Look {...view()} />}</Match>
        <Match when={unavailable()}>
          {(answer) => <Unanswered query={answer().query} asked={layersAsked()} />}
        </Match>
      </Switch>
      <Dialog
        open={asking() !== null}
        {...dialog()}
        cancelLabel={say(lang(), "part_cancel")}
        destructive
        onConfirm={() => {
          const held = asking();
          setAsking(null);
          if (held !== null) u.send(putPrompt(held.card.at, held.card.base, held.step));
        }}
        onCancel={() => {
          setAsking(null);
        }}
      />
    </>
  );
}
