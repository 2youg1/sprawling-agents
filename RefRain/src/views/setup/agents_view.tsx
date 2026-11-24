// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, onCleanup, Show, untrack } from "solid-js";

import { addAgent } from "../../core/access";
import { readAnswer } from "../../core/answered";
import { QUERIES } from "../../core/asking";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { AgentOffer, AxError, IdemKey } from "../../wire";
import Unanswered from "../parts/unanswered.solid";
import { setupPlacement } from "../settings/hosted";
import { RECEIPT_MS } from "../settings/saving";
import { readingOf } from "./agents";
import AgentsPage from "./agents_page_view";

export default function Agents() {
  const u = ui();
  const asked = storeValue(u.conn.asking.ask(QUERIES.agentCatalog));
  const receipts = storeValue(u.conn.receipts);
  const panel = storeValue(setupPlacement);
  const [text, setText] = createSignal("");
  const [pending, setPending] = createSignal<ReadonlyMap<string, IdemKey>>(new Map());
  const [unknown, setUnknown] = createSignal<ReadonlySet<string>>(new Set());
  const [errors, setErrors] = createSignal<ReadonlyMap<string, AxError>>(new Map());
  const timers = new Map<string, number>();
  const refresh = () => u.conn.asking.refresh(QUERIES.agentCatalog);
  onCleanup(() => {
    for (const timer of timers.values()) window.clearTimeout(timer);
  });
  const read = createMemo(() =>
    readAnswer(asked(), (answer) => ("agent_catalog" in answer ? answer.agent_catalog : undefined)),
  );
  const answer = () => {
    const actual = read();
    return actual.kind === "held" ? actual.value : undefined;
  };
  const missing = () => {
    const actual = read();
    return actual.kind === "unavailable" ? actual : undefined;
  };
  const reading = createMemo(() => readingOf(text(), answer()));
  const parsed = storeValue(() => {
    const held = reading();
    return held.kind === "paste"
      ? u.conn.asking.ask({ parse_agent_spec: { text: held.text } })
      : undefined;
  });
  const offer = () => {
    const actual = parsed();
    return actual !== undefined && "agent_spec" in actual ? actual.agent_spec : null;
  };
  const room = () => {
    const beneath = panel().beneath;
    return beneath.kind === "talk" ? beneath.address : null;
  };

  function retire(id: string): void {
    window.clearTimeout(timers.get(id));
    timers.delete(id);
    setPending((before) => {
      const next = new Map(before);
      next.delete(id);
      return next;
    });
    setUnknown((before) => {
      const next = new Set(before);
      next.delete(id);
      return next;
    });
  }

  createEffect(
    () => ({ actual: answer(), receipts: receipts(), operations: pending() }),
    ({ actual, receipts, operations }) => {
      for (const [id, request] of operations) {
        const reply = receipts.get(request);
        if (reply?.kind === "refused") {
          setErrors((before) => new Map(before).set(id, reply.error));
          retire(id);
        } else if (actual?.added.some((added) => added.id === id) === true) {
          retire(id);
          if (untrack(reading).kind === "paste") setText("");
        }
      }
    },
  );

  function add(offer: AgentOffer, here: boolean): void {
    if (pending().has(offer.id)) return;
    const command = addAgent(offer, here ? room() : null);
    if (!("add_agent" in command) || !u.send(command)) return;
    setErrors((before) => {
      const next = new Map(before);
      next.delete(offer.id);
      return next;
    });
    const id = command.add_agent.idem;
    setPending((before) => new Map(before).set(offer.id, id));
    timers.set(
      offer.id,
      window.setTimeout(() => setUnknown((before) => new Set(before).add(offer.id)), RECEIPT_MS),
    );
    refresh();
  }

  function paste(): void {
    void navigator.clipboard
      .readText()
      .then(setText)
      .catch(() => undefined);
  }

  return (
    <>
      <Show when={answer()}>
        {(held) => (
          <AgentsPage
            answer={held()}
            text={text()}
            reading={reading()}
            parsed={offer()}
            room={room()}
            adding={new Set(pending().keys())}
            unknown={unknown()}
            errors={errors()}
            onText={setText}
            onPaste={paste}
            onAdd={add}
            onCheck={refresh}
          />
        )}
      </Show>
      <Show when={missing()}>
        {(unavailable) => (
          <Unanswered
            query={unavailable().query}
            asked={QUERIES.agentCatalog}
            reason={unavailable().reason}
          />
        )}
      </Show>
    </>
  );
}
