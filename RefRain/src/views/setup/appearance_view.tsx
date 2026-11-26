// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, onSettled } from "solid-js";

import {
  BLEND_PERCENT,
  CHROMAS,
  DENSITIES,
  GLASSES,
  MOTIONS,
  blendOf,
  type Appearance,
} from "../../core/appearance";
import { fill, say, type Key } from "../../core/lang";
import { NOTIFYINGS } from "../../core/prefs";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Segmented from "../parts/segmented.solid";
import Showing from "../shared/showing.solid";
import {
  cellsOf,
  CHROMA_WORDS,
  DENSITY_WORDS,
  drawnBlend,
  GLASS_WORDS,
  MOTION_WORDS,
  saveReceipt,
  type Setting,
} from "./appearance";
import AppearanceType from "./appearance_type_view";
import { receiptOf, type ReceiptLook } from "./card";
import Card from "./card_view";
import NotificationSetting from "./notifying.solid";
import { sliderOf } from "./slider";
import Slider from "./slider.look.solid";
import Tier from "./tier_view";
import Wallpaper from "./wallpaper";

export default function AppearanceSection() {
  const u = ui();
  const lang = storeValue(u.lang);
  const held = storeValue(u.prefs.held);
  const root = document.documentElement;

  const look = () => held().appearance;
  const said = (key: Key): string => say(lang(), key);
  const landing = saveReceipt();
  const saved = storeValue(landing.saved);
  const [blend, setBlend] = createSignal<number | null>(null);

  function write(next: Appearance, name: Setting): void {
    u.prefs.setAppearance(next);
    landing.landed(name);
  }

  onSettled(() => {
    setBlend(look().blend ?? drawnBlend(root));
  });

  function reblend(moved: string): void {
    const percent = blendOf(moved);
    if (percent === null) return;
    setBlend(percent);
    write({ ...look(), blend: percent }, "blend");
  }

  const receipt = (name: Setting): ReceiptLook => receiptOf(saved() === name, said("setup_saved"));
  const typeReceipt = createMemo(() => {
    const active = saved();
    const label = said("setup_saved");
    return (name: Setting) => receiptOf(active === name, label);
  });
  const blendRange = () =>
    fill(said("appearance_blend_range"), {
      min: String(BLEND_PERCENT.min),
      max: String(BLEND_PERCENT.max),
      step: String(BLEND_PERCENT.step),
    });

  function Choice<V extends string>(props: {
    readonly title: string;
    readonly note: string;
    readonly value: V;
    readonly values: readonly V[];
    readonly words: Readonly<Record<V, Key>>;
    readonly setting: Setting;
    readonly change: (next: V) => Appearance;
  }) {
    return (
      <Card title={props.title} note={props.note} receipt={receipt(props.setting)}>
        <Segmented
          label={props.title}
          options={cellsOf(props.values, props.words, said)}
          held={props.value}
          onPick={(next) => write(props.change(next), props.setting)}
        />
      </Card>
    );
  }

  return (
    <div class="contents">
      <Wallpaper lang={lang()} />
      <AppearanceType look={look()} write={write} receipt={typeReceipt()} />
      <Choice
        title={said("appearance_density")}
        note={said("appearance_density_note")}
        setting="density"
        value={look().density}
        values={DENSITIES}
        words={DENSITY_WORDS}
        change={(density) => ({ ...look(), density })}
      />
      <Choice
        title={said("appearance_chroma")}
        note={said("appearance_chroma_note")}
        setting="chroma"
        value={look().chroma}
        values={CHROMAS}
        words={CHROMA_WORDS}
        change={(chroma) => ({ ...look(), chroma })}
      />
      <Choice
        title={said("appearance_motion")}
        note={said("appearance_motion_note")}
        setting="motion"
        value={look().motion}
        values={MOTIONS}
        words={MOTION_WORDS}
        change={(motion) => ({ ...look(), motion })}
      />

      <Tier />

      <Choice
        title={said("appearance_glass")}
        note={said("appearance_glass_note")}
        setting="glass"
        value={look().glass}
        values={GLASSES}
        words={GLASS_WORDS}
        change={(glass) => ({ ...look(), glass })}
      />

      <Card
        title={said("appearance_blend")}
        note={said("appearance_blend_note")}
        constraint={blendRange()}
        receipt={receipt("blend")}
      >
        <Slider
          {...sliderOf({
            label: said("appearance_blend"),
            range: BLEND_PERCENT,
            at: blend(),
            figure: (percent) => `${String(percent)}%`,
            onMove: reblend,
          })}
        />
      </Card>

      <NotificationSetting
        lang={lang()}
        value={held().notifying}
        options={NOTIFYINGS}
        update={u.prefs.setNotifying}
      />

      <Card title={said("showing_label")} note={said("showing_note")}>
        <Showing />
      </Card>
    </div>
  );
}
