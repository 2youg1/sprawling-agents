<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
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
</script>

<script lang="ts">
  import Notice from "../parts/notice.svelte";
  import Welcome from "../welcome.svelte";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";
</script>

{#each STATES as state (state.name)}
  {#each WIDTHS as width (width)}
    <Case label={`guide · ${state.name} at ${String(width)}`} {width}>
      <Stand
        link={{ kind: "live", city: "sprawling" }}
        unread={[]}
        waiting={[]}
        answers={answering(state)}
      >
        <Welcome rank="section" />
      </Stand>
    </Case>
  {/each}
{/each}

{#each WIDTHS as width (width)}
  <Case label={`notice · assigned before a main model at ${String(width)}`} {width}>
    <Notice
      seat="inline"
      weight="alert"
      action="cannot choose the main model"
      code="E_CONFIG_INVALID"
      subject="no model is chosen for this tag"
      recovery="attach a provider on the settings page and pick a model for this tag"
    />
  </Case>
{/each}
