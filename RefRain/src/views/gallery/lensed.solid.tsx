// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type {
  Answer,
  EvidenceItem,
  PrefixSegment,
  PrefixSlot,
  PromptTools,
  Query,
} from "../../wire";
import { Address, B3Hash, Locator, RunId, Seq } from "../../wire";

const RUN = RunId.make("5b1e7d2a-9c3f-4e8b-a1d2-3c4b5a6f7e80");

function segmentOf(slot: PrefixSlot, at: number, stored: boolean, text: string): PrefixSegment {
  return {
    slot,
    bytes: text.length * 37,
    hash: B3Hash.make(String(at).repeat(64)),
    sources: [
      { addr: Address.make(`shop/${slot}/Memo.md`), kept: text.length, dropped: 0 },
      {
        addr: Address.make(`shop/${slot}/Plan.md`),
        kept: text.length,
        dropped: at === 2 ? 1_840 : 0,
      },
    ],
    stored,
    text,
  };
}

const SEGMENTS: readonly PrefixSegment[] = [
  segmentOf(
    "city",
    1,
    true,
    "You are a resident of the city sprawling. Keep to the building you were placed in.",
  ),
  segmentOf(
    "building",
    2,
    true,
    "The shop sells what its residents make.\nEvery change ships with a test.",
  ),
  segmentOf("resident", 3, false, ""),
  segmentOf("run", 4, true, "Tidy the release notes for 0.0.10."),
];

const TOOLS: PromptTools = {
  hash: "b3-tools",
  len: 1840,
  index: 412,
  rows: [
    {
      name: "edit",
      namespace: "sprawling",
      exposure: "core",
      from: "default",
      len: 980,
      hint: "Replace text in a file you have read",
    },
    {
      name: "github_create_issue",
      namespace: "github",
      exposure: "off",
      from: "building",
      len: 310,
      hint: "create a issue on the forge",
    },
    {
      name: "plan",
      namespace: "sprawling",
      exposure: "dormant",
      from: "default",
      len: 640,
      hint: "Claim a node of the roadmap",
    },
    {
      name: "read",
      namespace: "sprawling",
      exposure: "core",
      from: "fixed",
      len: 860,
      hint: "Open one file by its path",
    },
  ],
};

const ITEMS: readonly EvidenceItem[] = [
  {
    at: Seq.make(412),
    kind: "screenshot",
    locator: Locator.make("shop/notes/.evidence/412.png"),
    picture: { width: 1440, height: 900, media_type: "image/png" },
  },
  {
    at: Seq.make(418),
    kind: "finished",
    locator: Locator.make("shop/notes/CHANGELOG.md"),
    picture: null,
  },
];

function answering(items: readonly EvidenceItem[]): (query: Query) => Answer | undefined {
  return (query) => {
    if (typeof query !== "object") return undefined;
    if ("prefix" in query)
      return { prefix: { run: query.prefix.run, segments: SEGMENTS, tools: TOOLS } };
    if ("evidence" in query) return { evidence: { run: query.evidence.run, items } };
    return undefined;
  };
}

import { For } from "solid-js";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { lookOf as copyLook } from "../parts/copy";
import CopyLook from "../parts/copy.look.solid";
import Context from "../run/context.solid";
import Evidence from "../run/evidence.solid";
import { lookOf } from "../run/prompt";
import Prompt, { PromptContent } from "../run/prompt.solid";
import { GalleryCase } from "./case";
import Stand from "./stand.solid";
import { LIVE } from "./timed.solid";

const OPENED = SEGMENTS[1]?.hash ?? "";
const HANDS = { toggle: () => undefined, visit: () => undefined };
const CITY = { kind: "live", city: "sprawling" } as const;
const ITEMS_CASES: readonly (readonly EvidenceItem[])[] = [ITEMS, []];

export default function LensedCases() {
  const lang = storeValue(ui().lang);
  return (
    <>
      <GalleryCase label="run · the prompt lens as the city answers it">
        <Stand link={CITY} unread={[]} waiting={[]} answers={answering(ITEMS)}>
          {() => <Prompt run={RUN} />}
        </Stand>
      </GalleryCase>
      <GalleryCase label="run · the prompt lens, one segment open with its copy receipt, one the store no longer holds">
        <PromptContent
          look={lookOf(
            { segments: SEGMENTS, tools: TOOLS },
            { open: new Set([OPENED, "tools"]) },
            lang(),
            HANDS,
          )}
          paintCopy={(item, text) =>
            item.key === OPENED ? (
              <CopyLook
                {...copyLook(
                  { text, note: say(lang(), "run_prompt_copy") },
                  { kind: "copied" },
                  lang(),
                  () => undefined,
                )}
              />
            ) : undefined
          }
        />
      </GalleryCase>
      <GalleryCase label="run · the prompt lens of a run sent no prompt">
        <PromptContent
          look={lookOf({ segments: [], tools: null }, { open: new Set() }, lang(), HANDS)}
        />
      </GalleryCase>
      <GalleryCase label="run · the context lens over forty turns">
        <Context turns={LIVE} />
      </GalleryCase>
      <For each={ITEMS_CASES}>
        {(items) => (
          <GalleryCase
            label={`run · the evidence lens ${items.length === 0 ? "with nothing to check" : "with a screenshot and a finish"}`}
          >
            <Stand link={CITY} unread={[]} waiting={[]} answers={answering(items)}>
              {() => <Evidence run={RUN} />}
            </Stand>
          </GalleryCase>
        )}
      </For>
    </>
  );
}
