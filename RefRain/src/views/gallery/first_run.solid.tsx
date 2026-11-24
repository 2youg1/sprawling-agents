// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";
import { QUERIES } from "../../core/asking";
import type { Answer, EndpointsAnswer, GuideProgress, IdentityAnswer, Query } from "../../wire";
import { ENDPOINTS } from "./served";
const BARE: EndpointsAnswer = { chosen: [], endpoints: [] };
const ATTACHED: EndpointsAnswer = { ...ENDPOINTS, chosen: [] };
const NAMED: IdentityAnswer = {
  stated: {
    about: "",
    user_id: "ada",
    mayor: "Cat",
    mayor_text: "",
    preferences_text: "",
    version: "b3:0000000000000000000000000000000000000000000000000000000000000001",
  },
};
interface State {
  readonly name: string;
  readonly endpoints: EndpointsAnswer;
  readonly guide: GuideProgress;
  readonly identity: IdentityAnswer | undefined;
}
const STATES: readonly State[] = [
  { name: "a fresh city", endpoints: BARE, guide: {}, identity: undefined },
  {
    name: "an attached endpoint, main model open",
    endpoints: ATTACHED,
    guide: { at: "provider" },
    identity: undefined,
  },
  {
    name: "a main model, dependencies put off, texts open",
    endpoints: ENDPOINTS,
    guide: { at: "texts", dependencies: "skipped", texts: "seen" },
    identity: NAMED,
  },
  {
    name: "walked to the end",
    endpoints: ENDPOINTS,
    guide: { at: "mcp", dependencies: "skipped", texts: "seen", skills: "skipped", mcp: "seen" },
    identity: NAMED,
  },
];
const WIDTHS: readonly number[] = [390, 1440];
function answering(state: State): (query: Query) => Answer | undefined {
  return (query) => {
    if (query === QUERIES.endpoints) return { endpoints: state.endpoints };
    if (query === QUERIES.guide) return { guide: state.guide };
    if (query === QUERIES.identity && state.identity !== undefined)
      return { identity: state.identity };
    return undefined;
  };
}
import Notice from "../parts/notice_view";
import Welcome from "../welcome.solid";
import { GalleryCase } from "./case";
import Stand from "./stand.solid";

export default function FirstRunCases() {
  return (
    <>
      <For
        each={Array.from(STATES ?? []).map((__rrItem0, __rrIndex1) => ({
          value: __rrItem0,
          key: __rrItem0.name,
        }))}
        keyed={(entry) => entry.key}
      >
        {(__rrItem0) => (
          <>
            <For
              each={Array.from(WIDTHS ?? []).map((__rrItem2, __rrIndex3) => ({
                value: __rrItem2,
                key: __rrItem2,
              }))}
              keyed={(entry) => entry.key}
            >
              {(__rrItem2) => (
                <>
                  <GalleryCase
                    label={`guide · ${__rrItem0().value.name} at ${String(__rrItem2().value)}`}
                    width={__rrItem2().value}
                  >
                    <Stand
                      link={{ kind: "live", city: "sprawling" }}
                      unread={[]}
                      waiting={[]}
                      answers={answering(__rrItem0().value)}
                      children={() => (
                        <>
                          <Welcome rank="section" />
                        </>
                      )}
                    />
                  </GalleryCase>
                </>
              )}
            </For>
          </>
        )}
      </For>
      <For
        each={Array.from(WIDTHS ?? []).map((__rrItem4, __rrIndex5) => ({
          value: __rrItem4,
          key: __rrItem4,
        }))}
        keyed={(entry) => entry.key}
      >
        {(__rrItem4) => (
          <>
            <GalleryCase
              label={`notice · assigned before a main model at ${String(__rrItem4().value)}`}
              width={__rrItem4().value}
            >
              <Notice
                seat="inline"
                weight="alert"
                action="cannot choose the main model"
                code="E_CONFIG_INVALID"
                subject="no model is chosen for this tag"
                recovery="attach a provider on the settings page and pick a model for this tag"
              />
            </GalleryCase>
          </>
        )}
      </For>
    </>
  );
}
