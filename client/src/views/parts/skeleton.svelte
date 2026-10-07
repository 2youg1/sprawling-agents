<script lang="ts">
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The shape of what is coming, drawn while it is on its way. It has the
// same rows, the same heights and the same left edge as the real thing,
// so the page does not jump when the answer lands.
//
// The seat: the bars and what a screen reader is told are decided in
// `./skeleton` and drawn by `./skeleton.look.svelte`.

import Look from "./skeleton.look.svelte";
import { lookOf } from "./skeleton";
import type { SkeletonProps } from "./skeleton";

const props: SkeletonProps = $props();
const look = $derived(lookOf(props));
</script>

<Look {...look} />
