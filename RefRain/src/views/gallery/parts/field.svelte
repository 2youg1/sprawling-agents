<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import { referenceFor, referenceText } from "../../../core/enrol";
  import { say } from "../../../core/lang";
  import { ui } from "../../../ui";
  import Field from "../../parts/field.svelte";
  import Case from "../case.svelte";

  const { lang } = ui();

  let host = $state("api.example.test");
  let bad = $state("api_gateway.internal");
  let key = $state(referenceText(referenceFor("example")));
  let measured = $state("a million");
  let stepped = $state("200000");
</script>

<Case label="field · help, the city's error, the browser's user-invalid, mono with fixed parts">
  <div class="flex flex-col gap-base">
    <Field
      label={say($lang, "setup_base_url")}
      help={say($lang, "part_help_base_url")}
      value={host}
      onInput={(value) => {
        host = value;
      }}
      prefix="https://"
      suffix="/v1"
    />
    <Field
      label={say($lang, "setup_base_url")}
      error={say($lang, "part_error_host")}
      value={bad}
      onInput={(value) => {
        bad = value;
      }}
    />
    <Field
      label={say($lang, "part_context_window")}
      help={say($lang, "part_context_window")}
      value={measured}
      onInput={(value) => {
        measured = value;
      }}
      pattern="[0-9]+"
    />
    <Field
      label={say($lang, "setup_key")}
      value={key}
      onInput={(value) => {
        key = value;
      }}
      mono
    />
  </div>
</Case>

<Case label="field · disabled, a hidden label, a number that steps">
  <div class="flex flex-col gap-base">
    <Field label={say($lang, "setup_key")} value={key} onInput={() => undefined} mono disabled />
    <Field
      label={say($lang, "part_context_window")}
      labelling="hidden"
      kind="number"
      step={1024}
      value={stepped}
      onInput={(value) => {
        stepped = value;
      }}
    />
  </div>
</Case>
