<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Twelve rows in the shape the real ones take, while the city is being
// asked. The bars are `parts/skeleton`'s own (client-SPEC 6: that
// component's seat is this machine's waiting state) - a skeleton is the
// shape of the thing it waits for, so a person watches the page fill
// rather than jump.
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Skeleton from "../parts/skeleton.svelte";

  const { lang } = ui();
</script>

<Skeleton label={say($lang, "machine_checking")} rows={12} tall />
