// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createUniqueId, Show } from "solid-js";
import type { Sending } from "../../core/doing";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import ControlTip from "../parts/control_tip";
import type { GlyphName } from "../parts/glyph";
import Glyph from "../parts/glyph.solid";
import { lookOf } from "./coin";
import type { Face } from "./coin_face";
import type { SpeechState } from "./speech";
import "./coin_view.css";

interface Props {
  readonly face: Face;
  readonly sending: Sending;
  readonly delivery: SpeechState["kind"];
  readonly onStop: () => void;
}

function symbol(sending: Sending, state: SpeechState["kind"]): GlyphName {
  switch (state) {
    case "preparing":
    case "pending":
      return "pulse";
    case "unknown":
      return "ring";
    case "refused":
      return "cross";
    case "idle":
      return sending === "dispatch" ? "send" : sending === "steer" ? "chevron" : "pulse";
  }
}

export default function Coin(props: Props) {
  const lang = storeValue(ui().lang);
  const id = createUniqueId();
  const look = createMemo(() => lookOf(props.face, props.sending, lang(), props.onStop));
  const state = createMemo(() =>
    props.delivery === "preparing"
      ? "pending"
      : props.delivery === "idle"
        ? undefined
        : props.delivery,
  );
  const waiting = () => props.face === "send" && (state() === "pending" || state() === "unknown");
  const hint = createMemo(() => {
    const action = look().command ?? "";
    if (props.face !== "send") return action;
    if (state() === "pending") return `${action} · ${say(lang(), "talk_delivery_pending")}`;
    if (state() === "unknown") return `${action} · ${say(lang(), "talk_delivery_unknown")}`;
    if (state() === "refused") return `${action} · ${say(lang(), "talk_failed_refused")}`;
    return action;
  });
  return (
    <Show when={props.face !== "idle"}>
      <ControlTip id={id} text={hint()} side="right">
        <button
          type={look().wire.type ?? "button"}
          class="talk-coin"
          data-control=""
          data-up={props.face === "stop" ? "stop" : "send"}
          data-kind={props.face === "send" ? props.sending : undefined}
          data-control-state={props.face === "send" ? state() : undefined}
          aria-label={hint()}
          aria-describedby={id}
          aria-disabled={waiting() ? "true" : undefined}
          onClick={(event) => {
            if (waiting()) event.preventDefault();
            else look().wire.onclick?.call(event.currentTarget, event);
          }}
        >
          <span class="talk-coin-faces" aria-hidden="true">
            <span class="talk-coin-face" data-face="send">
              <Glyph name={symbol(props.sending, props.delivery)} size="key" />
            </span>
            <span class="talk-coin-face" data-face="stop">
              <Glyph name="stop" size="stop" solid />
            </span>
          </span>
        </button>
      </ControlTip>
    </Show>
  );
}
