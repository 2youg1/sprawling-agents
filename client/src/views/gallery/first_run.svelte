<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The path from an empty city to the first assignment, in the three
  // states a person can meet it in: no provider attached, a provider
  // attached and no main model chosen, and a main model chosen. Each is
  // drawn at the two windows the path is judged at - a phone at 390 and
  // a desktop at 1440 - because the order of the welcome cards is the
  // whole point of the first two states, and a grid that reflows at 390
  // is where that order could quietly change.
  //
  // The refusal below is the one a person meets when they assign work
  // before any of this: the city's way out stands under the heading,
  // unfolded, since it is the only sentence that knows the cause.

  import { QUERIES } from "../../core/asking";
  import type { Answer, EndpointsAnswer, Query } from "../../wire";
  import { ENDPOINTS } from "./served";

  const BARE: EndpointsAnswer = { chosen: [], endpoints: [] };
  const UNCHOSEN: EndpointsAnswer = { chosen: [], endpoints: ENDPOINTS.endpoints };

  interface State {
    readonly name: string;
    readonly held: EndpointsAnswer;
  }

  const STATES: readonly State[] = [
    { name: "no provider", held: BARE },
    { name: "a provider, no main model", held: UNCHOSEN },
    { name: "a main model", held: ENDPOINTS },
  ];

  const WIDTHS: readonly number[] = [390, 1440];

  function answering(held: EndpointsAnswer): (query: Query) => Answer | undefined {
    return (query) => (query === QUERIES.endpoints ? { endpoints: held } : undefined);
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
    <Case label={`welcome · ${state.name} at ${String(width)}`} {width}>
      <Stand
        link={{ kind: "live", city: "sprawling" }}
        unread={[]}
        waiting={[]}
        answers={answering(state.held)}
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
