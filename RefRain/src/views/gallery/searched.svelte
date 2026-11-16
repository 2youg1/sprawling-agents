<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The web search card in the two states a person reads differently:
  // a city that never wrote `[search]`, and one that lists its own
  // services while the hall's file states something else.

  import SearchCard from "../setup/search/search.svelte";
  import Case from "./case.svelte";
  import { SEARCH, SEARCH_CUSTOM } from "./configured";
</script>

<Case label="search · the default service, nothing written">
  <SearchCard settled={SEARCH} />
</Case>

<Case label="search · two services of the city's own, the hall stating its own">
  <SearchCard settled={SEARCH_CUSTOM} />
</Case>
