// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The room a person works in: the box they write in, the words a model
// is still saying, the two lists that open over that box, and the
// questions the city is holding for them.
//
// Two of these need room the page gives them and a fixture does not.
// The list that opens over the composer opens *upward*, so the padding
// above it is the room the foot of a page has; the sheet the questions
// sit in is drawn where the page draws it.

import { For } from "solid-js";

import { sendingInto, type Doing, type Sending } from "../../core/doing";
import { EFFORTS } from "../../core/commands";
import { UNSTATED, offered } from "../../core/slash";
import type { ApprovalClass, ApprovalItem } from "../../wire";
import { ApprovalId, Locator, TimeMs } from "../../wire";
import { useSay } from "../../ui";
import { Popover } from "../parts/popover";
import { Composer } from "../talk/composer";
import { WaitingCards } from "../talk/waiting";
import { Case } from "./case";
import { CHOSEN, MODELS } from "./served";

// The postures a run can be in, in the order a dispatch meets them.
const POSTURES: readonly Doing[] = [
  { kind: "thinking" },
  { kind: "calling", tool: "exec", subject: "just check" },
  { kind: "waiting" },
  { kind: "frozen", completion: "done" },
  { kind: "unknown" },
];

// How many characters of a live turn are drawn faint. Kept in step with
// `thread.tsx` by being shown here at several lengths rather than by a
// second copy of the number: what this page is for is deciding whether
// the number is right.
const SAYING = [
  "on",
  "on it — reading the city",
  "on it — reading the city, then writing the plan it asks for",
];

function question(
  id: string,
  actor: string,
  what: string,
  key: readonly [ApprovalClass, string],
  at: number,
  tainted: boolean,
): ApprovalItem {
  return {
    id: ApprovalId.make(id),
    actor,
    action_desc: what,
    artifact: Locator.make(
      "cas:b3-0000000000000000000000000000000000000000000000000000000000000000",
    ),
    cluster_key: { class: key[0], detail: key[1] },
    created: TimeMs.make(at),
    tainted,
  };
}

// The one question the rail's dot counts. Named rather than reached by
// its position in the list below, because the dot and the cards are two
// readings of the same waiting question and a reordered list must not
// make them disagree about which one that is.
export const ONE_QUESTION: ApprovalItem = question(
  "ai_1",
  "lab/east",
  "exec: rm -rf target",
  ["question", "rm"],
  1,
  false,
);

// Three things a person can be asked, in the three shapes the cards
// take: one on its own, several identical ones answered together, and
// one raised by a run that began with somebody else's words - which is
// never grouped with anything.
const WAITING: readonly ApprovalItem[] = [
  ONE_QUESTION,
  question("ai_2", "lab/west", "edit: the building's own rules", ["question", "rules"], 2, false),
  question("ai_3", "lab/west", "edit: the building's own rules", ["question", "rules"], 3, false),
  question(
    "ai_4",
    "hall/mayor",
    "browser: open a page somebody linked",
    ["question", "open"],
    4,
    true,
  ),
];

export function Conversation() {
  const say = useSay();
  return (
    <>
      <For each={POSTURES}>
        {(doing) => (
          <Case label={`${doing.kind} · ${sendingInto(doing)}`}>
            <Composer
              placeholder={say("talk_placeholder_mayor")}
              sending={sendingInto(doing) satisfies Sending}
              hearing={doing.kind === "thinking"}
              onSend={() => false}
              onStop={() => false}
            />
          </Case>
        )}
      </For>

      <For each={SAYING}>
        {(text) => (
          <Case label={`saying · ${String(text.length)}`}>
            <div class="whitespace-pre-wrap leading-relaxed text-body">
              {text.slice(0, -10)}
              <span class="text-text-faint">{text.slice(-10)}</span>
              <span class="ml-tight inline-block h-caret w-hair animate-pulse bg-accent align-text-bottom" />
            </div>
          </Case>
        )}
      </For>

      <Case label="waiting">
        <WaitingCards items={WAITING} />
      </Case>

      <Case label="composer · an empty room opens in the middle">
        <div class="flex flex-col items-center gap-base py-section text-center">
          <p class="text-heading font-heading text-text-disabled">{say("talk_empty_mayor")}</p>
          <p class="text-note text-text-faint">{say("talk_opening_mayor")}</p>
          <div class="w-full">
            <Composer
              placeholder={say("talk_placeholder_mayor")}
              sending="dispatch"
              onSend={() => false}
              onStop={() => false}
            />
          </div>
        </div>
      </Case>

      <Case label="composer · docked once the room has a thread">
        <div class="px-pane pb-pane">
          <Composer
            placeholder={say("talk_placeholder_mayor")}
            sending="dispatch"
            hearing
            onSend={() => false}
            onStop={() => false}
          />
        </div>
      </Case>

      <Menus />
    </>
  );
}

// The two lists that open over the composer. They are their own
// component because each needs the same two layers of padding standing
// in for the page under them.
function Menus() {
  const say = useSay();
  return (
    <>
      <Case label="menu · a line that begins with a slash">
        <div class="pt-palette">
          <div class="pt-output">
            <div class="relative">
              <Popover
                label={say("talk_commands")}
                columns={[
                  {
                    id: "commands",
                    label: say("talk_commands"),
                    items: offered("/").map((each) => ({
                      id: each.spelling,
                      label: each.spelling,
                      hint:
                        each.grammar === ""
                          ? say(each.about)
                          : `${each.grammar} · ${say(each.about)}`,
                    })),
                  },
                ]}
                onApply={() => undefined}
                onClose={() => undefined}
                bind={() => undefined}
              />
            </div>
          </div>
        </div>
      </Case>

      <Case label="selector · model, workspace and effort in one list">
        <div class="pt-palette">
          <div class="pt-output">
            <div class="relative">
              <Popover
                label={say("talk_choose")}
                columns={[
                  {
                    id: "model",
                    label: say("talk_column_model"),
                    items: MODELS.map((each) => ({
                      id: each.id,
                      label: each.id,
                      hint: "zenmux",
                      chosen: each.id === CHOSEN.id,
                    })),
                  },
                  {
                    id: "workspace",
                    label: say("talk_column_workspace"),
                    items: [
                      { id: "hall/mayor", label: "hall/mayor", chosen: true },
                      { id: "lab/east", label: "lab/east" },
                    ],
                  },
                  {
                    id: "effort",
                    label: say("talk_column_effort"),
                    // The column the composer draws: nobody having
                    // chosen leads it and is the row marked, because
                    // that is the state a city nobody has told starts
                    // in, and each level carries what it costs.
                    items: [UNSTATED, ...EFFORTS].map((level) => ({
                      id: level,
                      label: say(`effort_${level}`),
                      hint: say(`effort_note_${level}`),
                      chosen: level === UNSTATED,
                    })),
                  },
                ]}
                onApply={() => undefined}
                onClose={() => undefined}
                bind={() => undefined}
              />
            </div>
          </div>
        </div>
      </Case>
    </>
  );
}
