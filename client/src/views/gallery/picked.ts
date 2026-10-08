// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The endpoints the model picker's fixtures are drawn from, written
// once: an open model served by three providers at three prices with
// three different sets of thinking levels, a closed model at its own
// vendor and an aggregator, and a local model with no thinking control;
// then an aggregator serving more models than there are providers, and
// a city whose only provider offers no thinking control at all.

import type { Effort, EndpointSummary, ModelFactsSummary } from "../../wire";
import { Window } from "../../wire";
import type { Names } from "../talk/composer";
import type { PickerFacts } from "../talk/picker_look";
import { UNTUNED } from "./configured";

interface Said {
  readonly canonical: string;
  readonly context: number;
  readonly price?: readonly [string, string];
  readonly levels?: readonly Effort[];
}

function model(id: string, said: Said): ModelFactsSummary {
  return {
    id,
    canonical: said.canonical,
    context_tokens: Window.make(said.context),
    input_modalities: ["text"],
    input_price: said.price?.[0] ?? null,
    output_price: said.price?.[1] ?? null,
    thinking: { levels: [...(said.levels ?? [])], on: "unknown", from: said.levels === undefined ? "unknown" : "upstream", words: [] },
  };
}

function endpoint(name: string, label: string, models: readonly ModelFactsSummary[], local = false): EndpointSummary {
  return {
    account_status: [],
    base_url: local ? "http://127.0.0.1:11434/v1" : `https://${name}.example/v1`,
    connection_kind: "openai_compat",
    dialect: "open_ai",
    has_credential: !local,
    label,
    local,
    models: [...models],
    name,
    tuning: UNTUNED,
  };
}

const DEEPSEEK = "deepseek-v4";
const GPT = "gpt-5.2";
const QWEN = "qwen3-coder:30b";

// Three models across five providers: the picker lists models first and
// the providers of the chosen one under it.
export const MANY_PROVIDERS: readonly EndpointSummary[] = [
  endpoint("deepseek", "DeepSeek", [model(DEEPSEEK, { canonical: DEEPSEEK, context: 1_000_000, price: ["$0.27", "$1.10"], levels: ["low", "high", "max"] })]),
  endpoint("openrouter", "OpenRouter", [
    model(`deepseek/${DEEPSEEK}`, { canonical: DEEPSEEK, context: 1_000_000, price: ["$0.30", "$1.20"], levels: ["low", "medium", "high", "xhigh"] }),
    model(`openai/${GPT}`, { canonical: GPT, context: 400_000, price: ["$1.80", "$14.50"], levels: ["minimal", "low", "medium", "high", "xhigh"] }),
  ]),
  endpoint("siliconflow", "SiliconFlow", [model(`deepseek-ai/${DEEPSEEK}`, { canonical: DEEPSEEK, context: 128_000, price: ["$0.25", "$1.00"], levels: ["low", "high"] })]),
  endpoint("openai", "OpenAI", [model(GPT, { canonical: GPT, context: 400_000, price: ["$1.75", "$14.00"], levels: ["minimal", "low", "medium", "high", "xhigh"] })]),
  endpoint("ollama", "Ollama", [model(QWEN, { canonical: QWEN, context: 256_000 })], true),
];

// An aggregator with more models than the city has providers: the
// picker lists providers first and the chosen one's models under it.
export const MANY_MODELS: readonly EndpointSummary[] = [
  endpoint("openrouter", "OpenRouter", [
    model(`deepseek/${DEEPSEEK}`, { canonical: DEEPSEEK, context: 1_000_000, price: ["$0.30", "$1.20"], levels: ["low", "medium", "high", "xhigh"] }),
    ...["claude-fable-5.1", "gemini-3-pro", "glm-5", "kimi-k3", "llama-5-maverick", "mistral-large-3", "qwen3-max", "grok-5"].map((id) =>
      model(`vendor/${id}`, { canonical: id, context: 200_000, price: ["$1.00", "$5.00"], levels: ["low", "medium", "high"] }),
    ),
  ]),
  endpoint("deepseek", "DeepSeek", [model(DEEPSEEK, { canonical: DEEPSEEK, context: 1_000_000, price: ["$0.27", "$1.10"], levels: ["low", "high", "max"] })]),
];

// One local provider whose models have no thinking control.
export const ONE_PROVIDER: readonly EndpointSummary[] = [
  endpoint("ollama", "Ollama", [model(QWEN, { canonical: QWEN, context: 256_000 }), model("gemma4:27b", { canonical: "gemma4:27b", context: 128_000 })], true),
];

export const AT_DEEPSEEK: Names = { endpoint: "deepseek", model: DEEPSEEK };
export const AT_OLLAMA: Names = { endpoint: "ollama", model: QWEN };
export const AT_OPENROUTER: Names = { endpoint: "openrouter", model: `deepseek/${DEEPSEEK}` };

const ignore = (): void => undefined;

// The facts one fixture hands the picker; its picks go nowhere.
export function pickerFacts(endpoints: readonly EndpointSummary[], chosen: Names | undefined, stated: Effort | null): PickerFacts {
  return { endpoints, chosen, stated, pick: { model: ignore, level: ignore } };
}
