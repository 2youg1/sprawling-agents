// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo } from "solid-js";
import { accel, marks } from "../../core/keys";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { Seq } from "../../wire";
import Glyph from "../parts/glyph.solid";
import { addressLineOf } from "../talk/address_line";
import { AddressLineLookView as AddressLine } from "../talk/address_line_view";
import { foldWire } from "../talk/fold";
import type { ForkWire } from "../talk/fork_button";
import { ForkButtonLookView as ForkButton } from "../talk/fork_button_view";
import { GaugeCheckpoint as Checkpoint } from "../talk/gauge_view";
import { InboxLookView as Inbox } from "../talk/inbox_view";
import { AsideLink, Fold } from "../talk/rule_view";
import { FACT } from "../talk/sandbox_fact";
import { TypedLineLook as TypedLine } from "../talk/typed_line_view";
import { mcpDoorOf } from "../welcome/door";
import Door from "../welcome/door.look.solid";
import Step from "../welcome/step.look.solid";
import { GalleryCase } from "./case";

const still = (): void => undefined;
const WORDS = { files: "3 files", asks: "waiting on you", done: "done" };
const LONG = "atlas-payments-reconciliation-service/ledger-close-and-settlement";
const fork: ForkWire = {
  type: "button",
  onmouseenter: still,
  onmouseleave: still,
  onfocus: still,
  onblur: still,
  onclick: still,
};

export default function LooksTalk() {
  const lang = storeValue(ui().lang);
  const lesson = createMemo(() => ({
    words: say(lang(), "talk_address_hint"),
    chord: marks(accel("/"), "other").join(" "),
  }));
  return (
    <>
      <GalleryCase label="thread facts · a plain fact, one that asks, a fold, a fork key and an aside link">
        <div class="flex flex-wrap items-center gap-base">
          <span class={FACT}>
            <Glyph name="check" size="sm" />
            <span class="sr-only">{WORDS.done}</span>
            {WORDS.files}
          </span>
          <span class={FACT}>{WORDS.asks}</span>
          <Fold label="2 more" wire={foldWire(false, still)} />
          <ForkButton label="branch" hint="branch from this turn" wire={fork} />
          <AsideLink text="open the run" href="#/run/7f3a" />
        </div>
      </GalleryCase>

      <GalleryCase label="address line · a session, the lesson before the palette was ever opened, a long address cut in its building, a room with no session yet">
        <div class="flex flex-col gap-snug">
          <AddressLine
            look={addressLineOf(lang(), "hall/mayor", Seq.make(23), still)}
            seat={{ at: "head" }}
          />
          <AddressLine
            look={addressLineOf(lang(), "hall/mayor", null, still)}
            seat={{ at: "head" }}
            lesson={lesson()}
          />
          <AddressLine
            look={addressLineOf(lang(), LONG, Seq.make(4182), still)}
            seat={{ at: "head" }}
          />
          <AddressLine
            look={addressLineOf(lang(), "atlas/api", null, still)}
            seat={{ at: "head" }}
          />
        </div>
      </GalleryCase>

      <GalleryCase
        label="address line · at a phone's width, the room kept whole, a room too long for the column cut last, and the lesson giving way first"
        width={320}
      >
        <div class="flex flex-col gap-snug">
          <AddressLine
            look={addressLineOf(lang(), LONG, Seq.make(4182), still)}
            seat={{ at: "head" }}
          />
          <AddressLine
            look={addressLineOf(
              lang(),
              "atlas/payments-reconciliation-ledger-close-and-settlement-overnight",
              Seq.make(4182),
              still,
            )}
            seat={{ at: "head" }}
          />
          <AddressLine
            look={addressLineOf(lang(), LONG, Seq.make(4182), still)}
            seat={{ at: "head" }}
            lesson={lesson()}
          />
        </div>
      </GalleryCase>

      <GalleryCase label="thread inbox · two letters waiting in this room">
        <Inbox
          label="inbox"
          heading="2 waiting"
          rows={[
            {
              key: "1",
              href: "#/talk/shop/checkout",
              kind: "letter",
              first: "The total is fixed.",
              from: "checkout",
              ago: "2m",
            },
            {
              key: "2",
              href: undefined,
              kind: "handback",
              first: "Tests pass on main.",
              from: "ci",
              ago: "5m",
            },
          ]}
        />
      </GalleryCase>

      <GalleryCase label="composer marks · the typed line lit and resting, two checkpoints on a line">
        <div class="flex flex-col gap-base">
          <div class="relative h-control">
            <TypedLine typed={120} lit={true} />
          </div>
          <div class="relative h-control">
            <TypedLine typed={0} lit={false} />
          </div>
          <div class="relative h-control">
            <Checkpoint reminder="first" at={60} along="line" />
            <Checkpoint reminder="second" at={85} along="line" />
          </div>
        </div>
      </GalleryCase>

      <GalleryCase label="welcome steps · one configured, one required with its body open, and the MCP door">
        <div class="flex flex-col">
          <Step
            number="1"
            title={say(lang(), "guide_step_provider")}
            standing="configured"
            word={say(lang(), "guide_standing_configured")}
            wire={{
              id: "fixture-step-1",
              type: "button",
              "aria-expanded": false,
              "aria-controls": "fixture-step-1-body",
              onclick: still,
            }}
          />
          <Step
            number="5"
            title={say(lang(), "guide_step_mcp")}
            standing="required"
            word={say(lang(), "guide_standing_required")}
            wire={{
              id: "fixture-step-5",
              type: "button",
              "aria-expanded": true,
              "aria-controls": "fixture-step-5-body",
              onclick: still,
            }}
          />
          <div id="fixture-step-5-body">
            <Door {...mcpDoorOf(lang())} />
          </div>
        </div>
      </GalleryCase>
    </>
  );
}
