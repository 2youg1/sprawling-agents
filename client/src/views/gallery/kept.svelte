<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // Both answers the settings page can give to "where does this choice
  // live".
  //
  // A running client reaches the second one only once the city has
  // answered `Query::Preferences` (`core/prefs_city.ts`), so a fixture
  // seats both: the badge is drawn, measured and read out at every page
  // width the gate opens, whichever state the page first paints in.

  import Kept from "../setup/kept.svelte";
  import Case from "./case.svelte";
</script>

<Case label="kept · this browser, until the city answers">
  <Kept keeper="browser" />
</Case>

<Case label="kept · the person's own config.toml">
  <Kept keeper="city" />
</Case>
