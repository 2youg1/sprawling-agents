<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // Where a User drops a skill: the city's library, which every building
  // may admit from, and the chosen building's own shelf, each with the
  // copy button that puts the folder on the clipboard. The city answers
  // with its own name, so what is drawn is the folder under that name
  // rather than a shape to be filled in.
  //
  // **Folders rather than an add button**, because the city's door for
  // writing a shelved document answers `not_built`
  // (`Command::PutShelved`), and no frame carries a path for the city to
  // install from; a button that is refused every time is a promise the
  // page cannot keep (`xtask wiring`). The folders are spelled with `/`
  // on every platform: they are read relative to the city's folder, and
  // Explorer, Finder and the Linux file managers all take that spelling.
  //
  // The welcome walk shows this much on its own, because a User who has
  // just built a city has nothing on the shelves to list yet.
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import type { Key } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import Button from "../parts/button.svelte";
  import { HALL } from "../shared/buildings";

  interface Props {
    // The building whose own shelf is shown beside the library.
    readonly building?: Address;
  }

  const { building = HALL }: Props = $props();

  const { lang } = ui();
  const belief = ui().conn.belief;
  // wording-ok: `<city>` marks a name the city has not answered with yet,
  // and the rest is a filesystem path - both are spelled the same in
  // every language.
  const city = $derived($belief.city ?? "<city>");
  const folders = $derived<readonly { readonly name: Key; readonly path: string }[]>([
    { name: "setup_skills", path: `${city}/.sprawling/library/` },
    { name: "setup_skills_building", path: `${city}/${building}/.sprawling/skills/` },
  ]);
</script>

<div class="flex flex-col gap-snug text-note text-text-quiet">
  {#each folders as folder (folder.name)}
    <p>{say($lang, folder.name)}</p>
    <div class="flex min-w-0 items-center gap-snug">
      <code class="min-w-0 truncate rounded-control bg-raised px-base py-snug font-mono text-text">{folder.path}</code>
      <Button
        label={say($lang, "setup_copy")}
        tone="quiet"
        onPress={() => {
          void navigator.clipboard.writeText(folder.path);
        }}
      />
    </div>
  {/each}
  <p class="text-text-faint">{say($lang, "setup_skills_how")}</p>
</div>
