// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { storeValue } from "../../core/solid_store";
import { BLEND_PERCENT } from "../../core/appearance";
import { chordOf } from "../setup/chord";
import { draftOf } from "../setup/draft";
import { doorOf } from "../setup/door";
import { sliderOf } from "../setup/slider";
import { tokenOf } from "../setup/token";
import { DEFAULTS, LABELS, spell } from "../../core/keys";
import { fill, say } from "../../core/lang";
import { ui } from "../../ui";
import { receiptOf } from "../setup/card";
import Card from "../setup/card_view";
import type { Action } from "../../core/keys";
import ChordView from "../setup/chord_view";
import Door from "../setup/door.look.solid";
import Draft from "../setup/draft.look.solid";
import Slider from "../setup/slider.look.solid";
import { TokenList } from "../setup/token_view";
import { TomlDisclosure } from "../setup/toml_view";

import { GalleryCase as Case } from "./case";

export default function SetupLooks() {
  const CARD = 360;
  const percent = (at: number): string => `${String(at)}%`;
  const still = (): void => undefined;
  const HANDS = { listen: still, bind: still };
  const TOKENS = { pick: still, unpick: still };
  const TOML = ["[ui]", 'lang = "en"', "body_px = 15"].join("\n");
  const MAYOR = ["# Mayor", "", "Answer in the language the person wrote in."].join("\n");
  const lang = storeValue(ui().lang);
  const range = () =>
    fill(say(lang(), "appearance_blend_range"), {
      min: String(BLEND_PERCENT.min),
      max: String(BLEND_PERCENT.max),
      step: String(BLEND_PERCENT.step),
    });
  const chord = () => ({
    prompt: say(lang(), "keys_press"),
    unbound: say(lang(), "keys_unbound"),
    tip: say(lang(), "keys_change"),
  });
  const reset = (token: string): { readonly word: string; readonly label: string } => ({
    word: say(lang(), "colours_token_default"),
    label: fill(say(lang(), "colours_token_default_for"), { token }),
  });

  function Chord(props: { readonly action: Action; readonly listening: boolean }) {
    return (
      <ChordView
        {...chordOf(
          {
            ...chord(),
            action: props.action,
            spelled: spell(DEFAULTS[props.action]),
            listening: props.listening,
          },
          HANDS,
        )}
      />
    );
  }
  const tokens = () => [
    tokenOf({ name: "--color-accent", drawn: undefined, reset: reset("--color-accent") }, TOKENS),
    tokenOf({ name: "--color-edge", drawn: undefined, reset: undefined }, TOKENS),
  ];
  function Toml() {
    return (
      <TomlDisclosure
        region={{ "aria-label": say(lang(), "setup_toml") }}
        toggle={say(lang(), "setup_toml_toggle")}
        path=".sprawling/CONFIG.toml"
        text={TOML}
      />
    );
  }
  function slider(at: number | null) {
    return sliderOf({
      label: say(lang(), "appearance_blend"),
      range: BLEND_PERCENT,
      at,
      figure: percent,
      onMove: still,
    });
  }
  return (
    <>
      <Case label="setup card · a change just landed, with its range" width={CARD}>
        <Card
          title={say(lang(), "appearance_blend")}
          note={say(lang(), "appearance_blend_note")}
          constraint={range()}
          receipt={receiptOf(true, say(lang(), "setup_saved"))}
        >
          <Slider {...slider(60)} />
        </Card>
      </Case>

      <Case label="setup card · at rest, nothing stated" width={CARD}>
        <Card
          title={say(lang(), "appearance_blend")}
          note={say(lang(), "appearance_blend_note")}
          constraint={range()}
          receipt={receiptOf(false, say(lang(), "setup_saved"))}
        >
          <Slider {...slider(null)} />
        </Card>
      </Case>

      <Case label="setup card · a door off the advanced group" width={CARD}>
        <Card title={say(lang(), "privacy_open")} note={say(lang(), "privacy_open_note")}>
          <Door
            {...doorOf({ kind: "setup", group: "privacy" }, say(lang(), "setup_group_privacy"))}
          />
        </Card>
      </Case>

      <Case label="setup keys · one row listening, one at rest, one with no shortcut" width={CARD}>
        <ul class="flex flex-col">
          <li class="flex items-center gap-base border-b border-edge py-snug text-label">
            <span class="min-w-0 flex-1 truncate text-text-quiet">
              {say(lang(), LABELS.palette)}
            </span>
            <Chord action={"palette"} listening={true} />
          </li>
          <li class="flex items-center gap-base border-b border-edge py-snug text-label">
            <span class="min-w-0 flex-1 truncate text-text-quiet">
              {say(lang(), LABELS["tier.cycle"])}
            </span>
            <Chord action={"tier.cycle"} listening={false} />
          </li>
          <li class="flex items-center gap-base py-snug text-label">
            <span class="min-w-0 flex-1 truncate text-text-quiet">
              {say(lang(), LABELS.finder)}
            </span>
            <Chord action={"finder"} listening={false} />
          </li>
        </ul>
      </Case>

      <Case label="setup colours · a token overridden and one as drawn" width={CARD}>
        <TokenList tokens={tokens()} />
      </Case>

      <Case label="setup governed · the document box" width={CARD}>
        <Draft {...draftOf(say(lang(), "governed_mayor"), MAYOR, still)} />
      </Case>

      <Case label="setup toml · the file as the city holds it">
        <Toml />
      </Case>
    </>
  );
}
