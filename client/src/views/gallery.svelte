<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // Every state worth looking at, on fixtures, with no city behind it.
  //
  // It is a route rather than a build flag for two reasons: the gate
  // that measures it opens the same bundle a person runs, and a person
  // deciding between two ways a thing could look can open it on their
  // own machine without standing up a provider first.
  //
  // **This file is the index and nothing else.** Each section under
  // `gallery/` owns one subject - the room a person works in, what a
  // run produced, the registry, the shared controls - together with
  // the fixture values that subject needs, so a fixture is read beside
  // the component it is about rather than four hundred lines above it.
  // The order below is the order a reader meets them in, and
  // `gallery/case.svelte` holds the one wrapper they all draw into.
  //
  // **The hints lead on purpose.** A hint opens above its control and
  // flips below when the window has no room above, so the one fixture
  // that can exercise the flip is the one nearest the top of the
  // window; everything after it is ordered for a person reading down
  // the page.
</script>

<script lang="ts">
  import { onMount } from "svelte";

  import { say } from "../core/lang";
  import { setUi, ui } from "../ui";
  import Anchored from "./gallery/anchored.svelte";
  import Conversation from "./gallery/conversation.svelte";
  import Filed from "./gallery/filed.svelte";
  import Followed from "./gallery/followed.svelte";
  import Hints from "./gallery/hints.svelte";
  import Keepers from "./gallery/kept.svelte";
  import Parts from "./gallery/parts.svelte";
  import Presences from "./gallery/presence.svelte";
  import Produced from "./gallery/produced.svelte";
  import Screens from "./gallery/screens.svelte";
  import Settings from "./gallery/settings.svelte";
  import Shelved from "./gallery/shelved.svelte";
  import Switches from "./gallery/switches.svelte";
  import Tables from "./gallery/tables.svelte";

  const { lang } = ui();

  // The city as the shell opened it, captured before any stand below
  // installs one of its own. `Stand` hands `ui.ts`'s one door to a
  // fixture subtree while that subtree initialises; this closes the
  // door once the route has mounted, so a component arriving later -
  // the next page a person walks to, or a panel opened under a fixture
  // - meets the real city and not the last stand made.
  const real = ui();
  onMount(() => {
    setUi({
      conn: real.conn,
      prefs: real.prefs,
      bar: real.bar,
      origin: real.origin,
      pairing: real.pairing,
      now: real.now,
    });
  });
</script>

<div class="w-full min-w-0 px-pane py-pane">
  <h1 class="mb-wide text-heading text-text">{say($lang, "gallery_title")}</h1>
  <Hints />
  <Presences />
  <Conversation />
  <Followed />
  <Anchored />
  <Produced />
  <Filed />
  <Screens />
  <Shelved />
  <Keepers />
  <Settings />
  <Tables />
  <Parts />
  <Switches />
</div>
