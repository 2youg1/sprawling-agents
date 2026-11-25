// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Effect, Result, Schema } from "effect";

interface LocalHandle extends FileSystemFileHandle {
  readonly queryPermission?: (options: { mode: "read" }) => Promise<PermissionState>;
  readonly requestPermission?: (options: { mode: "read" }) => Promise<PermissionState>;
}

declare global {
  interface Window {
    showOpenFilePicker?: (options: {
      readonly types: readonly { accept: { "image/*": readonly string[] } }[];
      readonly excludeAcceptAllOption: boolean;
    }) => Promise<readonly LocalHandle[]>;
  }
}

interface Look {
  readonly choice: Choice;
  readonly image: string | null;
  readonly unavailable: boolean;
}

type Problem = "read" | "write" | "invalid";

export interface WallpaperState {
  readonly look: Look;
  readonly dirty: boolean;
  readonly busy: boolean;
  readonly problem: Problem | null;
  readonly saved: boolean;
}

interface WallpaperSettings {
  readonly subscribe: (show: (state: WallpaperState) => void) => () => void;
  readonly select: (kind: "solid" | "marble") => void;
  readonly brightness: (value: number) => void;
  readonly fileWallpaper: (enabled: boolean) => void;
  readonly filePalette: (value: string) => void;
  readonly choose: (input: HTMLInputElement) => Promise<void>;
  readonly importFile: (file: File) => Promise<void>;
  readonly restoreSource: () => Promise<void>;
  readonly apply: () => Promise<void>;
  readonly revert: () => void;
}

export const BRIGHTNESS = { min: 0, max: 100, step: 1 } as const;
export const WALLPAPER_ACCEPT = ".png,.jpg,.jpeg,.webp,.avif";
export const WALLPAPER_MIB = 16;
export const WALLPAPER_EDGE = 4096;
const DATABASE = "refrain-appearance";
const STORE = "device";
const KEY = "appearance";
const MARBLE = new URL("../theme/marble.webp", import.meta.url).href;

const Brightness = Schema.Int.check(
  Schema.isBetween({ minimum: BRIGHTNESS.min, maximum: BRIGHTNESS.max }),
);
const validBrightness = Schema.is(Brightness);
export const FILE_PALETTES = ["inherit", "paper", "mist", "slate", "moss"] as const;
const FilePalette = Schema.Literals(FILE_PALETTES);
const validPalette = Schema.is(FilePalette);
const CurrentChoice = Schema.Struct({
  version: Schema.Literal(1),
  brightness: Brightness,
  fileWallpaper: Schema.optional(Schema.Boolean),
  filePalette: Schema.optional(FilePalette),
  wallpaper: Schema.Union([
    Schema.Struct({ kind: Schema.Literal("solid") }),
    Schema.Struct({ kind: Schema.Literal("marble") }),
    Schema.Struct({
      kind: Schema.Literal("copy"),
      name: Schema.String,
      blob: Schema.instanceOf(Blob),
    }),
    Schema.Struct({
      kind: Schema.Literal("file"),
      name: Schema.String,
      handle: Schema.declare(
        (handle): handle is LocalHandle =>
          typeof FileSystemFileHandle !== "undefined" && handle instanceof FileSystemFileHandle,
      ),
    }),
  ]),
});
type Choice = typeof CurrentChoice.Type;
const decodeChoice = Schema.decodeUnknownResult(CurrentChoice);
const DEFAULT: Choice = { version: 1, wallpaper: { kind: "solid" }, brightness: 50 };
const FIRST: Look = { choice: DEFAULT, image: null, unavailable: false };

function attempt<T>(work: () => Promise<T>): Promise<Result.Result<T, unknown>> {
  return Effect.runPromise(
    Effect.result(Effect.tryPromise({ try: work, catch: (error) => error })),
  );
}

function openStore(): Promise<IDBDatabase> {
  return new Promise((resolve, reject: (reason: unknown) => void) => {
    const request = indexedDB.open(DATABASE, 1);
    request.onupgradeneeded = () => {
      request.result.createObjectStore(STORE);
    };
    request.onsuccess = () => {
      resolve(request.result);
    };
    request.onerror = () => {
      reject(request.error ?? new Error("wallpaper_database"));
    };
  });
}

function transact<T>(
  db: IDBDatabase,
  mode: IDBTransactionMode,
  run: (store: IDBObjectStore) => IDBRequest<T>,
): Promise<T> {
  return new Promise((resolve, reject: (reason: unknown) => void) => {
    const transaction = db.transaction(STORE, mode);
    const request = run(transaction.objectStore(STORE));
    transaction.oncomplete = () => {
      resolve(request.result);
    };
    transaction.onabort = () => {
      reject(transaction.error ?? new Error("wallpaper_transaction"));
    };
  });
}

async function checkedImage(file: Blob): Promise<Blob | null> {
  if (!/^image\/(avif|jpeg|png|webp)$/.test(file.type) || file.size > WALLPAPER_MIB * 1024 ** 2) {
    return null;
  }
  const bitmap = await createImageBitmap(file);
  const canvas = document.createElement("canvas");
  const painted = Result.try(() => {
    if (Math.max(bitmap.width, bitmap.height) > WALLPAPER_EDGE) return false;
    canvas.width = bitmap.width;
    canvas.height = bitmap.height;
    const pen = canvas.getContext("2d");
    if (pen === null) return false;
    pen.drawImage(bitmap, 0, 0);
    return true;
  });
  bitmap.close();
  if (Result.isFailure(painted) || !painted.success) return null;
  return new Promise((resolve) => {
    canvas.toBlob(resolve, "image/webp");
  });
}

async function availableFile(handle: LocalHandle): Promise<File | null> {
  const permission = await handle.queryPermission?.({ mode: "read" });
  return permission === undefined || permission === "granted" ? handle.getFile() : null;
}

async function readableFile(handle: LocalHandle): Promise<Blob | null> {
  const file = await availableFile(handle);
  return file === null ? null : checkedImage(file);
}

async function openLook(choice: Choice): Promise<Look> {
  const { wallpaper } = choice;
  if (wallpaper.kind === "solid" || wallpaper.kind === "marble") {
    return simple(choice, wallpaper.kind);
  }
  const selected = await attempt(() =>
    wallpaper.kind === "copy" ? Promise.resolve(wallpaper.blob) : readableFile(wallpaper.handle),
  );
  const blob = Result.getOrNull(selected);
  return {
    choice,
    image: blob === null ? null : URL.createObjectURL(blob),
    unavailable: blob === null,
  };
}

function simple(choice: Choice, kind: "solid" | "marble"): Look {
  return {
    choice: { ...choice, wallpaper: { kind } },
    image: kind === "marble" ? MARBLE : null,
    unavailable: false,
  };
}

function sameAppearance(left: Look, right: Look): boolean {
  const one = left.choice.wallpaper;
  const other = right.choice.wallpaper;
  return (
    left.choice.brightness === right.choice.brightness &&
    (left.choice.fileWallpaper ?? false) === (right.choice.fileWallpaper ?? false) &&
    (left.choice.filePalette ?? "inherit") === (right.choice.filePalette ?? "inherit") &&
    (one === other || (one.kind === other.kind && (one.kind === "solid" || one.kind === "marble")))
  );
}

function dropImage(url: string | null): void {
  if (url?.startsWith("blob:") === true) URL.revokeObjectURL(url);
}

function draw(root: HTMLElement, earlier: Look | undefined, look: Look): void {
  if (earlier?.choice.brightness !== look.choice.brightness) {
    root.style.setProperty("--reading-level", String(look.choice.brightness / BRIGHTNESS.max));
  }
  root.toggleAttribute("data-file-wallpaper", look.choice.fileWallpaper === true);
  root.dataset.filePalette = look.choice.filePalette ?? "inherit";
  if (earlier?.image === look.image) return;
  if (look.image === null) root.style.removeProperty("--refrain-wallpaper-image");
  else root.style.setProperty("--refrain-wallpaper-image", `url(${JSON.stringify(look.image)})`);
}

function openSettings(root: HTMLElement): WallpaperSettings {
  const listeners = new Set<(state: WallpaperState) => void>();
  let db: IDBDatabase | null = null;
  let kept = FIRST;
  let drawn: Look | undefined;
  let look = FIRST;
  let busy = true;
  let saved = false;
  let problem: Problem | null = null;
  let abandoned = false;

  const state = (): WallpaperState => ({
    look,
    busy,
    saved,
    problem,
    dirty: !sameAppearance(look, kept),
  });

  const show = (): void => {
    draw(root, drawn, look);
    drawn = look;
    const current = state();
    for (const listener of listeners) listener(current);
  };

  const preview = (next: Look): void => {
    const image = look.image;
    look = next;
    saved = false;
    problem = null;
    show();
    if (image !== kept.image && image !== next.image) dropImage(image);
  };

  const previewEnded = (): boolean => abandoned;
  const isBusy = (): boolean => busy;

  const runChoice = async (choose: () => Promise<Look | "invalid" | null>): Promise<void> => {
    if (busy) return;
    busy = true;
    abandoned = false;
    show();
    const picked = await attempt(choose);
    const next = Result.getOrNull(picked);
    if (next !== null && next !== "invalid") {
      if (previewEnded()) dropImage(next.image);
      else preview(next);
    } else if (
      next === "invalid" ||
      (Result.isFailure(picked) &&
        !(picked.failure instanceof DOMException && picked.failure.name === "AbortError"))
    ) {
      problem = "invalid";
    }
    busy = false;
    if (previewEnded()) preview(kept);
    else show();
  };

  const read = async (): Promise<void> => {
    const stored = await attempt(async (): Promise<unknown> => {
      db = await openStore();
      return transact(db, "readonly", (store) => store.get(KEY));
    });
    if (Result.isFailure(stored)) problem = "read";
    else if (stored.success !== undefined) {
      const decoded = decodeChoice(stored.success);
      if (Result.isSuccess(decoded)) kept = await openLook(decoded.success);
      else problem = "read";
    }
    look = kept;
    busy = false;
    show();
  };
  async function onReturn(): Promise<void> {
    const before = kept;
    const wallpaper = before.choice.wallpaper;
    if (busy || wallpaper.kind !== "file") return;
    const available = Result.getOrNull(await attempt(() => availableFile(wallpaper.handle)));
    if ((available === null) === before.unavailable) return;
    const blob =
      available === null ? null : Result.getOrNull(await attempt(() => checkedImage(available)));
    const image = blob === null ? null : URL.createObjectURL(blob);
    if (kept !== before || isBusy()) {
      dropImage(image);
      return;
    }
    kept = { ...before, image, unavailable: image === null };
    if (look.choice.wallpaper === wallpaper) {
      look = { ...look, image, unavailable: image === null };
      show();
    }
    dropImage(before.image);
  }

  root.ownerDocument.defaultView?.addEventListener("focus", () => {
    void onReturn();
  });
  void read();

  return {
    subscribe(showState) {
      listeners.add(showState);
      showState(state());
      return () => {
        listeners.delete(showState);
      };
    },
    select(kind) {
      if (!busy) preview(simple(look.choice, kind));
    },
    brightness(value) {
      if (!busy && validBrightness(value))
        preview({ ...look, choice: { ...look.choice, brightness: value } });
    },
    fileWallpaper(enabled) {
      if (!busy) preview({ ...look, choice: { ...look.choice, fileWallpaper: enabled } });
    },
    filePalette(value) {
      if (!busy && validPalette(value))
        preview({ ...look, choice: { ...look.choice, filePalette: value } });
    },
    async choose(input) {
      const open = window.showOpenFilePicker?.bind(window);
      if (open === undefined) {
        input.click();
        return;
      }
      await runChoice(async () => {
        const [handle] = await open({
          types: [{ accept: { "image/*": WALLPAPER_ACCEPT.split(",") } }],
          excludeAcceptAllOption: true,
        });
        if (handle === undefined) return null;
        const blob = await checkedImage(await handle.getFile());
        if (blob === null) return "invalid";
        return {
          choice: { ...look.choice, wallpaper: { kind: "file", name: handle.name, handle } },
          image: URL.createObjectURL(blob),
          unavailable: false,
        };
      });
    },
    async importFile(file) {
      await runChoice(async () => {
        const blob = await checkedImage(file);
        if (blob === null) return "invalid";
        return {
          choice: { ...look.choice, wallpaper: { kind: "copy", name: file.name, blob } },
          image: URL.createObjectURL(blob),
          unavailable: false,
        };
      });
    },
    async restoreSource() {
      const { wallpaper } = look.choice;
      if (wallpaper.kind !== "file") return;
      await runChoice(async () => {
        await wallpaper.handle.requestPermission?.({ mode: "read" });
        const blob = await readableFile(wallpaper.handle);
        if (blob === null) return "invalid";
        const image = URL.createObjectURL(blob);
        if (!previewEnded() && kept.choice.wallpaper === wallpaper) {
          dropImage(kept.image);
          kept = { ...kept, image, unavailable: false };
        }
        return { ...look, image, unavailable: false };
      });
    },
    async apply() {
      if (busy || sameAppearance(look, kept)) return;
      busy = true;
      abandoned = false;
      problem = null;
      show();
      const result = await attempt(async () => {
        db ??= await openStore();
        return transact(db, "readwrite", (store) => store.put(look.choice, KEY));
      });
      if (Result.isSuccess(result)) {
        const previous = kept.image;
        kept = look;
        saved = !previewEnded();
        if (previous !== kept.image) dropImage(previous);
      } else if (previewEnded()) preview(kept);
      else problem = "write";
      busy = false;
      show();
    },
    revert() {
      abandoned = true;
      if (!busy) preview(kept);
    },
  };
}

let settings: WallpaperSettings | undefined;

export function wallpaperSettings(): WallpaperSettings {
  settings ??= openSettings(document.documentElement);
  return settings;
}
