// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { say } from "../../../core/lang";
import { ServerLabel } from "../../../wire";
import * as wiring from "./account_editor";
import { save } from "./account_editor";
import { account, ENDPOINT, flush, rig, typed } from "./accounts_fixture";

describe("a key typed into the account form", () => {
  test("is filed under the provider and account, and the account carries the reference the city answered", async () => {
    const { editor, hands, lists, requests, answer } = rig();
    typed(editor, "third", "fixture-key", "X-Third");
    save(editor, hands);
    typed(editor, "other", "", "X-Other");
    answer(0, new Response("secret:providers/house.third", { status: 201 }));
    await flush();
    expect(requests).toEqual([
      {
        url: "http://city.invalid/enroll",
        body: { realm: "providers", name: "house.third", value: "fixture-key" },
      },
    ]);
    expect(lists()).toEqual([
      [
        account("main"),
        account("spare"),
        {
          id: ServerLabel.make("third"),
          reference: "secret:providers/house.third",
          header: "X-Third",
        },
      ],
    ]);
    expect(editor.draft).toEqual({ id: "", reference: "", header: "", key: "" });
  });

  test("cannot be filed twice, and the list does not move while it is on its way", async () => {
    const { editor, hands, sent, requests, answer } = rig();
    typed(editor, "third", "fixture-key");
    save(editor, hands);
    save(editor, hands);
    wiring.move(editor, hands, "main", "down");
    wiring.remove(editor, hands, "spare");
    expect({ requests: requests.length, sent: sent.length }).toEqual({ requests: 1, sent: 0 });
    answer(0, new Response("secret:providers/house.third", { status: 201 }));
    await flush();
    expect(sent.length).toBe(1);
  });

  test("refused by the city keeps every field and can be tried again", async () => {
    const { editor, hands, sent, answer } = rig();
    typed(editor, "third", "fixture-key");
    save(editor, hands);
    answer(0, new Response("fixture refusal", { status: 403 }));
    await flush();
    expect({ draft: editor.draft, note: editor.note, pending: editor.pending, sent }).toEqual({
      draft: { id: "third", reference: "", header: "", key: "fixture-key" },
      note: "fixture refusal",
      pending: null,
      sent: [],
    });
    save(editor, hands);
    answer(1, new Response("secret:providers/house.third", { status: 201 }));
    await flush();
    expect(sent.length).toBe(1);
  });

  for (const failure of ["network", "body"] as const) {
    test(`lost to a ${failure} failure releases the form and keeps the draft`, async () => {
      const { editor, hands, sent, answers } = rig();
      typed(editor, "third", "fixture-key");
      save(editor, hands);
      if (failure === "network") answers[0]?.fail(new Error("fixture network failure"));
      else
        answers[0]?.complete(
          Object.assign(new Response(null, { status: 201 }), {
            text: () => Promise.reject(new Error("fixture body")),
          }),
        );
      await flush();
      expect({ pending: editor.pending, draft: editor.draft, note: editor.note, sent }).toEqual({
        pending: null,
        draft: { id: "third", reference: "", header: "", key: "fixture-key" },
        note: say("en", "enrol_unreachable"),
        sent: [],
      });
    });
  }

  test("that answers after the editor closed lands nowhere", async () => {
    const { editor, hands, sent, answer } = rig();
    typed(editor, "third", "fixture-key");
    save(editor, hands);
    wiring.leave(editor);
    answer(0, new Response("secret:providers/house.third", { status: 201 }));
    await flush();
    expect({ sent, note: editor.note }).toEqual({ sent: [], note: null });
  });

  test("cannot land on another provider, city or pairing than the one it was typed for", async () => {
    for (const change of ["provider", "origin", "token"] as const) {
      const { editor, hands, world, sent, answer } = rig();
      typed(editor, "third", "fixture-key");
      save(editor, hands);
      if (change === "provider") world.endpoint = { ...ENDPOINT, name: "elsewhere" };
      if (change === "origin") world.origin = "http://other.invalid";
      if (change === "token") world.token = null;
      answer(0, new Response("secret:providers/house.third", { status: 201 }));
      await flush();
      expect({ change, sent }).toEqual({ change, sent: [] });
    }
  });

  for (const again of ["fourth", "third"]) {
    test(`an old answer cannot claim a ${again} key typed after the editor reopened`, async () => {
      const { editor, hands, lists, answer } = rig();
      typed(editor, "third", "old-key");
      save(editor, hands);
      wiring.leave(editor);
      typed(editor, again, "new-key", "X-Again");
      save(editor, hands);
      const current = editor.pending?.ticket;
      answer(0, new Response("secret:providers/house.stale", { status: 201 }));
      await flush();
      expect({ pending: editor.pending?.ticket === current, lists: lists() }).toEqual({
        pending: true,
        lists: [],
      });
      answer(1, new Response(`secret:providers/house.${again}`, { status: 201 }));
      await flush();
      expect(lists()).toEqual([
        [
          account("main"),
          account("spare"),
          {
            id: ServerLabel.make(again),
            reference: `secret:providers/house.${again}`,
            header: "X-Again",
          },
        ],
      ]);
    });
  }
});

describe("a list that cannot be sent", () => {
  test("keeps the draft of a new account and says the link is down", async () => {
    const { editor, hands, world, sent, answer } = rig();
    world.open = false;
    typed(editor, "third", "fixture-key");
    save(editor, hands);
    answer(0, new Response("secret:providers/house.third", { status: 201 }));
    await flush();
    expect({ sent, draft: editor.draft, note: editor.note }).toEqual({
      sent: [],
      draft: { id: "third", reference: "", header: "", key: "fixture-key" },
      note: say("en", "setup_account_not_sent"),
    });
  });

  test("keeps an edit and keeps focus where it was on a removal", () => {
    const { editor, hands, world, focus } = rig();
    wiring.edit(editor, account("spare"));
    world.open = false;
    save(editor, hands);
    wiring.remove(editor, hands, "spare");
    expect({ editing: editor.editing, draft: editor.draft, focus }).toEqual({
      editing: "spare",
      draft: { id: "spare", reference: "secret:providers/house.spare", header: "", key: "" },
      focus: [],
    });
    world.open = true;
    save(editor, hands);
    expect({ editing: editor.editing, draft: editor.draft }).toEqual({
      editing: null,
      draft: { id: "", reference: "", header: "", key: "" },
    });
  });
});
