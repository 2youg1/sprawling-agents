// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The attached-model editor's round trip, without a component or a look:
// what refresh accepts, what save sends, which answer settles it, and how
// the account editor's flight excludes it.

import { describe, expect, test } from "bun:test";

import { say } from "../../../core/lang";
import type { ModelFact, Probed } from "../../../core/probed";
import { Ceiling, ServerLabel, Window } from "../../../wire";
import type { Command, EndpointsAnswer, ModelFactsSummary } from "../../../wire";
import { chosenRows, setTick } from "../model_rows";
import { begin, freshFlight } from "./flight";
import {
  awaiting,
  choose,
  freshModelEditor,
  lookOf,
  refresh,
  refuse,
  save,
  settleAnswer,
  settleProbe,
  untick,
} from "./model_editor";
import type { Hands } from "./model_editor";

function model(id: string, listing: "listed" | "named" = "listed"): ModelFactsSummary {
  return {
    id,
    context_tokens: Window.make(100_000),
    max_output_tokens: Ceiling.make(8192),
    input_modalities: ["text"],
    input_price: null,
    output_price: null,
    thinking: { levels: [], on: "unknown", from: "unknown", words: [] },
    generation: {
      output_pointer: "/max_tokens",
      settings: {},
      temperature: { support: "unknown" },
      top_p: { support: "unknown" },
      top_k: { support: "unknown" },
    },
    canonical: id,
    listing,
  };
}

const ANSWER: EndpointsAnswer = {
  chosen: [{ endpoint: "house", model: "a", tag: "main", max_output_tokens: Ceiling.make(8192) }],
  endpoints: [
    {
      name: "house",
      label: "House",
      base_url: "https://gateway.example.invalid/v1",
      dialect: "open_ai",
      connection_kind: "openai_compat",
      models: [model("a"), model("b"), model("typed", "named")],
      local: false,
      has_credential: true,
      tuning: {
        headers: [{ name: "x-tenant", value: "blue" }],
        overrides: [],
        accounts: [{ id: ServerLabel.make("fresh"), reference: "secret:providers/house.fresh" }],
      },
      account_status: [],
    },
  ],
};

function rig(answer: EndpointsAnswer = ANSWER) {
  const world: { answer: EndpointsAnswer; probed: Probed | null } = { answer, probed: null };
  const sent: Command[] = [];
  const flight = freshFlight();
  const editor = freshModelEditor(answer, "house", flight);
  const hands: Hands = {
    answer: () => world.answer,
    probed: () => world.probed,
    send: (command) => {
      sent.push(command);
      return true;
    },
  };
  return { world, sent, flight, editor, hands };
}

function probe(
  name: string,
  facts: readonly ModelFact[],
  failed: Probed["failure"] = null,
): Probed {
  return {
    name,
    baseUrl: "https://gateway.example.invalid/v1",
    models: facts.map((fact) => fact.id),
    facts,
    reach: null,
    failure: failed,
  };
}

function fact(id: string): ModelFact {
  return {
    id,
    contextTokens: 100_000,
    maxOutputTokens: 8192,
    inputModalities: ["text"],
    inputPrice: null,
    outputPrice: null,
  };
}

describe("refreshing an attached endpoint", () => {
  test("accepts only a newer answer for this row, keeps new rows unticked, and retains named rows", () => {
    const { editor, hands, sent } = rig();
    refresh(editor, hands);
    const before = hands.probed();
    settleProbe(editor, probe("other", [fact("other-model")]));
    expect(awaiting(editor)).toBe(true);
    const found = probe("house", [fact("a"), fact("b"), fact("c")]);
    expect(found === before).toBe(false);
    settleProbe(editor, found);
    expect(editor.served.map((row) => row.id)).toEqual(["a", "b", "c"]);
    expect(editor.chosen.map((row) => row.id)).toEqual(["a", "b", "typed"]);
    expect(editor.table.ticked.c).toBeUndefined();
    expect(editor.flight.current).toBeNull();
    expect(sent.length).toBe(1);
  });

  test("a failed answer leaves the stored facts and draft mounted", () => {
    const { editor, hands } = rig();
    const before = editor.served;
    refresh(editor, hands);
    settleProbe(editor, probe("house", [], { code: "E_TIMEOUT", subject: "gateway" }));
    expect(editor.served).toBe(before);
    expect(editor.chosen.map((row) => row.id)).toEqual(["a", "b", "typed"]);
    expect(editor.probeFailure).toEqual({ code: "E_TIMEOUT", subject: "gateway" });
    expect(lookOf(editor, hands, "en").alert?.code).toBe("E_TIMEOUT");
  });

  test("an account edit excludes refresh before any frame leaves", () => {
    const { editor, hands, flight, sent } = rig();
    begin(flight, "accounts");
    refresh(editor, hands);
    expect(sent).toEqual([]);
    expect(editor.probing).toBeNull();
  });
});

describe("saving an attached endpoint's models", () => {
  test("an unchanged draft cannot save; removing B sends no account list and no role command", () => {
    const { editor, hands, sent } = rig();
    expect(lookOf(editor, hands, "en").save.why).toBe(say("en", "setup_models_unchanged"));
    untick(editor, "b");
    expect(lookOf(editor, hands, "en").removal).toBe("will remove: b");
    save(editor, hands);
    expect(sent.map((command) => Object.keys(command)[0])).toEqual(["attach_endpoint"]);
    const frame = sent[0];
    expect(
      frame !== undefined && "attach_endpoint" in frame
        ? frame.attach_endpoint.tuning.accounts
        : [],
    ).toBeNull();
    expect(
      frame !== undefined && "attach_endpoint" in frame ? frame.attach_endpoint.admit : [],
    ).toEqual(["a"]);
    expect(
      frame !== undefined && "attach_endpoint" in frame ? frame.attach_endpoint.named : [],
    ).toEqual(["typed"]);
  });

  test("unticking the main model is refused locally, and a role move sends SelectModel only for that role", () => {
    const { editor, hands, sent } = rig();
    editor.served = [...editor.served, fact("c")];
    untick(editor, "a");
    expect(editor.chosen.some((row) => row.id === "a")).toBe(true);
    expect(sent).toEqual([]);
    setTick(editor.table, "c", true);
    editor.table.role.c = "main";
    editor.table.role.a = "";
    choose(editor, chosenRows(editor.table, editor.served));
    save(editor, hands);
    expect(sent.map((command) => Object.keys(command)[0])).toEqual([
      "attach_endpoint",
      "select_model",
    ]);
    const selection = sent[1];
    expect(
      selection !== undefined && "select_model" in selection ? selection.select_model.model : null,
    ).toBe("c");
  });

  test("changing only a role model's output facts still sends its selection", () => {
    const { editor, hands, sent } = rig();
    editor.table.output.a = "4096";
    choose(editor, chosenRows(editor.table, editor.served));
    save(editor, hands);
    expect(sent.map((command) => Object.keys(command)[0])).toEqual([
      "attach_endpoint",
      "select_model",
    ]);
    const selection = sent[1];
    expect(
      selection !== undefined && "select_model" in selection
        ? selection.select_model.max_output_tokens
        : null,
    ).toBe(Ceiling.make(4096));
  });

  test("changing only generation saves it and waits for that exact read-back", () => {
    const { editor, hands, world, sent } = rig();
    const draft = editor.generation.a;
    expect(draft).toBeDefined();
    if (draft !== undefined) draft.temperature = "0.2";
    save(editor, hands);
    expect(sent.map((command) => Object.keys(command)[0])).toEqual(["attach_endpoint"]);
    const frame = sent[0];
    const generation =
      frame !== undefined && "attach_endpoint" in frame
        ? (frame.attach_endpoint.tuning.generation ?? [])
        : [];
    expect(generation).toEqual([
      {
        model: "a",
        settings: { temperature: "0.2", top_p: null, top_k: null },
        output_tokens: null,
        support: {},
      },
    ]);
    settleAnswer(editor, world.answer);
    expect(awaiting(editor)).toBe(true);
    world.answer = {
      ...world.answer,
      endpoints: world.answer.endpoints.map((endpoint) => ({
        ...endpoint,
        tuning: { ...endpoint.tuning, generation: [...generation] },
      })),
    };
    settleAnswer(editor, world.answer);
    expect(awaiting(editor)).toBe(false);
  });

  test("an attached endpoint's raw overrides edit in the same save", () => {
    const { editor, hands, sent } = rig();
    editor.overrides.push({ pointer: "/temperature", value: "0.8" });
    save(editor, hands);
    const frame = sent[0];
    expect(
      frame !== undefined && "attach_endpoint" in frame
        ? frame.attach_endpoint.tuning.overrides
        : [],
    ).toEqual([{ pointer: "/temperature", value: "0.8" }]);
  });

  test("a capability correction makes an unsupported value saveable", () => {
    const answer = {
      ...ANSWER,
      endpoints: ANSWER.endpoints.map((endpoint) => ({
        ...endpoint,
        models: endpoint.models.map((model) =>
          model.id === "a"
            ? {
                ...model,
                generation: {
                  ...model.generation,
                  top_k: { support: "unsupported" as const, source: "upstream" as const },
                },
              }
            : model,
        ),
      })),
    };
    const { editor, hands, sent } = rig(answer);
    const draft = editor.generation.a;
    expect(draft).toBeDefined();
    if (draft === undefined) return;
    draft.top_k = "40";
    save(editor, hands);
    expect(sent).toEqual([]);
    draft.support = { top_k: "supported" };
    save(editor, hands);
    expect(sent.map((command) => Object.keys(command)[0])).toEqual(["attach_endpoint"]);
  });

  test("a save acknowledgement does not overwrite a newer role draft", () => {
    const { editor, hands, world } = rig();
    untick(editor, "b");
    save(editor, hands);
    editor.table.role.a = "";
    world.answer = {
      ...world.answer,
      endpoints: world.answer.endpoints.map((endpoint) => ({
        ...endpoint,
        models: [model("a"), model("typed", "named")],
      })),
    };
    settleAnswer(editor, world.answer);
    expect(editor.table.role.a).toBe("");
    expect(editor.table.heldRole.a).toBe("main");
  });

  test("clearing a role without a replacement sends nothing", () => {
    const { editor, hands, sent } = rig();
    editor.table.role.a = "";
    choose(editor, chosenRows(editor.table, editor.served));
    save(editor, hands);
    expect(sent).toEqual([]);
    expect(lookOf(editor, hands, "en").save.why).toContain("main");
  });

  test("only the answer carrying the saved models and roles settles the flight and returns focus", () => {
    const { editor, hands, world, flight } = rig();
    untick(editor, "b");
    save(editor, hands);
    settleAnswer(editor, world.answer);
    expect(awaiting(editor)).toBe(true);
    world.answer = {
      ...world.answer,
      endpoints: world.answer.endpoints.map((endpoint) => ({
        ...endpoint,
        models: [model("a"), model("typed", "named")],
      })),
    };
    settleAnswer(editor, world.answer);
    expect({ current: flight.current, saved: editor.saved, focus: editor.focusSummary }).toEqual({
      current: null,
      saved: ["a", "typed"],
      focus: true,
    });
  });

  test("a refusal releases the shared flight and stays beside the save control", () => {
    const { editor, hands, flight } = rig();
    untick(editor, "b");
    save(editor, hands);
    refuse(editor, {
      action: "attach",
      code: "E_CONFIG_INVALID",
      subject: "house",
      recovery: "check it",
      retry: "no",
      nearby: [],
    });
    expect(flight.current).toBeNull();
    expect(lookOf(editor, hands, "en").alert?.code).toBe("E_CONFIG_INVALID");
  });
});
