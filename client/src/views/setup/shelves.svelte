<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // Where a person drops a Markdown file to give the whole city one more
  // skill, and the copy button that puts it on the clipboard. The city
  // answers with its own name, so what is drawn is the folder as it is on
  // this machine rather than a shape to be filled in.
  //
  // The welcome walk shows this much on its own, because a person who has
  // just built a city has nothing on the shelves to list yet.
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Button from "../parts/button.svelte";

  const { lang } = ui();
  const belief = ui().conn.belief;
  // wording-ok: `<city>` marks a name the city has not answered with yet,
  // and the rest is a filesystem path - both are spelled the same in
  // every language.
  const path = $derived(`${$belief.city ?? "<city>"}/.sprawling/library/`);
</script>

<div class="flex flex-col gap-snug text-note text-text-quiet">
  <p>{say($lang, "setup_skills")}</p>
  <div class="flex items-center gap-snug">
    <code class="rounded-control bg-raised px-base py-snug font-mono text-text">{path}</code>
    <Button
      label={say($lang, "setup_copy")}
      tone="quiet"
      onPress={() => {
        void navigator.clipboard.writeText(path);
      }}
    />
  </div>
  <p class="text-text-faint">{say($lang, "setup_skills_note")}</p>
</div>
