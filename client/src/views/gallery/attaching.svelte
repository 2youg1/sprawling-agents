<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The parts of attaching a provider, each drawn by its own seat or
  // look so the render gate reads every one of them: the list of what is
  // attached with a long address and a row nobody named, the attach
  // form's fold and the two readings of a draft that has headers and an
  // override, and the probed-model table in the states its seat reaches
  // only after a person has typed - rows ticked with figures and roles
  // stated, and an endpoint that served no list, filled in by hand. The
  // table's look is drawn from the wiring's own value over a state
  // written here, which stays live: a press in the fixture redraws as it
  // does on the page.
  import { ui } from "../../ui";
  import type { EndpointsAnswer } from "../../wire";
  import { freshTable, lookOf } from "../setup/model_table";
  import TableLook from "../setup/model_table.look.svelte";
  import Advanced from "../setup/providers/advanced.svelte";
  import { freshDraft } from "../setup/providers/draft";
  import type { Draft } from "../setup/providers/draft";
  import EndpointList from "../setup/providers/endpoint_list.svelte";
  import Previews from "../setup/providers/preview.svelte";
  import Case from "./case.svelte";
  import { ENDPOINTS, PROBED } from "./served";

  const { lang } = ui();

  // One endpoint whose address runs past its row and whose id differs
  // from its name, beside one that has listed no model yet.
  const LISTED: EndpointsAnswer = {
    chosen: [],
    endpoints: ENDPOINTS.endpoints.map((endpoint, at) =>
      at === 0
        ? { ...endpoint, base_url: "https://gateway.example.invalid/tenants/house-of-many-models/openai/v1" }
        : { ...endpoint, models: [] },
    ),
  };

  const draft = $state<Draft>({
    ...freshDraft(),
    baseUrl: "https://api.example.invalid/v1",
    headers: [{ key: "team", name: "x-team", value: "blue" }],
    overrides: [{ key: "temperature", name: "/temperature", value: "0.2" }],
  });

  const worked = $state(freshTable());
  worked.textOnly = false;
  for (const row of PROBED) worked.ticked[row.id] = true;
  const first = PROBED[0]?.id;
  if (first !== undefined) {
    worked.output[first] = "32768";
    worked.input[first] = "text_image";
    worked.role[first] = "main";
  }

  const typed = $state(freshTable());
  typed.manual = "house/writer, house/reader";
  typed.ticked["house/writer"] = true;

  const workedLook = $derived(lookOf(worked, PROBED, $lang));
  const typedLook = $derived(lookOf(typed, [], $lang));
</script>

<Case label="provider · a long address, and a row with no model listed">
  <EndpointList answer={LISTED} />
</Case>

<Case label="provider · the attach form's fold, closed">
  <Advanced
    {draft}
    setDraft={(part, value) => {
      draft[part] = value;
    }}
    onRenamed={() => undefined}
  />
</Case>

<Case label="provider · the two readings of a draft with a header and an override">
  <Previews {draft} reference="secret:providers/example" model="house/writer" />
</Case>

<Case label="models · every row ticked, one with its figures and role stated">
  <TableLook {...workedLook} />
</Case>

<Case label="models · no list served, two ids typed by hand">
  <TableLook {...typedLook} />
</Case>
