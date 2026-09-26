<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // How hard the city thinks, and the three things a person has to know
  // before they choose: the choice is frozen for the length of a run,
  // the two wire formats disagree about where `none` is written, and
  // saying nothing is not the same as saying `none`.
  //
  // **There is no picker here, because this page cannot keep an
  // answer.** The standing level is the city's own `[model] effort`,
  // read from `CONFIG.toml` by the configuration ladder that freezes
  // every run; a level kept in this browser instead would ride on every
  // dispatch from it and outrank the file without saying so. What this page states
  // about the file is where the answer lives - the level is
  // `Query::Config`'s to answer and the session selector's to write - so
  // the section says that and stops there.
  //
  // **A session states its own level in the selector over the
  // composer**, which is where a dispatch is made and the only scope
  // this client can honestly offer: the city writes that level into the
  // room it opens, and every run in that room then holds it.
  //
  // **The level in force is read, never kept.** `Query::Config` answers
  // for an address rather than for the city, because the ladder has a
  // building's rung; the section asks for the first building in the
  // city's own order and names it beside the level, so the answer is
  // exactly as wide as what was asked. A city with no building yet
  // shows no line, because nothing would say which rung is in play.
  //
  // The welcome walk and the settings page both show this, so the
  // paragraph cannot be present on one page and missing from the other.
  // The words are in `lang.json` like every other word a reader is
  // handed - this file only says where they go.
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
    return [...held.city.buildings].map((each) => each.addr).sort((a, b) => a.localeCompare(b))[0] ?? null;
  });
  const config = $derived(first === null ? readable(undefined) : u.conn.asking.ask({ config: { addr: first } }));

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
  <p class="max-w-measure text-note leading-relaxed text-text-faint">{say($lang, "setup_effort_essay")}</p>
  {#if standing !== null}
    <p class="max-w-measure text-note text-text">{standing}</p>
  {/if}
  <p class="max-w-measure text-note text-text-quiet">{say($lang, "setup_effort_city")}</p>
  <p class="max-w-measure text-note text-text-quiet">{say($lang, "setup_effort_where")}</p>
</div>
