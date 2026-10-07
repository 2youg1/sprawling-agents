// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The card's chords against client/spec/Views/Parts/Decide.lean: a
// chord answers only what the card offers and only while it can be
// given (property 4), over every card a caller can hand in - each of the
// three answers absent, offered, or offered with a reason it cannot be
// given - and every action a chord can reach. That the three chords hold
// the accelerator (property 3) is the key table's, judged in
// `core/keys.test.ts`.

import { describe, expect, test } from "bun:test";

import { ACTIONS, type Action } from "../../core/keys";
import { answering, KEY, lookOf, ordered, type Answer, type Choice } from "./decide";

const ANSWERS: readonly Answer[] = ["yes", "edit", "no"];

type Offer = "absent" | "offered" | "refused";
const OFFERS: readonly Offer[] = ["absent", "offered", "refused"];

function choiceOf(answer: Answer, offer: Offer): readonly Choice[] {
  switch (offer) {
    case "absent":
      return [];
    case "offered":
      return [{ answer, label: answer, onPress: () => undefined }];
    case "refused":
      return [{ answer, label: answer, why: "the base moved", onPress: () => undefined }];
  }
}

// Every card: 3^3 combinations, each handed in reverse so the order the
// card draws is its own.
const CARDS: readonly (readonly Choice[])[] = OFFERS.flatMap((yes) =>
  OFFERS.flatMap((edit) =>
    OFFERS.map((no) => [...choiceOf("no", no), ...choiceOf("edit", edit), ...choiceOf("yes", yes)]),
  ),
);

const REACHED: readonly (Action | null)[] = [...ACTIONS, null];

describe("a chord answers only what the card offers", () => {
  test("over every card and every action", () => {
    for (const card of CARDS) {
      const offered = ordered(card);
      for (const action of REACHED) {
        const chosen = answering(offered, action);
        const wanted = offered.find((choice) => KEY[choice.answer] === action && choice.why === undefined);
        expect(chosen).toBe(wanted);
        if (chosen !== undefined) expect<Action | null>(KEY[chosen.answer]).toBe(action);
      }
    }
  });

  test("an action that is not one of the three answers nothing", () => {
    const all = ordered(ANSWERS.flatMap((answer) => choiceOf(answer, "offered")));
    const others = ACTIONS.filter((action) => !Object.values(KEY).includes(action));
    expect(others.map((action) => answering(all, action))).toEqual(others.map(() => undefined));
  });
});

describe("the answers the card draws", () => {
  test("at most one per answer, in the order yes, edit, no", () => {
    const doubled: readonly Choice[] = [
      { answer: "no", label: "first no", onPress: () => undefined },
      { answer: "yes", label: "yes", onPress: () => undefined },
      { answer: "no", label: "second no", onPress: () => undefined },
    ];
    expect(ordered(doubled).map((choice) => choice.label)).toEqual(["yes", "first no"]);
  });

  test("yes is the primary answer, the others quiet, each with its own chord", () => {
    const look = lookOf(
      {
        kind: "proposal",
        asker: "run 12",
        at: "v3",
        choices: ANSWERS.flatMap((answer) => choiceOf(answer, "offered")),
      },
      { uid: "s4", hear: () => undefined, marksOf: (action) => [action] },
    );
    expect(look.answers.map((answer) => [answer.answer, answer.tone, answer.marks])).toEqual([
      ["yes", "primary", ["decide.yes"]],
      ["edit", "quiet", ["decide.edit"]],
      ["no", "quiet", ["decide.no"]],
    ]);
  });

  test("the card is one group named by its heading and one entry of its list", () => {
    const look = lookOf(
      { kind: "ask", asker: "builder", at: "12:03", choices: [] },
      { uid: "s4", hear: () => undefined, marksOf: () => [] },
    );
    expect(look.card).toMatchObject({
      role: "group",
      tabindex: -1,
      "aria-labelledby": "s4-asker",
      "data-decide": "ask",
      "data-entry": "",
    });
    expect(look.asker).toEqual({ id: "s4-asker", text: "builder" });
    expect(look.glyph).toBe("gate");
  });
});
