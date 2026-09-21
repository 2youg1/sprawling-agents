// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What one provider answered when this city asked what it serves.
//
// Three fixtures read these rows - the table on the settings page, the
// model column of the selector over the composer, and the shared
// controls that sort and correct a table - so the rows are written once
// here. A second spelling of the same three model ids would be three
// facts with two homes, and the first time somebody corrected one of
// them the two screens would disagree about what this provider serves.

import type { ModelFact } from "../../core/probed";
import type { EndpointsAnswer } from "../../wire";

// The three model ids, each written once. Every list below names them
// through these, so a fixture cannot describe a model the endpoint does
// not serve.
const FABLE = "anthropic/claude-fable-5.1";
const NUCLEUS = "openai/gpt-nucleus-6";
const MUSE = "meta/muse-spark-1.3-contributor";
const SORA = "openai/sora-2";

// One row of the model table a provider's probe answers with, in the
// shape §3.4 asks for: an id, what it can read, and what it may write -
// the last of which a person corrects when the probe could not read it.
export interface ModelRow {
  readonly id: string;
  readonly context: string;
  readonly ceiling: string;
}

// The row every fixture treats as the chosen one, named rather than
// reached by its position: a list somebody reorders must not silently
// change which model a screen reports as picked.
export const CHOSEN: ModelRow = { id: FABLE, context: "204800", ceiling: "64000" };

export const MODELS: readonly ModelRow[] = [
  { id: MUSE, context: "131072", ceiling: "8192" },
  CHOSEN,
  { id: NUCLEUS, context: "400000", ceiling: "" },
];

// What one provider's probe answered, as a person meets it: three text
// models across three vendors, and one the text-only switch hides.
// Two rows state their own ceilings and prices, one states nothing but a
// name, and one is marked as video by the provider rather than by its
// spelling - which is the whole range this table has to render.
export const PROBED: readonly ModelFact[] = [
  {
    id: FABLE,
    contextTokens: 204_800,
    maxOutputTokens: 64_000,
    inputModalities: ["image", "text"],
    inputPrice: "0.000003",
    outputPrice: "0.000015",
  },
  {
    id: NUCLEUS,
    contextTokens: 400_000,
    maxOutputTokens: null,
    inputModalities: ["text"],
    inputPrice: null,
    outputPrice: null,
  },
  {
    id: SORA,
    contextTokens: null,
    maxOutputTokens: null,
    inputModalities: ["video"],
    inputPrice: null,
    outputPrice: null,
  },
  {
    id: MUSE,
    contextTokens: null,
    maxOutputTokens: null,
    inputModalities: [],
    inputPrice: null,
    outputPrice: null,
  },
];

// Two attached providers, one with a key filed for it and one without,
// which is the difference the endpoint list exists to show.
export const ENDPOINTS: EndpointsAnswer = {
  chosen: [{ endpoint: "zenmux", model: CHOSEN.id, tag: "main" }],
  endpoints: [
    {
      base_url: "https://api.zenmux.ai/v1",
      dialect: "open_ai",
      has_credential: true,
      label: "ZenMux",
      local: false,
      models: [FABLE, NUCLEUS],
      name: "zenmux",
    },
    {
      base_url: "http://127.0.0.1:11434/v1",
      dialect: "open_ai",
      has_credential: false,
      label: "local",
      local: true,
      models: ["local/qwen3"],
      name: "local",
    },
  ],
};
