// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The account editor's wiring, driven the way its seat drives it and
// with no look at all: whichever look draws the editor, these are the
// presses it hands on and what they send (client D93). The enrolment
// goes through the production `enrol` against a fetch that answers when
// the test says, so a key that comes back late can be raced against
// everything a person may do meanwhile.

import { describe, expect, test } from "bun:test";

import { enrol } from "../../../core/enrol";
import { say } from "../../../core/lang";
import { ProviderName, ServerLabel } from "../../../wire";
import type { AxError, Command, EndpointSummary, ProviderAccount } from "../../../wire";
import * as wiring from "./account_editor";
import { freshEditor, refuse, save, settle, shown } from "./account_editor";
import type { Editor, Hands, Step } from "./account_editor";
import { lookOf } from "./accounts";
import { endpointRoster } from "./rosters";

const account = (id: string, reference: string | null = `secret:providers/house.${id}`): ProviderAccount => ({
  id: ServerLabel.make(id),
  reference,
});

const ENDPOINT: EndpointSummary = {
  account_status: [
    { id: ServerLabel.make("main"), key: "stored" },
    { id: ServerLabel.make("spare"), key: "missing" },
  ],
  base_url: "https://api.example.invalid/v1",
  connection_kind: "openai_compat",
  dialect: "open_ai",
  has_credential: true,
  label: "house",
  local: false,
  models: [{ id: "fable", input_modalities: [], canonical: "fable", thinking: { levels: [], on: "unknown", from: "unknown", words: [] } }],
  name: "house",
  tuning: {
    headers: [{ name: "x-team", value: "blue" }],
    overrides: [],
    proxying: "except_local",
    timeout_ms: 90_000,
    accounts: [account("main"), account("spare")],
  },
};

// One editor and everything around it, recorded.
function rig(endpoint: EndpointSummary = ENDPOINT) {
  const sent: Command[] = [];
  const requests: { url: string; body: unknown }[] = [];
  const focus: string[] = [];
  const answers: { complete: (r: Response) => void; fail: (e: Error) => void }[] = [];
  globalThis.fetch = Object.assign(
    (url: string | URL | Request, init?: RequestInit) => {
      requests.push({ url: url instanceof Request ? url.url : url.toString(), body: typeof init?.body === "string" ? JSON.parse(init.body) : null });
      return new Promise<Response>((complete, fail) => answers.push({ complete, fail }));
    },
    { preconnect: () => undefined },
  );
  const world: { endpoint: EndpointSummary; origin: string; token: string | null; open: boolean } = {
    endpoint,
    origin: "http://city.invalid",
    token: "pairing",
    open: true,
  };
  const editor: Editor = freshEditor();
  const hands: Hands = {
    roster: () => endpointRoster(world.endpoint, "two"),
    lang: () => "en",
    reach: () => ({ origin: world.origin, token: world.token }),
    send: (command) => {
      if (!world.open) return false;
      sent.push(command);
      return true;
    },
    enrol,
    follow: (id: string, step: Step) => focus.push(`${id}:${step}`),
    focusHeading: () => focus.push("heading"),
  };
  const lists = () => sent.map((command) => ("attach_endpoint" in command ? command.attach_endpoint.tuning.accounts : null));
  const answer = (n: number, response: Response) => answers[n]?.complete(response);
  return { editor, hands, world, sent, lists, requests, focus, answers, answer };
}

const flush = () => new Promise((resolve) => setImmediate(resolve));
const typed = (editor: Editor, id: string, key: string, header = "") => {
  editor.draft = { id, reference: "", header, key };
};

describe("a key typed into the account form", () => {
  test("is filed under the provider and account, and the account carries the reference the city answered", async () => {
    const { editor, hands, lists, requests, answer } = rig();
    typed(editor, "third", "fixture-key", "X-Third");
    save(editor, hands);
    typed(editor, "other", "", "X-Other");
    answer(0, new Response("secret:providers/house.third", { status: 201 }));
    await flush();
    expect(requests).toEqual([
      { url: "http://city.invalid/enroll", body: { realm: "providers", name: "house.third", value: "fixture-key" } },
    ]);
    expect(lists()).toEqual([[account("main"), account("spare"), { id: ServerLabel.make("third"), reference: "secret:providers/house.third", header: "X-Third" }]]);
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
      else answers[0]?.complete(Object.assign(new Response(null, { status: 201 }), { text: () => Promise.reject(new Error("fixture body")) }));
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
      expect({ pending: editor.pending?.ticket === current, lists: lists() }).toEqual({ pending: true, lists: [] });
      answer(1, new Response(`secret:providers/house.${again}`, { status: 201 }));
      await flush();
      expect(lists()).toEqual([[account("main"), account("spare"), { id: ServerLabel.make(again), reference: `secret:providers/house.${again}`, header: "X-Again" }]]);
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

describe("the order of the accounts", () => {
  test("goes to the city whole, with the tuning it read back and the models it serves, and no legacy key", () => {
    const { editor, hands, sent, focus } = rig();
    wiring.move(editor, hands, "spare", "up");
    expect(sent.map((command) => ("attach_endpoint" in command ? { attach_endpoint: { ...command.attach_endpoint, idem: "-" } } : command))).toEqual([
      {
        attach_endpoint: {
          name: ProviderName.make("house"),
          base_url: ENDPOINT.base_url,
          dialect: "open_ai",
          secret: null,
          auth_header: null,
          admit: ["fable"],
          tuning: { ...ENDPOINT.tuning, accounts: [account("spare"), account("main")], account_retries: null },
          idem: "-",
        },
      },
    ]);
    expect(focus).toEqual(["spare:up"]);
  });

  test("moves again from what was sent until the city answers, then from the answer", () => {
    const { editor, hands, lists, world } = rig({ ...ENDPOINT, tuning: { ...ENDPOINT.tuning, accounts: [account("a"), account("b"), account("c")] } });
    wiring.move(editor, hands, "c", "up");
    wiring.move(editor, hands, "c", "up");
    expect(lists()).toEqual([
      [account("a"), account("c"), account("b")],
      [account("c"), account("a"), account("b")],
    ]);
    world.endpoint = { ...world.endpoint, tuning: { ...world.endpoint.tuning, accounts: [account("b"), account("a")] } };
    settle(editor, endpointRoster(world.endpoint, "two"));
    expect(shown(editor, endpointRoster(world.endpoint, "two"))).toEqual([account("b"), account("a")]);
  });

  test("a refusal draws the city's list again", () => {
    const { editor, hands } = rig();
    wiring.move(editor, hands, "spare", "up");
    const refusal: AxError = { action: "attach", code: "E_CONFIG_INVALID", subject: "house", recovery: "fix it", retry: "no", nearby: [] };
    refuse(editor, refusal);
    expect(shown(editor, endpointRoster(ENDPOINT, "two"))).toEqual([account("main"), account("spare")]);
  });

  test("a removal sends the list without the account and puts focus on the heading", () => {
    const { editor, hands, lists, focus } = rig();
    wiring.remove(editor, hands, "main");
    expect({ lists: lists(), focus }).toEqual({ lists: [[account("spare")]], focus: ["heading"] });
  });
});

describe("the key of a removed account", () => {
  const holds = { heading: () => undefined, control: () => () => undefined };

  test("is offered for deletion after the list that no longer names it, and the deletion follows that list", () => {
    const { editor, hands, sent } = rig();
    wiring.remove(editor, hands, "main");
    const offer = lookOf(editor, hands, holds).forget;
    expect(offer?.text).toBe(say("en", "setup_account_forget_offer").replace("{id}", "main"));
    offer?.press();
    expect({
      frames: sent.map((command) => Object.keys(command)[0]),
      forgotten: sent.map((command) => ("forget_secret" in command ? command.forget_secret.reference : null)).at(-1),
      awaits: wiring.awaits(editor),
      offer: lookOf(editor, hands, holds).forget,
    }).toEqual({
      frames: ["attach_endpoint", "forget_secret"],
      forgotten: "secret:providers/house.main",
      awaits: true,
      offer: undefined,
    });
  });

  test("is not offered for a key the vault does not hold, and the offer goes with the next change", () => {
    const { editor, hands } = rig({ ...ENDPOINT, tuning: { ...ENDPOINT.tuning, accounts: [account("main"), account("spare"), account("third")] } });
    wiring.remove(editor, hands, "spare");
    expect(lookOf(editor, hands, holds).forget).toBeUndefined();
    wiring.remove(editor, hands, "main");
    expect(lookOf(editor, hands, holds).forget).not.toBeUndefined();
    typed(editor, "fourth", "");
    wiring.save(editor, hands);
    expect(lookOf(editor, hands, holds).forget).toBeUndefined();
  });
});

describe("the retry count on one account", () => {
  const holds = { heading: () => undefined, control: () => () => undefined };

  test("is offered with two accounts, says the city's figure while none is chosen, and goes out with the list as drawn", () => {
    const { editor, hands, sent } = rig();
    wiring.move(editor, hands, "spare", "up");
    const offered = lookOf(editor, hands, holds).retries;
    expect({ held: offered?.held, fallback: offered?.fallback, options: offered?.options.map((option) => option.value) }).toEqual({
      held: null,
      fallback: say("en", "setup_account_retries_fallback").replace("{n}", say("en", "setup_account_retries_two")),
      options: ["one", "two"],
    });
    offered?.pick("one");
    const last = sent.at(-1);
    expect(last !== undefined && "attach_endpoint" in last ? last.attach_endpoint.tuning : null).toEqual({
      ...ENDPOINT.tuning,
      accounts: [account("spare"), account("main")],
      account_retries: "one",
    });
    expect(lookOf(editor, hands, holds).retries?.held).toBe("one");
  });

  test("keeps the chosen count when the list moves", () => {
    const { editor, hands, sent } = rig({ ...ENDPOINT, tuning: { ...ENDPOINT.tuning, account_retries: "one" } });
    wiring.move(editor, hands, "spare", "up");
    const last = sent.at(-1);
    expect(last !== undefined && "attach_endpoint" in last ? last.attach_endpoint.tuning.account_retries : null).toBe("one");
  });

  test("is not offered while one account is listed, because one account never reads it", () => {
    const { editor, hands } = rig({ ...ENDPOINT, tuning: { ...ENDPOINT.tuning, accounts: [account("main")] } });
    expect(lookOf(editor, hands, holds).retries).toBeUndefined();
  });
});

describe("what the look is given", () => {
  const holds = { heading: () => undefined, control: () => () => undefined };

  test("refuses the moves and the removal that would do nothing, and says why", () => {
    const { editor, hands } = rig({ ...ENDPOINT, tuning: { ...ENDPOINT.tuning, accounts: [account("main")] } });
    const row = lookOf(editor, hands, holds).rows[0];
    expect(row?.controls.map((control) => [control.key, control.why])).toEqual([
      ["up", say("en", "setup_account_first")],
      ["down", say("en", "setup_account_last")],
      ["edit", undefined],
      ["remove", say("en", "setup_account_only")],
    ]);
  });

  test("names each key's state as the city answered it, and a row it has not looked at as unread", () => {
    const { editor, hands } = rig({ ...ENDPOINT, tuning: { ...ENDPOINT.tuning, accounts: [account("main"), account("spare"), account("new")] } });
    expect(lookOf(editor, hands, holds).rows.map((row) => [row.place, row.id, row.key, row.wanting])).toEqual([
      [1, "main", say("en", "setup_account_key_stored"), false],
      [2, "spare", say("en", "setup_account_key_missing"), true],
      [3, "new", say("en", "setup_account_key_unread"), false],
    ]);
  });

  test("tells an endpoint attached with one key that a list replaces it", () => {
    const { editor, hands } = rig({ ...ENDPOINT, account_status: [], tuning: { ...ENDPOINT.tuning, accounts: null } });
    const look = lookOf(editor, hands, holds);
    expect({ legacy: look.legacy, rows: look.rows }).toEqual({ legacy: say("en", "setup_accounts_legacy"), rows: [] });
  });

  test("refuses an id that is not a label or is already listed, and a key read from the environment", () => {
    const { editor, hands } = rig({ ...ENDPOINT, account_status: [{ id: ServerLabel.make("main"), key: "environment" }] });
    typed(editor, "Main", "");
    expect(lookOf(editor, hands, holds).save.why).toBe(say("en", "setup_account_id_shape"));
    typed(editor, "main", "");
    expect(lookOf(editor, hands, holds).save.why).toBe(say("en", "setup_account_id_taken"));
    wiring.edit(editor, account("main"));
    const key = lookOf(editor, hands, holds).fields.find((field) => field.name === "key");
    expect({ why: lookOf(editor, hands, holds).save.why, disabled: key?.disabled, help: key?.help }).toEqual({
      why: undefined,
      disabled: true,
      help: say("en", "setup_account_key_environment_help"),
    });
  });
});
