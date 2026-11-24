// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createUniqueId, For, Show } from "solid-js";
import { readAnswer } from "../../core/answered";
import { fill, say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address, ProposalCard } from "../../wire";
import Button from "../parts/button.solid";
import Card from "./proposals_card_view";

export default function Proposals(props: {
  readonly doc: Address;
  readonly unshowable: (card: ProposalCard) => string | undefined;
  readonly onShow: (card: ProposalCard) => void;
}) {
  const u = ui();
  const lang = storeValue(u.lang);
  const id = createUniqueId();
  const asked = storeValue(() => u.conn.asking.ask({ proposals: props.doc }));
  const held = createMemo(() => {
    const found = readAnswer(asked(), (answer) =>
      "proposals" in answer ? answer.proposals : undefined,
    );
    return found.kind === "held" ? found.value : undefined;
  });
  const rows = () => held()?.open ?? [];
  return (
    <Show when={rows().length > 0}>
      <section
        aria-labelledby={id}
        class="flex max-h-[40%] flex-none flex-col gap-snug overflow-y-auto border-b border-edge px-wide py-snug"
      >
        <h3 id={id} class="text-note text-text-quiet">
          {fill(say(lang(), "proposals_title"), { n: String(rows().length) })}
        </h3>
        <For each={rows()} keyed={(card) => card.id}>
          {(card) => {
            const why = createMemo(() => props.unshowable(card()));
            return (
              <Card
                doc={props.doc}
                card={card()}
                version={held()?.version ?? null}
                lead={
                  <div class="flex justify-end">
                    <Button
                      tone="quiet"
                      label={say(lang(), "proposal_show")}
                      {...(why() === undefined ? {} : { why: why() ?? "" })}
                      onPress={() => props.onShow(card())}
                    />
                  </div>
                }
              />
            );
          }}
        </For>
      </section>
    </Show>
  );
}
