<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts" module>
	// A choice among many, searched rather than scrolled: the control a
	// provider's model list needs, where a native select puts two hundred
	// rows behind one arrow.
	//
	// The keyboard owns it: the trigger opens, typing filters, the arrows
	// move, Enter picks, Escape closes and gives the focus back to the
	// trigger it came from. The filter box is the combobox (WAI-ARIA),
	// the list is its listbox, and the cursor travels as
	// `aria-activedescendant` from the filter to the row it is on.

	export interface Choice {
		readonly value: string;
		// Already in the person's language, or an identifier they typed.
		readonly label: string;
		// The second line: a provider, a price, a context window.
		readonly note?: string;
	}

	export interface ComboboxProps {
		// The accessible name of the control.
		readonly label: string;
		// What the trigger says while nothing is chosen, and what the search
		// box says while it is empty.
		readonly placeholder: string;
		// What stands in the list when the search matches nothing.
		readonly empty: string;
		readonly choices: readonly Choice[];
		// The value in force; the cursor is never marked as chosen.
		readonly value: string | null;
		// The picked value; closing and focus return stay in this file.
		readonly onPick: (value: string) => void;
		// How the popup is drawn before anyone touches it. Every screen
		// starts closed; the gallery draws it open so the list is measured.
		readonly starts?: "open" | "closed";
	}
</script>

<script lang="ts">
	import { untrack } from "svelte";

	const { label, placeholder, empty, choices, value, onPick, starts = "closed" }: ComboboxProps =
		$props();

	// One instance, one id root: the list and the rows derive their ids
	// from it, so `aria-controls` and `aria-activedescendant` always name
	// elements that are there.
	const uid = $props.id();

	let open = $state(untrack(() => starts) === "open");
	let query = $state("");
	// Where the cursor wants to be. It is clamped where it is read rather
	// than reset where the list changes: a filter that shortens the list
	// must not be able to leave the cursor pointing past its end.
	let at = $state(0);

	let trigger = $state<HTMLButtonElement | undefined>(undefined);
	let search = $state<HTMLInputElement | undefined>(undefined);
	let root = $state<HTMLDivElement | undefined>(undefined);

	const matches = $derived.by(() => {
		const needle = query.trim().toLowerCase();
		if (needle === "") return choices;
		return choices.filter((choice) =>
			`${choice.label} ${choice.note ?? ""} ${choice.value}`.toLowerCase().includes(needle),
		);
	});
	const cursor = $derived(Math.max(0, Math.min(at, matches.length - 1)));
	const chosen = $derived(choices.find((each) => each.value === value));

	// The filter takes the focus the moment the popup opens, so every key
	// in the table below lands on the combobox rather than on the trigger.
	// Without scrolling: the popup sits under the trigger the person just
	// pressed, and a popup drawn open on load must not move the page.
	$effect(() => {
		if (open && search !== undefined) search.focus({ preventScroll: true });
	});

	// Closing resets the popup's own state - the filter and the cursor -
	// and the one thing that varies is where the focus goes: a closing
	// that answers the person (Enter, Escape) hands it back to the trigger
	// they came from, a closing that obeys a leaving focus (Tab, a click
	// elsewhere) leaves the focus where it is already going.
	function close(focus: "opener" | "leave"): void {
		open = false;
		query = "";
		at = 0;
		if (focus === "opener") trigger?.focus();
	}

	function toggle(): void {
		if (open) {
			close("opener");
			return;
		}
		open = true;
	}

	function take(choice: Choice | undefined): void {
		if (choice === undefined) return;
		onPick(choice.value);
		close("opener");
	}

	// The key table of client/Spec.lean §7-5, an if-chain over the one string a
	// keyboard event carries; a key outside the table is the platform's,
	// not this component's. The trigger shares this handler only so that
	// Escape answers there too; every other key belongs to the combobox
	// and falls through on the trigger.
	function onKeydown(event: KeyboardEvent): void {
		if (!open) return;
		if (event.target !== search) {
			if (event.key === "Escape") {
				event.preventDefault();
				close("opener");
			}
			return;
		}
		if (event.key === "ArrowDown") {
			event.preventDefault();
			at = cursor + 1;
			return;
		}
		if (event.key === "ArrowUp") {
			event.preventDefault();
			at = cursor - 1;
			return;
		}
		if (event.key === "Home") {
			event.preventDefault();
			at = 0;
			return;
		}
		if (event.key === "End") {
			event.preventDefault();
			at = matches.length - 1;
			return;
		}
		if (event.key === "Enter") {
			event.preventDefault();
			take(matches[cursor]);
			return;
		}
		if (event.key === "Escape") {
			event.preventDefault();
			close("opener");
			return;
		}
		// Tab closes the popup and lets the focus move on: no
		// preventDefault, and no focus handed to the trigger.
		if (event.key === "Tab") close("leave");
	}

	// Blurring away or clicking outside is one path: the focus leaves the
	// component, and a click on the list rows is not that, because the
	// list keeps the focus where it is (see its mousedown below).
	function onFocusout(event: FocusEvent): void {
		if (!open) return;
		const next = event.relatedTarget;
		if (next instanceof Node && root?.contains(next)) return;
		close("leave");
	}
</script>

<div
	class="relative flex w-full min-w-0 flex-col gap-tight"
	bind:this={root}
	onfocusout={onFocusout}
>
	<button
		type="button"
		bind:this={trigger}
		class="flex h-control w-full min-w-0 items-center justify-between gap-snug rounded-control border border-edge-input bg-raised px-base text-body text-text hover:bg-raised-hover"
		aria-label={label}
		onclick={toggle}
		onkeydown={onKeydown}
	>
		<span class="truncate">
			{#if chosen !== undefined}
				{chosen.label}
			{:else}
				<span class="text-text-faint">{placeholder}</span>
			{/if}
		</span>
	</button>
	{#if open}
		<!-- A stacking number, because every combobox root is positioned:
			without one the next picker in the same form, later in the
			document, is painted over this list and leaves one row of its
			own trigger where the models should be. The popup enters as
			`rise` and leaves as a cut. -->
		<div
			class="rise absolute top-full left-0 z-10 mt-tight flex w-full flex-col rounded-panel border border-edge-panel bg-raised p-tight shadow-float"
		>
			<input
				bind:this={search}
				bind:value={query}
				class="mb-tight h-control w-full rounded-control bg-raised px-base text-body text-text placeholder:text-text-faint"
				{placeholder}
				aria-label={label}
				role="combobox"
				aria-expanded={open}
				aria-controls="{uid}-list"
				aria-activedescendant={matches.length > 0 ? `${uid}-${String(cursor)}` : undefined}
				oninput={() => {
					// Printable characters filter and reset the cursor to
					// the first row.
					at = 0;
				}}
				onkeydown={onKeydown}
			/>
			<ul
				id="{uid}-list"
				class="max-h-output overflow-y-auto"
				role="listbox"
				aria-label={label}
				onmousedown={(event) => {
					// A pick by pointer must not move the focus first: the
					// blur would close the popup before the click landed.
					event.preventDefault();
				}}
			>
				{#each matches as choice, index (choice.value)}
					<!-- svelte-ignore a11y_click_events_have_key_events (the row is picked by pointer; the key table belongs to the combobox, which holds the focus) -->
					<li
						id="{uid}-{index}"
						role="option"
						aria-selected={choice.value === value}
						class={[
							"flex cursor-pointer items-baseline justify-between gap-snug rounded-control px-base py-snug text-body",
							index === cursor ? "bg-raised-hover text-text" : "text-text-quiet",
						]}
						onmouseenter={() => {
							at = index;
						}}
						onclick={() => {
							take(choice);
						}}
					>
						<span class="truncate">{choice.label}</span>
						{#if choice.note !== undefined}
							<span class="shrink-0 text-note text-text-faint">{choice.note}</span>
						{/if}
					</li>
				{:else}
					<li role="presentation" class="px-base py-snug text-note text-text-faint">{empty}</li>
				{/each}
			</ul>
		</div>
	{/if}
</div>
