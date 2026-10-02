<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts" module>
	// The question asked before something that cannot be taken back. An
	// action that can be undone is done straight away and offered back as
	// a Notice; only the irreversible one stops and explains first.
	//
	// **The platform owns the modality.** `showModal()` puts the element
	// in the top layer, traps the focus, marks the rest of the page
	// `inert`, and answers Escape - four behaviours this file used to
	// carry as a wrapper, a keydown handler, a stacking order and a focus
	// call. The top layer also means no `z-index`: nothing on the page can
	// be given a number that puts it over a modal dialog.
	//
	// The focus goes to the way out rather than to the way through: on a
	// question about deleting something, the safe answer is the one
	// already under the hand. Document order is what says so - the
	// platform focuses the first control inside the dialog, and the
	// cancel button is written before the confirming one.
	//
	// **The scrim is the dialog's own backdrop, dimmed rather than
	// washed.** `::backdrop` reaches the page's colour tokens only where
	// an engine inherits custom properties into it, and a token that
	// fails to resolve leaves `background-color` at its initial value - a
	// modal with no scrim at all, on an engine nobody tested. A
	// brightness filter asks the platform for no colour, so it darkens
	// the page under both lightings and cannot fail quietly.

	export interface DialogProps {
		readonly open: boolean;
		// Already in the person's language: what is about to happen.
		readonly title: string;
		// What it will cost, in the words of the page.
		readonly detail?: string;
		readonly confirmLabel: string;
		readonly cancelLabel: string;
		readonly onConfirm: () => void;
		readonly onCancel: () => void;
		// Whether the confirming answer destroys something.
		readonly destructive?: boolean;
	}
</script>

<script lang="ts">
	import Button from "./button.svelte";

	const {
		open,
		title,
		detail,
		confirmLabel,
		cancelLabel,
		onConfirm,
		onCancel,
		destructive = false,
	}: DialogProps = $props();

	// One instance, one id root: the title and the detail derive their
	// ids from it, so the two ARIA references name elements that are
	// unique per instance and stable across hydration.
	const uid = $props.id();
	const heading = `${uid}-title`;
	const body = `${uid}-detail`;

	// The element is held as state rather than plain storage so that the
	// effect below runs again when the ref arrives, whichever order the
	// first render and the first `open` happen in.
	let sheet = $state<HTMLDialogElement | undefined>(undefined);

	// The caller owns whether the question stands; this only carries that
	// answer to the element. Asking an already-open dialog to open throws,
	// so each call is made only on the edge it belongs to.
	$effect(() => {
		if (sheet === undefined) return;
		if (open) {
			if (!sheet.open) sheet.showModal();
			return;
		}
		if (sheet.open) sheet.close();
	});
</script>

<!-- Open and closed are one property with two displays, so the opening
	and the closing are the same transition read in two directions.
	`display` and `overlay` are discrete properties: without
	`transition-discrete` the box would vanish on the first frame of the
	closing and take its fade with it. A shadowed face draws no border
	(client-SPEC 4-34). -->
<dialog
	bind:this={sheet}
	class="m-auto hidden w-full max-w-measure flex-col gap-base rounded-panel bg-raised p-pane opacity-0 shadow-sheet transition-[opacity,display,overlay] transition-discrete duration-panel ease-leave open:flex open:opacity-100 open:ease-arrive starting:open:opacity-0 motion-reduce:transition-none backdrop:bg-transparent backdrop:backdrop-brightness-50"
	aria-labelledby={heading}
	aria-describedby={detail === undefined ? undefined : body}
	oncancel={(event) => {
		// Escape reaches here as a cancel request. The default would close
		// the element behind the caller's back and leave `open` saying it
		// is still up, so the request is answered by the caller instead.
		event.preventDefault();
		onCancel();
	}}
>
	<h2 id={heading} class="text-heading text-text">{title}</h2>
	{#if detail !== undefined}
		<p id={body} class="text-note text-text-quiet">{detail}</p>
	{/if}
	<div class="flex items-center justify-end gap-snug">
		<!-- The way out is written first, so it is the control the
			platform puts under the hand when the dialog opens. -->
		<Button label={cancelLabel} tone="secondary" onPress={onCancel} />
		<Button label={confirmLabel} tone={destructive ? "destructive" : "primary"} onPress={onConfirm} />
	</div>
</dialog>
