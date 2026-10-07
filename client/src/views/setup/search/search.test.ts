// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The web search card's wiring, driven the way its seat drives it and
// with no look at all: what each press sends as the city's whole
// `[search]`, and how a service's account editor files its keys and
// sends its list (client D93, D94).

import { describe, expect, test } from "bun:test";

import { enrol } from "../../../core/enrol";
import { fill, say } from "../../../core/lang";
import { ServerLabel } from "../../../wire";
import type { AxError, Command, SearchConfiguration, SearchSupplier, SettledSearch } from "../../../wire";
import * as accounts from "../providers/account_editor";
import { lookOf as accountsLookOf } from "../providers/accounts";
import { lookOf } from "./search";
import * as wiring from "./search_editor";
import type { SearchEditor, SearchHands } from "./search_editor";

const label = (text: string) => ServerLabel.make(text);

const service = (id: string, extra: Partial<SearchSupplier> = {}): SearchSupplier => ({
  id: label(id),
  url: `https://mcp.${id}.invalid/mcp`,
  remote: `${id}_search`,
  query_field: "query",
  accounts: [],
  ...extra,
});

const SETTLED: SettledSearch = {
  account_status: [],
  configuration: "default",
  default_url: "https://mcp.exa.ai/mcp",
  from: "default",
};

const custom = (selected: string, ...suppliers: SearchSupplier[]): SearchConfiguration => ({
  custom: { selected: label(selected), suppliers },
});

function rig(settled: SettledSearch = SETTLED) {
  const sent: Command[] = [];
  const world = { settled, open: true };
  const editor: SearchEditor = wiring.freshSearch();
  const hands: SearchHands = {
    settled: () => world.settled,
    lang: () => "en",
    send: (command) => {
      if (!world.open) return false;
      sent.push(command);
      return true;
    },
  };
  const values = () => sent.map((command) => ("configure_city" in command ? command.configure_city.search : null));
  return { editor, hands, world, sent, values };
}

const typed = (editor: SearchEditor, id: string, rest: Partial<wiring.SupplierDraft> = {}) => {
  editor.draft = { id, url: `https://mcp.${id}.invalid/mcp`, remote: `${id}_search`, query: "query", count: "", objective: "", ...rest };
};

describe("the choice of service", () => {
  test("sends default and off as the city's whole [search], and nothing for the choice already in force", () => {
    const { editor, hands, values } = rig();
    wiring.choose(editor, hands, "default");
    wiring.choose(editor, hands, "off");
    wiring.choose(editor, hands, "default");
    expect(values()).toEqual(["off", "default"]);
  });

  test("opens the form for a first service rather than sending a custom value with no service", () => {
    const { editor, hands, values } = rig();
    wiring.choose(editor, hands, "custom");
    const look = lookOf(editor, hands);
    expect({ values: values(), held: look.choice.held, form: look.form?.title }).toEqual({
      values: [],
      held: "custom",
      form: say("en", "search_add"),
    });
    typed(editor, "brave", { count: "count" });
    wiring.save(editor, hands);
    expect(values()).toEqual([custom("brave", service("brave", { count_field: "count", objective_field: null }))]);
  });

  test("puts the last custom value back after a turn through off", () => {
    const listed = custom("brave", service("brave"));
    const { editor, hands, values } = rig({ ...SETTLED, city: listed, configuration: listed, from: "city" });
    wiring.choose(editor, hands, "off");
    wiring.choose(editor, hands, "custom");
    expect(values()).toEqual(["off", listed]);
  });
});

describe("the list of services", () => {
  const listed = custom("brave", service("tavily"), service("brave", { accounts: [{ id: label("main"), reference: "secret:search/brave.main", header: "x-key" }] }));
  const settled: SettledSearch = { ...SETTLED, city: listed, configuration: listed, from: "city" };

  test("selects another service with the list as it stands", () => {
    const { editor, hands, values } = rig(settled);
    wiring.select(editor, hands, label("tavily"));
    expect(values()).toEqual([custom("tavily", service("tavily"), service("brave", { accounts: [{ id: label("main"), reference: "secret:search/brave.main", header: "x-key" }] }))]);
  });

  test("refuses to remove the service in use and the only service, and says why", () => {
    const { editor, hands, values } = rig(settled);
    wiring.remove(editor, hands, label("brave"));
    const controls = lookOf(editor, hands).suppliers.map((row) => [row.id, row.controls.map((control) => [control.key, control.why])]);
    expect({ values: values(), controls }).toEqual({
      values: [],
      controls: [
        ["tavily", [["use", undefined], ["edit", undefined], ["remove", undefined]]],
        ["brave", [["use", say("en", "search_in_use")], ["edit", undefined], ["remove", say("en", "search_remove_in_use")]]],
      ],
    });
    wiring.remove(editor, hands, label("tavily"));
    expect(lookOf(editor, hands).suppliers.map((row) => row.controls.find((control) => control.key === "remove")?.why)).toEqual([say("en", "search_only")]);
  });

  test("keeps an edited service's place and its accounts", () => {
    const { editor, hands, values } = rig(settled);
    const brave = lookOf(editor, hands).suppliers[1];
    brave?.controls.find((control) => control.key === "edit")?.press();
    wiring.input(editor, "url", "https://elsewhere.invalid/mcp");
    wiring.save(editor, hands);
    const [sentValue] = values();
    expect(sentValue).toEqual(
      custom(
        "brave",
        service("tavily"),
        service("brave", {
          url: "https://elsewhere.invalid/mcp",
          count_field: null,
          objective_field: null,
          accounts: [{ id: label("main"), reference: "secret:search/brave.main", header: "x-key" }],
        }),
      ),
    );
  });

  test("refuses a form with a bad id, a listed id, or a missing part", () => {
    const { editor, hands } = rig(settled);
    typed(editor, "Brave");
    expect(lookOf(editor, hands).form?.save.why).toBe(say("en", "setup_account_id_shape"));
    typed(editor, "brave");
    expect(lookOf(editor, hands).form?.save.why).toBe(say("en", "search_id_taken"));
    typed(editor, "exa", { remote: " " });
    expect(lookOf(editor, hands).form?.save.why).toBe(say("en", "search_required"));
  });

  test("draws the city's value again after a refusal, and keeps the draft when the frame cannot leave", () => {
    const { editor, hands, world, values } = rig(settled);
    wiring.select(editor, hands, label("tavily"));
    const refusal: AxError = { action: "configure", code: "E_CONFIG_INVALID", subject: "[search]", recovery: "fix it", retry: "no", nearby: [] };
    wiring.refuse(editor, refusal);
    expect(wiring.shownSearch(editor, world.settled)).toEqual(listed);
    world.open = false;
    typed(editor, "exa");
    wiring.save(editor, hands);
    expect({ sent: values().length, note: editor.note, draft: editor.draft.id }).toEqual({ sent: 1, note: say("en", "setup_account_not_sent"), draft: "exa" });
  });

  test("says that a file nearer the hall states its own [search], which the card does not change", () => {
    const { editor, hands } = rig({ ...settled, configuration: "off", from: "building" });
    expect(lookOf(editor, hands).override).toBe(
      fill(say("en", "search_override"), { choice: say("en", "search_choice_off"), layer: say("en", "search_layer_building") }),
    );
  });
});

describe("a service's account editor", () => {
  const listed = custom("brave", service("tavily"), service("brave"));
  const settled: SettledSearch = { ...SETTLED, city: listed, configuration: listed, from: "city" };

  test("files a key under the search realm and sends the whole [search] with only this service's accounts replaced", async () => {
    const { editor: card, hands: cardHands } = rig(settled);
    const brave = lookOf(card, cardHands).suppliers[1];
    expect(brave?.id).toBe("brave");
    if (brave === undefined) return;
    const roster = brave.roster;
    const sent: Command[] = [];
    const requests: unknown[] = [];
    globalThis.fetch = Object.assign(
      (_url: string | URL | Request, init?: RequestInit) => {
        requests.push(typeof init?.body === "string" ? JSON.parse(init.body) : null);
        return Promise.resolve(new Response("secret:search/brave.main", { status: 201 }));
      },
      { preconnect: () => undefined },
    );
    const editor = accounts.freshEditor();
    const hands: accounts.Hands = {
      roster: () => roster,
      lang: () => "en",
      reach: () => ({ origin: "http://city.invalid", token: "pairing" }),
      send: (command) => sent.push(command) > 0,
      enrol,
      follow: () => undefined,
      focusHeading: () => undefined,
    };
    const look = accountsLookOf(editor, hands, { heading: () => undefined, control: () => () => undefined });
    expect({ header: look.fields.find((field) => field.name === "header")?.folded, order: look.order, retries: look.retries }).toEqual({
      header: false,
      order: say("en", "setup_accounts_order_search"),
      retries: undefined,
    });
    editor.draft = { id: "main", reference: "", header: "x-subscription-token", key: "fixture-key" };
    accounts.save(editor, hands);
    await new Promise((resolve) => setImmediate(resolve));
    expect({ requests, sent: sent.map((command) => ("configure_city" in command ? command.configure_city.search : null)) }).toEqual({
      requests: [{ realm: "search", name: "brave.main", value: "fixture-key" }],
      sent: [custom("brave", service("tavily"), service("brave", { accounts: [{ id: label("main"), reference: "secret:search/brave.main", header: "x-subscription-token" }] }))],
    });
  });
});
