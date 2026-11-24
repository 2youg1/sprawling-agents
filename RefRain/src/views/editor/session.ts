// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option, Schema } from "effect";
import { derived, get, readonly, writable } from "../../core/store";

import { readAnswer } from "../../core/answered";
import { positionsOf } from "../../core/document_pos";
import type { EditorChange, Positions } from "../../core/document_pos";
import {
  RECEIPT_QUERY,
  advance,
  draftPlace,
  readDraft,
  receiptIn,
  saveOf,
  writeDraft,
} from "../../core/document_save";
import type { Happened, Receipt } from "../../core/document_save";
import { nextSpan, opened, recorded, whole } from "../../core/document_windows";
import type { Gathering, Opened } from "../../core/document_windows";
import type { Ui } from "../../ui";
import { Address } from "../../wire";
import type { Answer, AxError, B3Hash } from "../../wire";
import type { DocumentItem } from "../inspect/open";
import type { Editing } from "./editing";
import { Gathered } from "./gathered";

export type Locked =
  | { readonly kind: "gathering"; readonly through: number; readonly total: number }
  | { readonly kind: "too_large"; readonly through: number; readonly total: number }
  | { readonly kind: "recorded"; readonly version: B3Hash }
  | { readonly kind: "lost"; readonly version: B3Hash };

export interface Held {
  readonly version: B3Hash;
  readonly text: string;
  readonly source: "opened" | "saved" | "moved";
  readonly at: number;
}

export interface Opening {
  readonly text: string;
  readonly changes: readonly EditorChange[];
}

const DRAFT_PAUSE_MS = 300;

export function documentAt(building: Address, path: string): Address | null {
  return Option.getOrNull(
    Schema.decodeOption(Address)(path.startsWith("/") ? path.slice(1) : `${building}/${path}`),
  );
}

const unsaved = writable<ReadonlyMap<string, number>>(new Map());
export const pendingDocuments = readonly(unsaved);

const unsavedKey = (at: Address, version: B3Hash | null): string => `${at}@${version ?? ""}`;

export function holdsDraft(document: DocumentItem, pending = get(unsaved)): boolean {
  const at = documentAt(document.building, document.path);
  return at !== null && pending.has(unsavedKey(at, document.version));
}

interface SessionFields {
  readonly answer: Answer | undefined;
  readonly positions: Positions | null;
  readonly opening: Opening | null;
  readonly receipt: Receipt;
  readonly theirs: Positions | null;
  readonly versions: readonly Held[];
  readonly lostDraft: B3Hash | null;
  readonly editing: Editing | null;
}

type PendingSend = Extract<Receipt, { kind: "saving" | "pending" }>["sent"];

export class Session {
  private readonly data = writable<SessionFields>({
    answer: undefined,
    positions: null,
    opening: null,
    receipt: { kind: "clean" },
    theirs: null,
    versions: [],
    lostDraft: null,
    editing: null,
  });
  readonly state = derived(this.data, (held) => ({
    ...held,
    read: this.read,
    file: this.file,
    locked: this.locked,
  }));

  get answer(): Answer | undefined {
    return get(this.data).answer;
  }
  private set answer(value: Answer | undefined) {
    this.data.update((held) => ({ ...held, answer: value }));
  }

  get positions(): Positions | null {
    return get(this.data).positions;
  }
  private set positions(value: Positions | null) {
    this.data.update((held) => ({ ...held, positions: value }));
  }

  get opening(): Opening | null {
    return get(this.data).opening;
  }
  private set opening(value: Opening | null) {
    this.data.update((held) => ({ ...held, opening: value }));
  }

  get receipt(): Receipt {
    return get(this.data).receipt;
  }
  private set receipt(value: Receipt) {
    this.data.update((held) => ({ ...held, receipt: value }));
    this.heldDraft(value.kind !== "clean" && value.kind !== "saved");
    this.watchReceipt();
  }

  get theirs(): Positions | null {
    return get(this.data).theirs;
  }
  private set theirs(value: Positions | null) {
    this.data.update((held) => ({ ...held, theirs: value }));
  }

  get versions(): readonly Held[] {
    return get(this.data).versions;
  }
  private set versions(value: readonly Held[]) {
    this.data.update((held) => ({ ...held, versions: value }));
  }

  get lostDraft(): B3Hash | null {
    return get(this.data).lostDraft;
  }
  private set lostDraft(value: B3Hash | null) {
    this.data.update((held) => ({ ...held, lostDraft: value }));
  }

  get editing(): Editing | null {
    return get(this.data).editing;
  }
  private set editing(value: Editing | null) {
    this.data.update((held) => ({ ...held, editing: value }));
  }

  get read() {
    return readAnswer(this.answer, (held) => ("document" in held ? held.document : undefined));
  }

  get file(): Opened | null {
    const read = this.read;
    return read.kind === "held" ? opened(read.value) : null;
  }

  get locked(): Locked | null {
    return this.lockOf();
  }

  private readonly current: Gathered;
  private readonly other: Gathered;
  private expected: B3Hash | null = null;
  private pause: ReturnType<typeof setTimeout> | undefined;
  private keepAt = 0;
  private readonly stops: (() => void)[] = [];
  private receiptStop: (() => void) | undefined;
  private followed: PendingSend | null = null;
  private held = false;

  constructor(
    private readonly ui: Ui,
    private readonly at: Address,
    private readonly asked: B3Hash | null,
  ) {
    this.current = new Gathered(ui.conn.asking);
    this.other = new Gathered(ui.conn.asking);
    const settle = () => {
      this.settle(this.current.value, this.other.value);
      this.data.update((held) => ({ ...held }));
    };
    this.stops.push(
      this.current.state.subscribe(settle),
      this.other.state.subscribe(settle),
      ui.conn.asking.ask({ document: { at } }).subscribe((answer) => {
        this.answer = answer;
        const file = this.file;
        if (file?.kind === "text" && file.gathering.version !== this.current.value?.version) {
          this.current.start(file.gathering);
        }
      }),
      this.followRefusals(),
      this.followLink(),
    );
  }

  close(): void {
    this.keep();
    this.receiptStop?.();
    this.receiptStop = undefined;
    for (const stop of this.stops.splice(0)) stop();
    this.current.close();
    this.other.close();
    this.heldDraft(false);
  }

  private heldDraft(active: boolean): void {
    if (active === this.held) return;
    this.held = active;
    const key = unsavedKey(this.at, this.asked);
    unsaved.update((pending) => {
      const next = new Map(pending);
      const count = (next.get(key) ?? 0) + (active ? 1 : -1);
      if (count === 0) next.delete(key);
      else next.set(key, count);
      return next;
    });
  }

  private settle(current: Gathering | null, other: Gathering | null): void {
    if (current === null) return;
    if (this.positions === null) this.open(current, other);
    else if (settled(current, this.current.lost)) this.moved(current);
  }

  private open(current: Gathering, other: Gathering | null): void {
    if (this.asked !== null && this.asked !== current.version) {
      if (other === null) this.other.start(recorded(this.asked, current.format));
      else if (settled(other, this.other.lost)) this.begin(other, [], { kind: "clean" });
      return;
    }
    const kept = this.ui.prefs.draft(draftPlace(this.at));
    const version =
      kept === "" ? null : (readDraft(kept, Number.MAX_SAFE_INTEGER)?.version ?? null);
    if (!settled(current, this.current.lost)) return;
    if (version === null || version === current.version) {
      const changes =
        readDraft(kept, positionsOf(current.version, current.encoding, current.text).editor.length)
          ?.changes ?? [];
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
    this.begin(other, readDraft(kept, base.editor.length)?.changes ?? [], {
      kind: "conflict",
      current: current.version,
    });
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
    this.versions = [
      ...this.versions,
      { version: positions.version, text: positions.editor, source, at: this.ui.now() },
    ];
  }

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
      return this.other.lost === null
        ? { kind: "recorded", version: this.asked }
        : { kind: "lost", version: this.asked };
    }
    if (current === null) return null;
    if (this.current.lost !== null) return { kind: "lost", version: current.version };
    if (whole(current)) return null;
    const total = current.bytes ?? current.through;
    return nextSpan(current) === null
      ? { kind: "too_large", through: current.through, total }
      : { kind: "gathering", through: current.through, total };
  }

  attach(editing: Editing): void {
    this.editing = editing;
    editing.readOnly(this.locked !== null);
  }

  edited(): void {
    const editing = this.editing;
    if (editing === null) return;
    const flying = editing.editedSinceSent();
    const empty = flying === null ? !editing.hasChanges() : !flying;
    const receipt = this.receipt;
    const same =
      (receipt.kind === "clean" && empty) ||
      (receipt.kind === "draft" && !empty) ||
      ((receipt.kind === "saving" || receipt.kind === "pending") && receipt.edited === !empty);
    if (!same) this.happen({ kind: "edited", empty });
    this.keepAt = performance.now() + DRAFT_PAUSE_MS;
    this.pause ??= setTimeout(this.keepWhenQuiet, DRAFT_PAUSE_MS);
  }

  private readonly keepWhenQuiet = (): void => {
    const remaining = this.keepAt - performance.now();
    if (remaining > 0) this.pause = setTimeout(this.keepWhenQuiet, remaining);
    else this.keep();
  };

  private keep(): void {
    clearTimeout(this.pause);
    this.pause = undefined;
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
    const held = this.receipt;
    const next = advance(held, happened);
    if (held !== next) this.receipt = next;
  }

  private watchReceipt(): void {
    const receipt = this.receipt;
    const sent = receipt.kind === "saving" || receipt.kind === "pending" ? receipt.sent : null;
    if (sent === this.followed) return;
    this.followed = sent;
    this.receiptStop?.();
    this.receiptStop = undefined;
    if (sent === null) return;
    const stop = this.ui.conn.asking.ask(RECEIPT_QUERY).subscribe((answer) => {
      if (sent !== this.followed) return;
      const read = readAnswer(answer, (held) => ("history" in held ? held.history : undefined));
      const version = read.kind === "held" ? receiptIn(read.value.records, sent) : null;
      if (version !== null) this.landed(version);
    });
    if (sent === this.followed) this.receiptStop = stop;
    else stop();
  }

  private landed(version: B3Hash): void {
    this.editing?.landed();
    this.happen({ kind: "landed", version });
    const theirs = this.theirs;
    if (theirs?.version === version) {
      this.positions = theirs;
      this.theirs = null;
      this.versions = this.versions.map((each) =>
        each.version === version ? { ...each, source: "saved" } : each,
      );
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
      this.happen({ kind: "refusal", error });
      const kind = this.receipt.kind;
      if (kind === "saving" || kind === "pending") return;
      this.editing?.unsent();
      if (kind === "conflict") this.ui.conn.asking.refresh({ document: { at: this.at } });
    });
  }

  private followLink(): () => void {
    let live = get(this.ui.conn.state).kind === "live";
    return this.ui.conn.state.subscribe((state) => {
      const now = state.kind === "live";
      if (now === live) return;
      live = now;
      const receipt = this.receipt;
      if (!now && receipt.kind === "saving") this.happen({ kind: "lost" });
      if (now && receipt.kind === "pending" && this.ui.conn.command(receipt.sent.command)) {
        this.happen({ kind: "relinked" });
      }
    });
  }
}

function settled(gathering: Gathering, lost: string | null): boolean {
  return lost !== null || nextSpan(gathering) === null;
}
