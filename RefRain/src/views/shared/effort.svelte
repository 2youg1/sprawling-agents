<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
</script>

<script lang="ts">
  import { readable } from "svelte/store";
  import { QUERIES } from "../../core/asking";
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";

  const u = ui();
  const { lang } = u;
  const city = u.conn.asking.ask(QUERIES.city);

  const first = $derived.by(() => {
    const held = $city;
    if (held === undefined || !("city" in held)) return null;
    return (
      [...held.city.buildings].map((each) => each.addr).sort((a, b) => a.localeCompare(b))[0] ??
      null
    );
  });
  const config = $derived(
    first === null ? readable(undefined) : u.conn.asking.ask({ config: { addr: first } }),
  );

  const standing = $derived.by((): string | null => {
    const answer = $config;
    if (first === null || answer === undefined || !("config" in answer)) return null;
    const settled = answer.config.effort;
    return settled === null || settled === undefined
      ? fill(say($lang, "setup_effort_unstated"), { addr: first })
      : fill(say($lang, "setup_effort_standing"), {
          addr: first,
          effort: settled.effort,
          from: say($lang, `setup_effort_from_${settled.from}`),
        });
  });
</script>

<div class="flex flex-col gap-base">
  <p class="text-note text-text-quiet">{say($lang, "setup_effort")}</p>
  <p class="max-w-measure text-note leading-read text-text-faint">
    {say($lang, "setup_effort_essay")}
  </p>
  {#if standing !== null}
    <p class="max-w-measure text-note text-text">{standing}</p>
  {/if}
  <p class="max-w-measure text-note text-text-quiet">{say($lang, "setup_effort_city")}</p>
  <p class="max-w-measure text-note text-text-quiet">{say($lang, "setup_effort_where")}</p>
</div>
