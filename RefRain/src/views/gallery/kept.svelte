<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import Kept from "../setup/kept.svelte";
  import Case from "./case.svelte";
</script>

<Case label="kept · this browser, until the city answers">
  <Kept keeper="browser" />
</Case>

<Case label="kept · the person's own config.toml">
  <Kept keeper="city" />
</Case>
