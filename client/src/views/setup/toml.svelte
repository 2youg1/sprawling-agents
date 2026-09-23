<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The city's own file, as this build can speak of it.
  //
  // **A proofing tool, not a setting.** It is collapsed at the foot of
  // the body rather than standing as a third column: a block that costs
  // the body its width is a block nobody reads (client-SPEC 4-36). What
  // stands inside is the one sentence this build can say about the file
  // - it cannot be read here - and no second spelling of the file
  // itself. `Query::Config` answers would fill it, each value beside the
  // layer it came from (client-SPEC 4-30); until the page asks, this is
  // the empty state and nothing here hand-spells a line of `config.toml`.

  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import EmptyState from "../parts/empty.svelte";

  const { lang } = ui();
</script>

<aside class="min-w-0" aria-label={say($lang, "setup_toml")}>
  <details class="rounded-card bg-chrome/60 px-base py-tight">
    <summary class="cursor-pointer text-label text-text-quiet hover:text-text">
      {say($lang, "setup_toml_toggle")}
    </summary>
    <EmptyState missing="setup_toml_unread" />
  </details>
</aside>
