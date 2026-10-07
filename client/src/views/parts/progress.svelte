<script lang="ts">
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How far along something is, as a bar and as the two numbers the bar
// was drawn from. The numbers are there because a bar alone cannot be
// read out, compared, or believed.
//
// The seat: what the bar says is decided in `./progress` and drawn by
// `./progress.look.svelte`.

import Look from "./progress.look.svelte";
import { lookOf } from "./progress";
import type { ProgressProps } from "./progress";

const props: ProgressProps = $props();
const look = $derived(lookOf(props));
</script>

<Look {...look} />
