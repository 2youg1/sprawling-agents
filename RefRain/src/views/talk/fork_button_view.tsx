// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createUniqueId } from "solid-js";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { RunId } from "../../wire";
import ControlTip from "../parts/control_tip";
import Glyph from "../parts/glyph.solid";
import { forkLookOf, type ForkButtonLook } from "./fork_button";
import { planFork, type ForkEntry, type ForkPlan } from "./forking";

export function ForkButtonLookView(props: ForkButtonLook) {
  const hint = createUniqueId();
  return (
    <div
      data-fork
      class="shrink-0 opacity-0 transition-opacity ease-leave group-focus-within:opacity-100 group-focus-within:ease-arrive group-hover:opacity-100 group-hover:ease-arrive focus-within:opacity-100 focus-within:ease-arrive"
    >
      <ControlTip id={hint} text={props.hint}>
        <button
          type={props.wire.type}
          data-control="surface"
          aria-describedby={hint}
          onMouseEnter={() => props.wire.onmouseenter()}
          onMouseLeave={() => props.wire.onmouseleave()}
          onFocus={() => props.wire.onfocus()}
          onBlur={() => props.wire.onblur()}
          onClick={() => props.wire.onclick()}
          class="flex h-control-sm items-center gap-tight rounded-control px-tight text-note whitespace-nowrap text-text-faint hover:text-text-quiet"
        >
          <Glyph name="branch" size="sm" />
          {props.label}
        </button>
      </ControlTip>
    </div>
  );
}

export default function ForkButton(props: {
  readonly run: RunId;
  readonly entry: ForkEntry;
  readonly onFork?: ((plan: ForkPlan) => void) | undefined;
  readonly onHover: (entry: ForkEntry | null) => void;
}) {
  const lang = storeValue(ui().lang);
  const look = createMemo(() =>
    forkLookOf(
      props.entry,
      { label: say(lang(), "fork_here"), hint: say(lang(), "fork_here_hint") },
      {
        pick: (entry) => props.onFork?.(planFork(props.run, entry)),
        hover: (entry) => props.onHover(entry),
      },
    ),
  );
  return <ForkButtonLookView label={look().label} hint={look().hint} wire={look().wire} />;
}
