// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { say } from "../../../core/lang";
import { ProviderName, ServerLabel } from "../../../wire";
import type { AxError } from "../../../wire";
import * as wiring from "./account_editor";
import { refuse, settle, shown } from "./account_editor";
import { lookOf } from "./accounts";
import { account, ENDPOINT, rig, typed } from "./accounts_fixture";
import { begin, end, freshFlight } from "./flight";
import { endpointRoster } from "./rosters";

describe("the order of the accounts", () => {
  test("goes to the city whole, with the tuning it read back and the models it serves, and no legacy key", () => {
    const { editor, hands, sent, focus } = rig();
    wiring.move(editor, hands, "spare", "up");
    expect(
      sent.map((command) =>
        "attach_endpoint" in command
          ? { attach_endpoint: { ...command.attach_endpoint, idem: "-" } }
          : command,
      ),
    ).toEqual([
      {
        attach_endpoint: {
          name: ProviderName.make("house"),
          base_url: ENDPOINT.base_url,
          dialect: "open_ai",
          secret: null,
          auth_header: null,
          admit: ["fable"],
          named: [],
          tuning: {
            ...ENDPOINT.tuning,
            accounts: [account("spare"), account("main")],
            account_retries: null,
          },
          idem: "-",
        },
      },
    ]);
    expect(focus).toEqual(["spare:up"]);
  });

  test("moves again from what was sent until the city answers, then from the answer", () => {
    const { editor, hands, lists, world } = rig({
      ...ENDPOINT,
      tuning: { ...ENDPOINT.tuning, accounts: [account("a"), account("b"), account("c")] },
    });
    wiring.move(editor, hands, "c", "up");
    wiring.move(editor, hands, "c", "up");
    expect(lists()).toEqual([
      [account("a"), account("c"), account("b")],
      [account("c"), account("a"), account("b")],
    ]);
    world.endpoint = {
      ...world.endpoint,
      tuning: { ...world.endpoint.tuning, accounts: [account("b"), account("a")] },
    };
    settle(editor, endpointRoster(world.endpoint, "two"));
    expect(shown(editor, endpointRoster(world.endpoint, "two"))).toEqual([
      account("b"),
      account("a"),
    ]);
  });

  test("the shared row ticket excludes a model save and releases on the city's newer answer", () => {
    const flight = freshFlight();
    const { editor, hands, world, sent } = rig(ENDPOINT, flight);
    const operation = begin(flight, "models");
    wiring.move(editor, hands, "spare", "up");
    expect(sent).toEqual([]);

    if (operation !== null) end(flight, operation);
    wiring.move(editor, hands, "spare", "up");
    expect(sent.length).toBe(1);
    wiring.move(editor, hands, "main", "down");
    expect(sent.length).toBe(1);
    world.endpoint = { ...world.endpoint, label: "House, reordered" };
    settle(editor, endpointRoster(world.endpoint, "two"));
    expect(flight.current).toBeNull();
  });

  test("a refusal draws the city's list again", () => {
    const { editor, hands } = rig();
    wiring.move(editor, hands, "spare", "up");
    const refusal: AxError = {
      action: "attach",
      code: "E_CONFIG_INVALID",
      subject: "house",
      recovery: "fix it",
      retry: "no",
      nearby: [],
    };
    refuse(editor, refusal);
    expect(shown(editor, endpointRoster(ENDPOINT, "two"))).toEqual([
      account("main"),
      account("spare"),
    ]);
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
      forgotten: sent
        .map((command) => ("forget_secret" in command ? command.forget_secret.reference : null))
        .at(-1),
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
    const { editor, hands } = rig({
      ...ENDPOINT,
      tuning: {
        ...ENDPOINT.tuning,
        accounts: [account("main"), account("spare"), account("third")],
      },
    });
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
    expect({
      held: offered?.held,
      fallback: offered?.fallback,
      options: offered?.options.map((option) => option.value),
    }).toEqual({
      held: null,
      fallback: say("en", "setup_account_retries_fallback").replace(
        "{n}",
        say("en", "setup_account_retries_two"),
      ),
      options: ["one", "two"],
    });
    offered?.pick("one");
    const last = sent.at(-1);
    expect(
      last !== undefined && "attach_endpoint" in last ? last.attach_endpoint.tuning : null,
    ).toEqual({
      ...ENDPOINT.tuning,
      accounts: [account("spare"), account("main")],
      account_retries: "one",
    });
    expect(lookOf(editor, hands, holds).retries?.held).toBe("one");
  });

  test("keeps the chosen count when the list moves", () => {
    const { editor, hands, sent } = rig({
      ...ENDPOINT,
      tuning: { ...ENDPOINT.tuning, account_retries: "one" },
    });
    wiring.move(editor, hands, "spare", "up");
    const last = sent.at(-1);
    expect(
      last !== undefined && "attach_endpoint" in last
        ? last.attach_endpoint.tuning.account_retries
        : null,
    ).toBe("one");
  });

  test("is not offered while one account is listed, because one account never reads it", () => {
    const { editor, hands } = rig({
      ...ENDPOINT,
      tuning: { ...ENDPOINT.tuning, accounts: [account("main")] },
    });
    expect(lookOf(editor, hands, holds).retries).toBeUndefined();
  });
});

describe("what the look is given", () => {
  const holds = { heading: () => undefined, control: () => () => undefined };

  test("refuses the moves and the removal that would do nothing, and says why", () => {
    const { editor, hands } = rig({
      ...ENDPOINT,
      tuning: { ...ENDPOINT.tuning, accounts: [account("main")] },
    });
    const row = lookOf(editor, hands, holds).rows[0];
    expect(row?.controls.map((control) => [control.key, control.why])).toEqual([
      ["up", say("en", "setup_account_first")],
      ["down", say("en", "setup_account_last")],
      ["edit", undefined],
      ["remove", say("en", "setup_account_only")],
    ]);
  });

  test("names each key's state as the city answered it, and a row it has not looked at as unread", () => {
    const { editor, hands } = rig({
      ...ENDPOINT,
      tuning: { ...ENDPOINT.tuning, accounts: [account("main"), account("spare"), account("new")] },
    });
    expect(
      lookOf(editor, hands, holds).rows.map((row) => [row.place, row.id, row.key, row.wanting]),
    ).toEqual([
      [1, "main", say("en", "setup_account_key_stored"), false],
      [2, "spare", say("en", "setup_account_key_missing"), true],
      [3, "new", say("en", "setup_account_key_unread"), false],
    ]);
  });

  test("tells an endpoint attached with one key that a list replaces it", () => {
    const { editor, hands } = rig({
      ...ENDPOINT,
      account_status: [],
      tuning: { ...ENDPOINT.tuning, accounts: null },
    });
    const look = lookOf(editor, hands, holds);
    expect({ legacy: look.legacy, rows: look.rows }).toEqual({
      legacy: say("en", "setup_accounts_legacy"),
      rows: [],
    });
  });

  test("refuses an id that is not a label or is already listed, and a key read from the environment", () => {
    const { editor, hands } = rig({
      ...ENDPOINT,
      account_status: [{ id: ServerLabel.make("main"), key: "environment" }],
    });
    typed(editor, "Main", "");
    expect(lookOf(editor, hands, holds).save.why).toBe(say("en", "setup_account_id_shape"));
    typed(editor, "main", "");
    expect(lookOf(editor, hands, holds).save.why).toBe(say("en", "setup_account_id_taken"));
    wiring.edit(editor, account("main"));
    const key = lookOf(editor, hands, holds).fields.find((field) => field.name === "key");
    expect({
      why: lookOf(editor, hands, holds).save.why,
      disabled: key?.disabled,
      help: key?.help,
    }).toEqual({
      why: undefined,
      disabled: true,
      help: say("en", "setup_account_key_environment_help"),
    });
  });
});
