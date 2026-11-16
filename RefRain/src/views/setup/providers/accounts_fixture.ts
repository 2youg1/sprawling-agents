// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { enrol } from "../../../core/enrol";
import { ServerLabel } from "../../../wire";
import type { Command, EndpointSummary, ProviderAccount } from "../../../wire";
import { freshEditor } from "./account_editor";
import type { Editor, Hands, Step } from "./account_editor";
import type { FlightState } from "./flight";
import { endpointRoster } from "./rosters";

export const account = (
  id: string,
  reference: string | null = `secret:providers/house.${id}`,
): ProviderAccount => ({
  id: ServerLabel.make(id),
  reference,
});

export const ENDPOINT: EndpointSummary = {
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
  models: [
    {
      id: "fable",
      input_modalities: [],
      canonical: "fable",
      listing: "listed",
      thinking: { levels: [], on: "unknown", from: "unknown", words: [] },
      generation: {
        output_pointer: "/max_tokens",
        settings: {},
        temperature: { support: "unknown" },
        top_p: { support: "unknown" },
        top_k: { support: "unknown" },
      },
    },
  ],
  name: "house",
  tuning: {
    headers: [{ name: "x-team", value: "blue" }],
    overrides: [],
    proxying: "except_local",
    timeout_ms: 90_000,
    accounts: [account("main"), account("spare")],
  },
};

export function rig(endpoint: EndpointSummary = ENDPOINT, flight?: FlightState) {
  const sent: Command[] = [];
  const requests: { url: string; body: unknown }[] = [];
  const focus: string[] = [];
  const answers: { complete: (r: Response) => void; fail: (e: Error) => void }[] = [];
  globalThis.fetch = Object.assign(
    (url: string | URL | Request, init?: RequestInit) => {
      requests.push({
        url: url instanceof Request ? url.url : url.toString(),
        body: typeof init?.body === "string" ? JSON.parse(init.body) : null,
      });
      return new Promise<Response>((complete, fail) => answers.push({ complete, fail }));
    },
    { preconnect: () => undefined },
  );
  const world: { endpoint: EndpointSummary; origin: string; token: string | null; open: boolean } =
    {
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
    ...(flight === undefined ? {} : { flight }),
  };
  const lists = () =>
    sent.map((command) =>
      "attach_endpoint" in command ? command.attach_endpoint.tuning.accounts : null,
    );
  const answer = (n: number, response: Response) => answers[n]?.complete(response);
  return { editor, hands, world, sent, lists, requests, focus, answers, answer };
}

export const flush = () => new Promise((resolve) => setImmediate(resolve));
export const typed = (editor: Editor, id: string, key: string, header = "") => {
  editor.draft = { id, reference: "", header, key };
};
