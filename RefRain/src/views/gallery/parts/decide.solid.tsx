// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { fill, say } from "../../../core/lang";
import { ui } from "../../../ui";
import Decide from "../../parts/decide_view";
import { GalleryCase } from "../case";
import { storeValue } from "../../../core/solid_store";

export default function DecisionsCases() {
  const lang = storeValue(ui().lang);
  const NOTHING = (): void => undefined;
  return (
    <>
      <GalleryCase label="decide · a resident's question, yes and no">
        <Decide
          kind="question"
          asker={fill(say(lang(), "wait_from"), { actor: "builder" })}
          at="12:03"
          choices={[
            { answer: "yes", label: say(lang(), "wait_allow"), onPress: NOTHING },
            { answer: "no", label: say(lang(), "wait_deny"), onPress: NOTHING },
          ]}
          children={
            <>
              <p class="text-note text-text-faint">{say(lang(), "wait_deny_keeps")}</p>
            </>
          }
        />
      </GalleryCase>
      <GalleryCase label="decide · a door waiting on the person's own hand">
        <Decide
          kind="ask"
          asker={say(lang(), "mailbox_ask_title")}
          at="12:04"
          choices={[{ answer: "no", label: say(lang(), "dismiss"), onPress: NOTHING }]}
          children={
            <>
              <p class="text-note text-text-faint">{say(lang(), "part_why_halted")}</p>
            </>
          }
        />
      </GalleryCase>
      <GalleryCase label="decide · a proposal on a moved base, only rejecting it can be given">
        <Decide
          kind="proposal"
          asker={fill(say(lang(), "proposal_from"), { who: "editor" })}
          at="v3"
          choices={[
            {
              answer: "yes",
              label: say(lang(), "proposal_accept"),
              why: say(lang(), "proposal_stale_why"),
              onPress: NOTHING,
            },
            {
              answer: "edit",
              label: say(lang(), "proposal_edit"),
              why: say(lang(), "proposal_stale_why"),
              onPress: NOTHING,
            },
            { answer: "no", label: say(lang(), "proposal_reject"), onPress: NOTHING },
          ]}
          children={
            <>
              <p class="text-note text-text-faint">
                {fill(say(lang(), "proposal_stale"), { was: "v3", now: "v5" })}
              </p>
            </>
          }
        />
      </GalleryCase>
    </>
  );
}
