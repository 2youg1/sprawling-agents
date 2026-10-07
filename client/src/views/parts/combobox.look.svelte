<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts">
	// How a combobox is drawn, and nothing else: every word arrives
	// translated and every key and press arrives wired (`./combobox`,
	// `ComboboxLook`), and each wire bag is spread on the element it
	// names. Another look - one built from a component library - draws
	// the same control by taking the same value.
	//
	// The value in force is the deeper wash; the cursor is a bar at the
	// row's leading edge as well as a rung of the surface, because a
	// border keeps its shape when forced colours take every fill away.
	import type { ComboboxLook } from "./combobox";

	const look: ComboboxLook = $props();
</script>

<div class="relative flex w-full min-w-0 flex-col gap-tight" {...look.root}>
	<button
		class="flex h-control w-full min-w-0 items-center justify-between gap-snug rounded-control border border-edge-input bg-raised px-base text-body text-text hover:bg-raised-hover"
		{...look.trigger}
	>
		<span class={["truncate", look.unset && "text-text-faint"]}>{look.face}</span>
	</button>
	{#if look.layer !== undefined}
		<!-- A stacking number, because every combobox root is positioned:
			without one the next picker in the same form, later in the
			document, is painted over this list. -->
		<div
			class={[
				"layer absolute left-0 z-10 w-full flex-col rounded-panel border border-edge-panel bg-raised p-tight shadow-float",
				look.side === "below" ? "top-full mt-tight" : "bottom-full mb-tight",
				look.open && "open",
			]}
			data-side={look.side}
			{...look.layer}
		>
			<input
				class="mb-tight h-control w-full rounded-control bg-raised px-base text-body text-text placeholder:text-text-faint"
				{...look.search}
			/>
			<ul class="max-h-output overflow-y-auto" {...look.list}>
				{#each look.options as option (option.key)}
					<li
						class={[
							"relative flex cursor-pointer items-baseline justify-between gap-snug rounded-control px-base py-snug text-body",
							option.chosen ? "chosen wash-strong text-text" : option.cursor ? "bg-raised-hover text-text" : "text-text-quiet",
							option.cursor && "cursor",
						]}
						{...option.wire}
					>
						<span class="truncate">{option.label}</span>
						{#if option.note !== undefined}
							<span class="shrink-0 text-note text-text-faint">{option.note}</span>
						{/if}
					</li>
				{:else}
					<li role="presentation" class="px-base py-snug text-note text-text-faint">{look.empty}</li>
				{/each}
			</ul>
		</div>
	{/if}
</div>

<style>
	/* The popup arrives from the trigger's side and leaves the same way:
	 * arriving decelerates over `panel`, leaving accelerates over `short`,
	 * and `display` is held until the leaving ends. Every duration is a
	 * token, so a person who turned motion off sees it open and close in
	 * place. */
	.layer {
		display: none;
		opacity: 0;
		translate: 0 calc(-1 * var(--spacing-snug));
		transition:
			opacity var(--transition-duration-short) var(--ease-leave),
			translate var(--transition-duration-short) var(--ease-leave),
			display var(--transition-duration-short) allow-discrete;
	}
	.layer[data-side="above"] {
		translate: 0 var(--spacing-snug);
	}
	.layer.open {
		display: flex;
		opacity: 1;
		translate: 0 0;
		transition-duration: var(--transition-duration-panel);
		transition-timing-function: var(--ease-arrive);
	}
	@starting-style {
		.layer.open {
			opacity: 0;
			translate: 0 calc(-1 * var(--spacing-snug));
		}
		.layer.open[data-side="above"] {
			translate: 0 var(--spacing-snug);
		}
	}
	.cursor::before {
		content: "";
		position: absolute;
		inset-block: var(--spacing-tight);
		inset-inline-start: 0;
		border-inline-start: var(--spacing-hair) solid var(--color-accent);
	}
	/* Forced colours drop the wash and keep an outline. */
	.chosen {
		outline: 1px solid transparent;
		outline-offset: -1px;
	}
</style>
