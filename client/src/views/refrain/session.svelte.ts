// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One document open in RefRain: which version the editor stands on, the
// draft kept for it, the save in flight and its receipt, the versions
// this page has held, and the city's version when it moved underneath
// (client/Spec.lean §4-46, §7N). The rules are the core's - coordinates in
// `document_pos.ts`, windows in `document_windows.ts`, the receipt in
// `document_save.ts` - and this file only puts them in the order a page
// meets them. Made while `refrain.svelte` initialises, so everything it
// asks stops with that component.

import { Option, Schema } from "effect";
import { untrack } from "svelte";
import { SvelteSet } from "svelte/reactivity";
import { get } from "svelte/store";

import { readAnswer } from "../../core/answered";
import { positionsOf } from "../../core/document_pos";
import type { EditorChange, Positions } from "../../core/document_pos";
import { RECEIPT_QUERY, advance, draftPlace, readDraft, receiptIn, saveOf, writeDraft } from "../../core/document_save";
import type { Happened, Receipt } from "../../core/document_save";
import { nextSpan, opened, recorded, whole } from "../../core/document_windows";
import type { Gathering, Opened } from "../../core/document_windows";
import type { Ui } from "../../ui";
import { Address } from "../../wire";
import type { Answer, AxError, B3Hash } from "../../wire";
import type { DocumentItem } from "../inspect/open.svelte";
import type { Editing } from "./editing";
import { Gathered } from "./gathered.svelte";

// Why the editor takes no typing, as the head line says it.
export type Locked =
  | { readonly kind: "gathering"; readonly through: number; readonly total: number }
  | { readonly kind: "too_large"; readonly through: number; readonly total: number }
  | { readonly kind: "recorded"; readonly version: B3Hash }
  | { readonly kind: "lost"; readonly version: B3Hash };

// A version whose whole text this page held, and where it came from.
export interface Held {
  readonly version: B3Hash;
  readonly text: string;
  readonly source: "opened" | "saved" | "moved";
  readonly at: number;
}

// What the editor mounts with: the baseline's editor text and a kept
// draft's changes on it.
export interface Opening {
  readonly text: string;
  readonly changes: readonly EditorChange[];
}

// How often a draft is written to the browser at most: once the typing
// pauses this long.
const DRAFT_PAUSE_MS = 300;

// The address a building's document is read at: the two joined, `null`
// when the join is not an address the city reads.
export function documentAt(building: Address, path: string): Address | null {
  return Option.getOrNull(Schema.decodeOption(Address)(`${building}/${path}`));
}

// The sessions that hold words the city has not taken, by document and
// version; the inspector's tab strip marks their tabs (docs/frontend-method.md §7F).
// A session leaves when it is saved, clean again, or closed.
const unsaved = new SvelteSet<string>();

const unsavedKey = (at: Address, version: B3Hash | null): string => `${at}@${version ?? ""}`;

export function holdsDraft(document: DocumentItem): boolean {
  const at = documentAt(document.building, document.path);
  return at !== null && unsaved.has(unsavedKey(at, document.version));
}

export class Session {
  answer = $state<Answer | undefined>(undefined);
  positions = $state<Positions | null>(null);
  opening = $state<Opening | null>(null);
  receipt = $state<Receipt>({ kind: "clean" });
  theirs = $state<Positions | null>(null);
  versions = $state<readonly Held[]>([]);
  lostDraft = $state<B3Hash | null>(null);
  editing = $state.raw<Editing | null>(null);

  readonly read = $derived(readAnswer(this.answer, (held) => ("document" in held ? held.document : undefined)));
  readonly file = $derived<Opened | null>(this.read.kind === "held" ? opened(this.read.value) : null);
  readonly locked = $derived.by((): Locked | null => this.lockOf());

  private readonly current: Gathered;
  private readonly other: Gathered;
  private expected: B3Hash | null = null;
  private pause: ReturnType<typeof setTimeout> | undefined;

  constructor(
    private readonly ui: Ui,
    private readonly at: Address,
    private readonly asked: B3Hash | null,
  ) {
    this.current = new Gathered(ui.conn.asking);
    this.other = new Gathered(ui.conn.asking);
    $effect(() => ui.conn.asking.ask({ document: { at } }).subscribe((answer) => (this.answer = answer)));
    $effect(() => {
      const file = this.file;
      if (file?.kind === "text" && file.gathering.version !== untrack(() => this.current.value?.version)) {
        untrack(() => {
          this.current.start(file.gathering);
        });
      }
    });
    $effect(() => {
      this.settle(this.current.value, this.other.value);
    });
    $effect(() => this.followReceipts());
    $effect(() => this.followRefusals());
    $effect(() => this.followLink());
    $effect(() => () => {
      this.keep();
    });
    $effect(() => {
      const kind = this.receipt.kind;
      if (kind === "clean" || kind === "saved") return;
      const key = unsavedKey(at, asked);
      unsaved.add(key);
      return () => {
        unsaved.delete(key);
      };
    });
  }

  // ------------------------------------------------------------ opening

  // Decides the baseline once the version it stands on is in, then
  // follows the city's version as it moves.
  private settle(current: Gathering | null, other: Gathering | null): void {
    if (current === null) return;
    untrack(() => {
      if (this.positions === null) this.open(current, other);
      else if (settled(current, this.current.lost)) this.moved(current);
    });
  }

  private open(current: Gathering, other: Gathering | null): void {
    if (this.asked !== null && this.asked !== current.version) {
      if (other === null) this.other.start(recorded(this.asked, current.format));
      else if (settled(other, this.other.lost)) this.begin(other, [], { kind: "clean" });
      return;
    }
    const kept = this.ui.prefs.draft(draftPlace(this.at));
    const version = kept === "" ? null : readDraft(kept, Number.MAX_SAFE_INTEGER)?.version ?? null;
    if (!settled(current, this.current.lost)) return;
    if (version === null || version === current.version) {
      const changes = readDraft(kept, positionsOf(current.version, current.encoding, current.text).editor.length)?.changes ?? [];
      this.begin(current, changes, changes.length === 0 ? { kind: "clean" } : { kind: "draft" });
      return;
    }
    if (other === null) {
      this.other.start(recorded(version, current.format));
      return;
    }
    if (!settled(other, this.other.lost)) return;
    if (this.other.lost !== null) {
      this.lostDraft = version;
      this.begin(current, [], { kind: "clean" });
      return;
    }
    const base = positionsOf(other.version, other.encoding, other.text);
    this.begin(other, readDraft(kept, base.editor.length)?.changes ?? [], { kind: "conflict", current: current.version });
    this.theirs = positionsOf(current.version, current.encoding, current.text);
    this.hold(this.theirs, "moved");
  }

  private begin(on: Gathering, changes: Opening["changes"], receipt: Receipt): void {
    const positions = positionsOf(on.version, on.encoding, on.text);
    this.positions = positions;
    this.receipt = receipt;
    this.opening = { text: positions.editor, changes };
    this.hold(positions, "opened");
  }

  private hold(positions: Positions, source: Held["source"]): void {
    if (this.versions.some((each) => each.version === positions.version)) return;
    this.versions = [...this.versions, { version: positions.version, text: positions.editor, source, at: this.ui.now() }];
  }

  // The city answered another version of the file than the baseline:
  // this page's own save landing, or somebody else's write.
  private moved(current: Gathering): void {
    const base = this.positions;
    if (base === null || current.version === base.version || this.asked !== null) return;
    const theirs = positionsOf(current.version, current.encoding, current.text);
    if (current.version === this.expected) {
      this.expected = null;
      this.positions = theirs;
      this.hold(theirs, "saved");
      return;
    }
    this.hold(theirs, "moved");
    this.theirs = theirs;
    const was = this.receipt.kind;
    this.happen({ kind: "moved", version: current.version });
    if ((was === "clean" || was === "saved") && this.editing !== null) {
      this.editing.follow(theirs.editor);
      this.positions = theirs;
      this.theirs = null;
    }
  }

  private lockOf(): Locked | null {
    const current = this.current.value;
    if (this.asked !== null && current !== null && this.asked !== current.version) {
      return this.other.lost === null ? { kind: "recorded", version: this.asked } : { kind: "lost", version: this.asked };
    }
    if (current === null) return null;
    if (this.current.lost !== null) return { kind: "lost", version: current.version };
    if (whole(current)) return null;
    const total = current.bytes ?? current.through;
    return nextSpan(current) === null
      ? { kind: "too_large", through: current.through, total }
      : { kind: "gathering", through: current.through, total };
  }

  // ------------------------------------------------------------ editing

  attach(editing: Editing): void {
    this.editing = editing;
    editing.readOnly(this.locked !== null);
  }

  edited(): void {
    const editing = this.editing;
    if (editing === null) return;
    const flying = editing.editedSinceSent();
    this.happen({ kind: "edited", empty: flying === null ? editing.changes().length === 0 : !flying });
    clearTimeout(this.pause);
    this.pause = setTimeout(() => {
      this.keep();
    }, DRAFT_PAUSE_MS);
  }

  // The draft as the browser keeps it: against the version the editor
  // stands on, or the version a landed save left once it is known.
  private keep(): void {
    const editing = this.editing;
    const base = this.positions;
    if (editing === null || base === null || this.locked !== null) return;
    this.ui.prefs.setDraft(
      draftPlace(this.at),
      writeDraft({ version: this.expected ?? base.version, changes: editing.changes() }),
    );
  }

  save(): void {
    const editing = this.editing;
    const base = this.positions;
    const kind = this.receipt.kind;
    if (editing === null || base === null || this.locked !== null || editing.composing()) return;
    if (kind !== "draft" && kind !== "refused") return;
    const sent = saveOf(this.at, base, editing.changes());
    if (!this.ui.conn.command(sent.command)) return;
    editing.sent();
    this.happen({ kind: "sent", sent });
  }

  // Carries the draft onto the city's version, or drops it there.
  rebase(): void {
    const theirs = this.theirs;
    const editing = this.editing;
    if (theirs === null || editing === null) return;
    editing.rebase(theirs.editor);
    this.based(theirs, editing.changes().length === 0);
  }

  discard(): void {
    const theirs = this.theirs;
    const editing = this.editing;
    if (theirs === null || editing === null) return;
    editing.follow(theirs.editor);
    this.based(theirs, true);
  }

  private based(theirs: Positions, empty: boolean): void {
    this.positions = theirs;
    this.theirs = null;
    this.happen({ kind: "based", empty });
    this.keep();
  }

  private happen(happened: Happened): void {
    this.receipt = advance(this.receipt, happened);
  }

  // ------------------------------------------------------------ the city

  private followReceipts(): (() => void) | undefined {
    const receipt = this.receipt;
    if (receipt.kind !== "saving" && receipt.kind !== "pending") return undefined;
    return this.ui.conn.asking.ask(RECEIPT_QUERY).subscribe((answer) => {
      const read = readAnswer(answer, (held) => ("history" in held ? held.history : undefined));
      const version = read.kind === "held" ? receiptIn(read.value.records, receipt.sent) : null;
      if (version === null) return;
      untrack(() => {
        this.landed(version);
      });
    });
  }

  // A save of this page landed. The city's answer for the new version
  // may already be in, read as somebody else's write; it is this save's.
  private landed(version: B3Hash): void {
    this.editing?.landed();
    this.happen({ kind: "landed", version });
    const theirs = this.theirs;
    if (theirs?.version === version) {
      this.positions = theirs;
      this.theirs = null;
      this.versions = this.versions.map((each) => (each.version === version ? { ...each, source: "saved" } : each));
    } else {
      this.expected = version;
    }
    this.keep();
  }

  private followRefusals(): () => void {
    let last: AxError | null = get(this.ui.conn.belief).refusal;
    return this.ui.conn.belief.subscribe((belief) => {
      const error = belief.refusal;
      if (error === null || error === last) return;
      last = error;
      untrack(() => {
        this.happen({ kind: "refusal", error });
        const kind = this.receipt.kind;
        if (kind === "saving" || kind === "pending") return;
        this.editing?.unsent();
        if (kind === "conflict") this.ui.conn.asking.refresh({ document: { at: this.at } });
      });
    });
  }

  private followLink(): () => void {
    let live = get(this.ui.conn.state).kind === "live";
    return this.ui.conn.state.subscribe((state) => {
      const now = state.kind === "live";
      if (now === live) return;
      live = now;
      untrack(() => {
        const receipt = this.receipt;
        if (!now && receipt.kind === "saving") this.happen({ kind: "lost" });
        if (now && receipt.kind === "pending" && this.ui.conn.command(receipt.sent.command)) {
          this.happen({ kind: "relinked" });
        }
      });
    });
  }
}

// Whether a gathering has gone as far as it will: whole, past the bound,
// or stopped by a window the city could not read.
function settled(gathering: Gathering, lost: string | null): boolean {
  return lost !== null || nextSpan(gathering) === null;
}
