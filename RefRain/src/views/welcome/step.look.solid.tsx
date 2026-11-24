// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import Glyph from "../parts/glyph.solid";
import type { Standing } from "./guide";
import type { StepLook } from "./step";

export type PortProps = StepLook;

export default function WelcomeStepLook(look: StepLook & { readonly routeFocus?: boolean }) {
  const INK: Readonly<Record<Standing, string>> = {
    configured: "text-text",
    required: "text-alert",
    skipped: "text-text-faint",
    seen: "text-text-quiet",
    untouched: "text-text-faint",
  };
  return (
    <>
      <h2>
        <button
          id={look.wire.id}
          type={look.wire.type}
          aria-controls={look.wire["aria-controls"]}
          aria-expanded={look.wire["aria-expanded"] ? "true" : "false"}
          data-route-focus={look.routeFocus ? "" : undefined}
          data-control="surface"
          onClick={() => look.wire.onclick()}
          class={
            "grid w-full grid-cols-[4ch_minmax(0,1fr)_auto] items-baseline gap-x-base py-base text-left transition-colors ease-leave hover:wash hover:ease-arrive focus-visible:wash"
          }
        >
          <span class={"figure text-heading text-text-faint"}>{look.number}</span>
          <span class={"min-w-0 text-label font-label text-text"}>{look.title}</span>
          <span class={["flex items-center gap-tight text-note", INK[look.standing]]}>
            {look.standing === "configured" ? (
              <>
                <Glyph name={"check"} class={"size-glyph-sm"} />
              </>
            ) : undefined}
            {look.word !== undefined ? <>{look.word}</> : undefined}
          </span>
        </button>
      </h2>
    </>
  );
}
