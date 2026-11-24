// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, For, Show } from "solid-js";
import { readAnswer } from "../../core/answered";
import { say } from "../../core/lang";
import { buildingOf } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { ago } from "../../core/time";
import { ui } from "../../ui";
import type { Address, B3Hash, TimeMs } from "../../wire";
import Card from "../editor/proposals_card_view";
import { openDocument, openLetter } from "../inspect/open";
import Button from "../parts/button.solid";
import Decide from "../parts/decide_view";
import { recoveryWords } from "../parts/notice_title";
import Waiting from "../talk/waiting_view";
import { documentsOf, openCards } from "./deciding_proposals";
import NoticeEntry from "./notice_entry";
import { stoppedOf, WHY } from "./pending";
import { returnTo } from "./returning";
import Section from "./section";
import { noticeSweeps, shown, sweep } from "./swept";

function Document(props: {
  readonly doc: Address;
  readonly at: TimeMs | null;
  readonly onLeave: () => void;
}) {
  const u = ui();
  const lang = storeValue(u.lang);
  const asked = storeValue(() => u.conn.asking.ask({ proposals: props.doc }));
  const proposals = createMemo(() => {
    const found = readAnswer(asked(), (answer) =>
      "proposals" in answer ? answer.proposals : undefined,
    );
    return found.kind === "held" ? found.value : null;
  });
  function letter(card: B3Hash): void {
    openLetter({ doc: props.doc, card }, () => {
      returnTo(card);
    });
    props.onLeave();
  }
  function file(): void {
    const building = buildingOf(props.doc);
    openDocument({ building, path: props.doc.slice(building.length + 1), version: null });
    props.onLeave();
  }
  return (
    <Show when={proposals()}>
      {(found) => (
        <For each={found().open} keyed={(card) => card.id}>
          {(card) => (
            <div class="my-base">
              <Card
                doc={props.doc}
                card={card()}
                version={found().version ?? null}
                lead={
                  <>
                    <p class="text-note text-text-quiet">{say(lang(), WHY.proposal)}</p>
                    <p class="font-mono text-note wrap-anywhere text-text-quiet">{props.doc}</p>
                    <div class="flex min-w-0 items-center gap-snug">
                      <span class="min-w-0 flex-1 text-note text-text-faint">
                        {props.at === null ? "" : ago(lang(), props.at, u.now())}
                      </span>
                      <span data-letter={card().id} class="contents">
                        <Button
                          tone="quiet"
                          label={say(lang(), "letter_open")}
                          onPress={() => letter(card().id)}
                        />
                      </span>
                      <Button tone="quiet" label={say(lang(), "proposal_open")} onPress={file} />
                    </div>
                  </>
                }
              />
            </div>
          )}
        </For>
      )}
    </Show>
  );
}

export default function Deciding(props: { readonly onLeave: () => void }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const approvals = storeValue(u.approvals);
  const sweeps = storeValue(noticeSweeps);
  const cards = storeValue(openCards(u.conn.asking));
  const documents = createMemo(() => documentsOf(cards()));
  const stopped = createMemo(() =>
    stoppedOf(belief().notices.filter((notice) => shown(notice, sweeps()))),
  );
  const questions = createMemo(() => approvals() ?? []);
  const count = createMemo(
    () => stopped().asks.length + stopped().failures.length + questions().length + cards().length,
  );
  return (
    <Section
      title={say(lang(), "mailbox_deciding")}
      empty={say(lang(), "mailbox_deciding_none")}
      count={count()}
    >
      <For each={stopped().asks} keyed={(notice) => notice.key}>
        {(notice) => (
          <div class="my-base">
            <Decide
              kind="ask"
              asker={say(lang(), "mailbox_ask_title")}
              at={ago(lang(), notice().at, u.now())}
              choices={[
                {
                  answer: "no",
                  label: say(lang(), "dismiss"),
                  onPress: () => {
                    sweep([notice()]);
                  },
                },
              ]}
            >
              <p class="text-body">{notice().error.action}</p>
              <p class="font-mono text-note wrap-anywhere text-text-quiet">
                {notice().error.subject}
              </p>
              <p class="mt-snug text-note text-text">
                {recoveryWords(lang(), notice().error.code, notice().error.recovery)}
              </p>
              <p class="mt-snug text-note text-text-quiet">{say(lang(), WHY.ask)}</p>
            </Decide>
          </div>
        )}
      </For>
      <For each={stopped().failures} keyed={(notice) => notice.key}>
        {(notice) => <NoticeEntry notice={notice()} blocking />}
      </For>
      <Show when={questions().length > 0}>
        <p class="mt-base text-note text-text-quiet">{say(lang(), WHY.question)}</p>
      </Show>
      <Waiting items={questions()} />
      <For each={documents()} keyed={(offered) => offered.doc}>
        {(offered) => <Document doc={offered().doc} at={offered().at} onLeave={props.onLeave} />}
      </For>
    </Section>
  );
}
