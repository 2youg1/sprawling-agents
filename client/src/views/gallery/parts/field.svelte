<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The text field with help, with the city's error, with the browser's
  // own reading of `pattern`, and in mono with fixed parts.
  import { say } from "../../../core/lang";
  import { referenceFor, referenceText } from "../../../core/enrol";
  import { ui } from "../../../ui";
  import Field from "../../parts/field.svelte";
  import Case from "../case.svelte";

  const { lang } = ui();

  let host = $state("api.zenmux.ai");
  let bad = $state("api_gateway.internal");
  let key = $state(referenceText(referenceFor("zenmux")));
  let measured = $state("a million");
</script>

<!-- Two red edges that mean two things: `error` is the city's answer
and arrives red at once; `:user-invalid` is the browser reading
`pattern`, and turns red only after the person has left the box. -->
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
