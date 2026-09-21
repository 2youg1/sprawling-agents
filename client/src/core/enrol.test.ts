// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Enrolment, which the client had no test for at all - so the page's
// spelling of a vault place and the city's could drift apart with
// nothing failing. Two facts are pinned here: the reference the page
// derives has the grammar `kernel::SecretRef` writes, and what a form
// keeps is the text the city answered with rather than the page's
// promise.

import { describe, expect, test } from "bun:test";

import { enrol, keyField, referenceFor, referenceText, secretFor } from "./enrol";

// One HTTP answer, in place of the city. The stub replaces the one
// browser facility `enrol` reaches for, so nothing else about it is
// mocked and the body under test is the production path.
function answering(body: string, status: number): () => void {
  const held = globalThis.fetch;
  const answer = (): Promise<Response> =>
    Promise.resolve(new Response(body, { status }));
  globalThis.fetch = Object.assign(answer, { preconnect: held.preconnect });
  return () => {
    globalThis.fetch = held;
  };
}

const ZENMUX = { origin: "http://localhost:7000", token: null, value: "sk-x", lang: "en" } as const;

describe("the place in the vault", () => {
  test("the page spells a reference the way the city writes one", () => {
    // `SecretRef::new` writes `secret:` then a realm and a name of
    // `[A-Za-z0-9._-]+` joined by `/`. The page states the same shape
    // here and nowhere else, so a form cannot promise a place the city
    // would refuse to resolve.
    const spelled = referenceText(referenceFor("zenmux"));
    expect(spelled).toBe("secret:providers/zenmux");
    expect(spelled).toMatch(/^secret:[A-Za-z0-9._-]+\/[A-Za-z0-9._-]+$/);
  });
});

describe("what a form keeps after an enrolment", () => {
  test("the city's answer is what is stored, not the page's promise", async () => {
    const restore = answering("secret:providers/zenmux", 201);
    const outcome = await enrol({ ...ZENMUX, ...referenceFor("zenmux") });
    restore();
    expect(outcome).toEqual({ kind: "stored", reference: "secret:providers/zenmux" });
  });

  test("a refusal carries the city's own sentence", async () => {
    const restore = answering("a realm is letters, digits, dot, dash and underscore", 400);
    const outcome = await enrol({ ...ZENMUX, ...referenceFor("zen mux") });
    restore();
    expect(outcome).toEqual({
      kind: "refused",
      reason: "a realm is letters, digits, dot, dash and underscore",
    });
  });

  // A city that refused with nothing said is answered in the person's
  // language rather than with an empty box.
  test("a refusal with no body says so", async () => {
    const restore = answering("", 400);
    const outcome = await enrol({ ...ZENMUX, ...referenceFor("zenmux") });
    restore();
    expect(outcome).toEqual({ kind: "refused", reason: "the city did not say why" });
  });

  test("the stored reference is used only with the provider it was filed under", () => {
    const held = { provider: "zenmux", reference: "secret:providers/zenmux" };
    expect(secretFor(held, "zenmux")).toBe("secret:providers/zenmux");
    expect(secretFor(held, "openai")).toBeNull();
    expect(keyField(held, "openai")).toEqual({ kind: "empty" });
  });
});
