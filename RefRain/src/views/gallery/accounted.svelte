<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import Accounts from "../setup/providers/accounts.svelte";
  import { endpointRoster } from "../setup/providers/rosters";
  import Case from "./case.svelte";
  import { ACCOUNTED, UNLISTED } from "./configured";
</script>

<Case label="provider · accounts in order, each with its key">
  <Accounts roster={endpointRoster(ACCOUNTED, "two")} />
</Case>

<Case label="provider · attached with one key, no accounts yet">
  <Accounts roster={endpointRoster(UNLISTED, "two")} />
</Case>
