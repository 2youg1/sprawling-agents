// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { enrol, keyField, referenceFor, referenceText, secretFor } from "./enrol";

function answering(body: string, status: number): () => void {
  const held = globalThis.fetch;
  const answer = (): Promise<Response> => Promise.resolve(new Response(body, { status }));
  globalThis.fetch = Object.assign(answer, { preconnect: held.preconnect });
  return () => {
    globalThis.fetch = held;
  };
}

const EXAMPLE = {
  origin: "http://localhost:7000",
  token: null,
  value: "sk-x",
  lang: "en",
} as const;

describe("the place in the vault", () => {
  test("the page spells a reference the way the city writes one", () => {
    const spelled = referenceText(referenceFor("example"));
    expect(spelled).toBe("secret:providers/example");
    expect(spelled).toMatch(/^secret:[A-Za-z0-9._-]+\/[A-Za-z0-9._-]+$/);
  });
});

describe("what a form keeps after an enrolment", () => {
  test("the city's answer is what is stored, not the page's promise", async () => {
    const restore = answering("secret:providers/example", 201);
    const outcome = await enrol({ ...EXAMPLE, ...referenceFor("example") });
    restore();
    expect(outcome).toEqual({ kind: "stored", reference: "secret:providers/example" });
  });

  test("a refusal carries the city's own sentence", async () => {
    const restore = answering("a realm is letters, digits, dot, dash and underscore", 400);
    const outcome = await enrol({ ...EXAMPLE, ...referenceFor("example id") });
    restore();
    expect(outcome).toEqual({
      kind: "refused",
      reason: "a realm is letters, digits, dot, dash and underscore",
    });
  });

  test("a refusal with no body says so", async () => {
    const restore = answering("", 400);
    const outcome = await enrol({ ...EXAMPLE, ...referenceFor("example") });
    restore();
    expect(outcome).toEqual({ kind: "refused", reason: "the city did not say why" });
  });

  test("the stored reference is used only with the provider it was filed under", () => {
    const held = { provider: "example", reference: "secret:providers/example" };
    expect(secretFor(held, "example")).toBe("secret:providers/example");
    expect(secretFor(held, "openai")).toBeNull();
    expect(keyField(held, "openai")).toEqual({ kind: "empty" });
  });
});
