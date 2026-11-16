<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The seat of the notice (`./notice`): it reads the props in the
  // reader's language and draws whatever `./notice.look.svelte` is.
  import { ui } from "../../ui";
  import { noticeOf } from "./notice";
  import type { NoticeProps } from "./notice";
  import Look from "./notice.look.svelte";

  // Taken whole rather than destructured: spreading the rest of a union
  // loses the field that tells its two shapes apart.
  const props: NoticeProps = $props();

  const { lang } = ui();
</script>

<Look {...noticeOf(props, $lang)} />
