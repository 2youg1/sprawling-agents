// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Show, createSignal, createUniqueId, onSettled } from "solid-js";

import { fill, say, type Key, type Lang } from "../../core/lang";
import { RenderSnippet, type Snippet } from "../../core/snippet";
import {
  BRIGHTNESS,
  WALLPAPER_ACCEPT,
  WALLPAPER_EDGE,
  WALLPAPER_MIB,
  wallpaperSettings,
  type WallpaperState,
} from "../../core/wallpaper";
import { sliderOf } from "./slider";
import Slider from "./slider.look.solid";

interface Props {
  readonly lang: Lang;
}

interface CardProps extends Props {
  readonly controls: Snippet;
  readonly saved: boolean;
}

function Card(props: CardProps) {
  return (
    <div class="flex min-w-0 flex-col gap-snug rounded-card bg-raised px-base py-snug">
      <div class="flex flex-col gap-hair">
        <span class="text-label font-label text-text">{say(props.lang, "wallpaper_title")}</span>
        <p class="text-note text-text-faint">{say(props.lang, "wallpaper_note")}</p>
      </div>
      <RenderSnippet snippet={props.controls} args={[]} />
      <span role="status" aria-atomic="true" class="ml-auto text-note text-text-quiet">
        {props.saved ? say(props.lang, "setup_saved") : ""}
      </span>
    </div>
  );
}

const BUTTON =
  "inline-flex items-center gap-snug rounded-control px-base text-label " +
  "aria-disabled:bg-raised aria-disabled:text-text-disabled " +
  "transition-[background-color,color,opacity,transform] ease-leave " +
  "hover:ease-arrive active:ease-arrive active:scale-(--scale-press) " +
  "still:transition-none still:active:scale-100";
const SECONDARY = "h-control bg-raised text-text data-[state=idle]:hover:bg-raised-hover";
const PRIMARY = "h-control-lg bg-accent text-on-accent data-[state=idle]:hover:bg-accent-hover";

interface ButtonProps {
  readonly label: string;
  readonly stopped?: string | undefined;
  readonly busy?: boolean;
  readonly primary?: boolean;
  readonly onPress: () => void;
}

function Button(props: ButtonProps) {
  const reason = createUniqueId();
  const disabled = () => props.stopped !== undefined || props.busy === true;
  return (
    <>
      <button
        data-control=""
        type="button"
        class={`${BUTTON} ${props.primary === true ? PRIMARY : SECONDARY}`}
        data-state={
          props.busy === true ? "loading" : props.stopped === undefined ? "idle" : "stopped"
        }
        aria-busy={props.busy === true ? "true" : "false"}
        aria-disabled={disabled() ? "true" : "false"}
        aria-describedby={props.stopped === undefined ? undefined : reason}
        title={props.stopped}
        onClick={() => {
          if (!disabled()) props.onPress();
        }}
      >
        <Show when={props.busy === true}>
          <span class="inline-block size-dot pulse rounded-pill bg-current" aria-hidden="true" />
        </Show>
        {props.label}
      </button>
      <Show when={props.stopped !== undefined}>
        <span id={reason} class="sr-only">
          {props.stopped}
        </span>
      </Show>
    </>
  );
}

function problemOf(lang: Lang, problem: WallpaperState["problem"] | undefined): string | undefined {
  switch (problem) {
    case "read":
      return say(lang, "wallpaper_read_failed");
    case "write":
      return say(lang, "wallpaper_write_failed");
    case "invalid":
      return fill(say(lang, "wallpaper_invalid"), {
        limit: String(WALLPAPER_MIB),
        edge: String(WALLPAPER_EDGE),
      });
    case null:
    case undefined:
      return undefined;
  }
}

export default function Wallpaper(props: Props) {
  const settings = wallpaperSettings();
  const [held, setHeld] = createSignal<WallpaperState>();
  let files: HTMLInputElement | undefined;
  const said = (key: Key): string => say(props.lang, key);
  const selected = () => held()?.look.choice.wallpaper.kind;
  const local = () => selected() === "file" || selected() === "copy";
  const busy = () => held()?.busy ?? true;
  const disabled = () => (busy() ? said("wallpaper_wait") : undefined);
  const unchanged = () => (held()?.dirty === true ? undefined : said("wallpaper_no_changes"));

  onSettled(() => {
    const unsubscribe = settings.subscribe(setHeld);
    return () => {
      unsubscribe();
      settings.revert();
    };
  });

  function pick(): void {
    if (files !== undefined) void settings.choose(files);
  }

  function sourceNote(): string | undefined {
    const source = held()?.look.choice.wallpaper;
    if (source?.kind !== "file" && source?.kind !== "copy") return undefined;
    return fill(said(source.kind === "copy" ? "wallpaper_copy" : "wallpaper_source"), {
      name: source.name,
    });
  }

  const controls: Snippet = () => (
    <div class="grid gap-snug items-start">
      <fieldset class="flex flex-wrap items-center gap-tight" disabled={busy()}>
        <legend class="sr-only">{said("wallpaper_title")}</legend>
        <label class="inline-flex h-control items-center gap-tight">
          <input
            type="radio"
            name="refrain-wallpaper"
            checked={selected() === "solid"}
            onChange={() => settings.select("solid")}
          />
          {said("wallpaper_solid")}
        </label>
        <label class="inline-flex h-control items-center gap-tight">
          <input
            type="radio"
            name="refrain-wallpaper"
            checked={selected() === "marble"}
            onChange={() => settings.select("marble")}
          />
          {said("wallpaper_marble")}
        </label>
        <Button
          label={said(local() ? "wallpaper_choose_another" : "wallpaper_local")}
          stopped={disabled()}
          onPress={pick}
        />
        <input
          type="file"
          accept={WALLPAPER_ACCEPT}
          hidden
          ref={(element) => {
            files = element;
          }}
          onChange={(event) => {
            const input = event.currentTarget;
            const file = input.files?.item(0);
            input.value = "";
            if (file !== null && file !== undefined) void settings.importFile(file);
          }}
        />
      </fieldset>
      <Show when={sourceNote()}>{(note) => <p class="text-note text-text-quiet">{note()}</p>}</Show>
      <Show when={held()?.look.unavailable === true}>
        <p role="status" class="text-note text-text">
          {said("wallpaper_unavailable")}
        </p>
        <Button
          label={said("wallpaper_restore_access")}
          stopped={disabled()}
          onPress={() => {
            void settings.restoreSource();
          }}
        />
      </Show>
      <div class="grid grid-cols-2 gap-base">
        <div class="rounded-card" data-refrain-wallpaper-preview="" aria-hidden="true" />
        <div class="rounded-card flex items-center justify-center" data-refrain-reading-canvas="">
          <span class="text-body text-text">{said("wallpaper_canvas")}</span>
        </div>
      </div>
      <Show when={held()}>
        {(current) => (
          <fieldset disabled={current().busy}>
            <Slider
              {...sliderOf({
                label: said("wallpaper_brightness"),
                range: BRIGHTNESS,
                at: current().look.choice.brightness,
                figure: (value) => `${String(value)}%`,
                onMove: (raw) => {
                  settings.brightness(Number(raw));
                },
              })}
            />
          </fieldset>
        )}
      </Show>
      <Show when={problemOf(props.lang, held()?.problem)}>
        {(problem) => (
          <p role="alert" class="text-note text-text">
            {problem()}
          </p>
        )}
      </Show>
      <div class="flex flex-wrap gap-tight">
        <Button
          primary
          label={said("wallpaper_apply")}
          busy={busy()}
          stopped={busy() ? undefined : unchanged()}
          onPress={() => {
            void settings.apply();
          }}
        />
        <Button
          label={said("wallpaper_revert")}
          stopped={disabled() ?? unchanged()}
          onPress={settings.revert}
        />
      </div>
    </div>
  );
  return <Card lang={props.lang} controls={controls} saved={held()?.saved === true} />;
}
