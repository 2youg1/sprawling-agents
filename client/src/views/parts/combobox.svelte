<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts">
	// A choice among many, searched rather than scrolled: the control a
	// provider's model list needs, where a native select puts two hundred
	// rows behind one arrow.
	//
	// This is the seat: it holds the popup's state and the elements focus
	// and measuring need, and draws whatever `./combobox.look.svelte` is.
	// What a key or a press does is decided in `./combobox` (the key table
	// of client/Spec.lean §7-5); where the popup opens and how far its list
	// scrolls is `./layer`, the rule the popover follows too.

	import { tick, untrack } from "svelte";
	import type { Attachment } from "svelte/attachments";

	import { lookOf, matching, rowId } from "./combobox";
	import type { Answer, Choice, ComboboxProps, Hands, Layer, Return } from "./combobox";
	import Look from "./combobox.look.svelte";
	import { revealIn, sideFor } from "./layer";
	import type { Side } from "./layer";

	// A form's combobox opens below its trigger, where the eye already is.
	const PREFERRED: Side = "below";

	const props: ComboboxProps = $props();
	const uid = $props.id();

	let layer = $state<Layer>(untrack(() => props.starts) === "open" ? "open" : "unopened");
	let query = $state("");
	// Where the cursor wants to be. It is clamped where it is read rather
	// than reset where the list changes: a filter that shortens the list
	// must not be able to leave the cursor pointing past its end.
	let at = $state(0);
	// The rows a closing popup keeps while its look draws it away.
	let kept = $state<readonly Choice[]>([]);
	let side = $state<Side>(PREFERRED);

	let root = $state<HTMLDivElement | undefined>(undefined);
	let trigger = $state<HTMLButtonElement | undefined>(undefined);
	let panel = $state<HTMLDivElement | undefined>(undefined);
	let search = $state<HTMLInputElement | undefined>(undefined);
	let list = $state<HTMLUListElement | undefined>(undefined);

	const live = $derived(matching(props.choices, query));
	const rows = $derived(layer === "open" ? live : kept);
	const cursor = $derived(Math.max(0, Math.min(at, rows.length - 1)));

	// Closing resets the popup's own state - the filter and the cursor -
	// and the one thing that varies is where the focus goes (`Return`).
	function close(focus: Return): void {
		kept = live;
		layer = "shut";
		query = "";
		at = 0;
		if (focus === "opener") trigger?.focus();
	}

	function take(choice: Choice | undefined): void {
		if (choice === undefined) return;
		props.onPick(choice.value);
		close("opener");
	}

	function act(answered: Answer): void {
		switch (answered.kind) {
			case "cursor":
				at = answered.at;
				// A clamped key still reveals a cursor scrolled out by the wheel.
				void tick().then(revealCursor);
				return;
			case "take":
				take(rows.at(cursor));
				return;
			case "close":
				close(answered.focus);
				return;
			case "pass":
				return;
		}
	}

	function revealCursor(): void {
		if (layer !== "open" || list === undefined || rows.length === 0) return;
		const row = document.getElementById(rowId(uid, cursor));
		if (row !== null) revealIn(list, row);
	}

	const keep =
		<E extends HTMLElement>(set: (node: E | undefined) => void): Attachment<E> =>
		(node) => {
			set(node);
			return () => {
				set(undefined);
			};
		};

	const hands: Hands = {
		toggle: () => {
			if (layer === "open") {
				close("opener");
				return;
			}
			side = PREFERRED;
			layer = "open";
			// The filter takes the focus once the popup is drawn, so every key
			// of the table lands on the combobox rather than on the trigger.
			// Without scrolling: the popup sits beside the trigger the person
			// just pressed. Only an opening by the person takes the focus: a
			// popup drawn open on load that took it would lose it to the next
			// one drawn open, and a leaving focus closes the popup.
			void tick().then(() => search?.focus({ preventScroll: true }));
		},
		act,
		type: (typed) => {
			// Printable characters filter and put the cursor on the first row.
			query = typed;
			at = 0;
		},
		point: (index) => {
			at = index;
		},
		take,
		// Blurring away and clicking outside are one path: the focus leaves
		// the control. A click on a row is not that, because the list keeps
		// the focus where it is.
		leave: (next) => {
			if (layer !== "open") return;
			if (next instanceof Node && root?.contains(next) === true) return;
			close("leave");
		},
		holds: {
			root: keep((node) => {
				root = node;
			}),
			trigger: keep((node) => {
				trigger = node;
			}),
			layer: keep((node) => {
				panel = node;
			}),
			search: keep((node) => {
				search = node;
			}),
			list: keep((node) => {
				list = node;
			}),
		},
	};

	const look = $derived(lookOf({ props, uid, layer, query, cursor, rows, side }, hands));

	// Measured once per opening, on the side it was drawn on: a side that
	// changed with every letter typed would make the list jump while the
	// person reads it.
	$effect(() => {
		if (layer === "open" && panel !== undefined) side = sideFor(panel, PREFERRED);
	});

	// The cursor moves with a filter as well as with a key.
	$effect(revealCursor);
</script>

<Look {...look} />
