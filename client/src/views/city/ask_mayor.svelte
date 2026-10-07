<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The seat of the way to the Mayor an empty page offers: it reads the
  // language and hands `./ask_mayor.ts`'s value to whatever
  // `./ask_mayor.look.svelte` is.
  import { ui } from "../../ui";
  import { lookOf } from "./ask_mayor";
  import Look from "./ask_mayor.look.svelte";

  const { lang } = ui();
</script>

<Look {...lookOf($lang)} />
