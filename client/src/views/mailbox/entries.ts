// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Walking the mailbox's entries from the keyboard (client-SPEC 7-11):
// `j` and `k` move to the next and the previous entry, and a digit
// reaches the entry it is drawn beside. An entry is any element marked
// `data-entry`, in document order, so the sections need not know how
// many entries stand above them; the digit beside each is a CSS counter
// (`theme.css`, the mailbox block) counting the same marks in the same
// order, so the number drawn and the number pressed cannot disagree.
//
// The keys are the column's own, as a list's arrows are a list's own:
// letters typed into a field, or pressed with a modifier, are not
// theirs, and an entry the virtual list has not mounted is not one yet.

// How many entries a digit can reach: one to nine.
const DIGITS = 9;

function writing(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    target instanceof HTMLSelectElement ||
    (target instanceof HTMLElement && target.isContentEditable)
  );
}

function entriesIn(root: HTMLElement): HTMLElement[] {
  return [...root.querySelectorAll("[data-entry]")].filter((node) => node instanceof HTMLElement);
}

// An entry that goes somewhere is followed; one that is answered in
// place (a decide card) takes the focus, where its own keys are heard.
function reach(entry: HTMLElement): void {
  if (entry instanceof HTMLAnchorElement) entry.click();
  else entry.focus();
}

export function walk(root: HTMLElement, event: KeyboardEvent): void {
  if (event.ctrlKey || event.metaKey || event.altKey || event.isComposing || writing(event.target)) return;
  const entries = entriesIn(root);
  const digit = Number.parseInt(event.key, 10);
  if (event.key.length === 1 && digit >= 1 && digit <= DIGITS) {
    const entry = entries[digit - 1];
    if (entry === undefined) return;
    event.preventDefault();
    reach(entry);
    return;
  }
  const step = event.key === "j" ? 1 : event.key === "k" ? -1 : 0;
  if (step === 0) return;
  const target = event.target;
  const at = entries.findIndex((entry) => target instanceof Node && entry.contains(target));
  const next = entries[at < 0 ? 0 : Math.min(entries.length - 1, Math.max(0, at + step))];
  if (next === undefined) return;
  event.preventDefault();
  next.focus();
}
