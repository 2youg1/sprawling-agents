// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { flushSync } from "svelte";

import type { PrivacyAction, PrivacyAnswer, PrivacyResult } from "../../../wire";
import { IdemKey } from "../../../wire";
import { HOME, IDEM } from "../../gallery/privacy_answer";
import { sectionsOf, type EntryModel } from "./page";
import { privacyPage, type PrivacyPage } from "./state.svelte";

// The page's wiring against a stand-in for the city: the answer is a
// value the test replaces, every command is recorded, and the link
// carries a command or not as the test says.
function harness(carries: boolean) {
  let answer = $state.raw<PrivacyAnswer>(HOME);
  const sent: PrivacyAction[] = [];
  let asked = 0;
  let page: PrivacyPage | undefined;
  const stop = $effect.root(() => {
    page = privacyPage({
      answer: () => answer,
      send: (action) => {
        sent.push(action);
        return { idem: IDEM, sent: carries };
      },
      refresh: () => {
        asked += 1;
      },
    });
  });
  flushSync();
  const recall = (): EntryModel => {
    const found = sectionsOf(answer)
      .flatMap((section) => section.entries)
      .find((each) => each.row.control === "recall_snapshots");
    if (found === undefined) return expect.unreachable("recall_snapshots is in the fixture");
    return found;
  };
  const answerWith = (idem: IdemKey, result: PrivacyResult): void => {
    answer = { ...HOME, outcomes: [{ idem, action: { restore: { control: "recall_snapshots", expected: { value: "absent" } } }, result }] };
    flushSync();
  };
  if (page === undefined) return expect.unreachable("the page was built");
  return { page, sent, asked: () => asked, recall, answerWith, stop };
}

describe("the privacy page's wiring", () => {
  test("sends nothing for a press that is cancelled", () => {
    const { page, sent, recall, stop } = harness(true);
    page.press(recall(), "restore");
    expect(page.heldOf("recall_snapshots").stage).toEqual({ kind: "confirming", action: "restore" });
    page.cancel(recall());
    stop();
    expect(page.heldOf("recall_snapshots").stage).toEqual({ kind: "idle" });
    expect(sent).toEqual([]);
  });

  test("sends the confirmed action with the value shown, asks again, and settles on its own idem only", () => {
    const { page, sent, asked, recall, answerWith, stop } = harness(true);
    page.press(recall(), "restore");
    page.confirm(recall());
    expect(sent).toEqual([{ restore: { control: "recall_snapshots", expected: { value: "dword", number: 1 } } }]);
    expect(asked()).toBe(1);
    answerWith(IdemKey.make("idem1-ffffffffffffffffffffffffffffffff"), { refused: { code: "conflict", error: { action: "", code: "E_INVALID_ARGS", nearby: [], recovery: "", retry: "no", subject: "" } } });
    expect(page.heldOf("recall_snapshots").stage).toEqual({ kind: "sending", action: "restore", idem: IDEM });
    answerWith(IDEM, { restored: { operation: 2 } });
    stop();
    expect(page.heldOf("recall_snapshots")).toEqual({ stage: { kind: "settled", idem: IDEM }, shown: { restored: { operation: 2 } } });
  });

  test("leaves an action the link could not carry ready to press again", () => {
    const { page, asked, recall, stop } = harness(false);
    page.press(recall(), "restore");
    page.confirm(recall());
    stop();
    expect(page.heldOf("recall_snapshots").stage).toEqual({ kind: "unsent", action: "restore" });
    expect(asked()).toBe(0);
  });

  test("never offers what an entry cannot do, even in a bulk restore", () => {
    const { page, sent, stop } = harness(true);
    page.restoreAll(sectionsOf(HOME).flatMap((section) => section.entries));
    stop();
    expect(sent).toEqual([{ restore: { control: "recall_snapshots", expected: { value: "dword", number: 1 } } }]);
  });
});
