// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The dot at the top of the rail, in each of the three ways it is
// drawn.
//
// The dot has two states and one of them moves: grey is nothing to do,
// yellow is at least one of three things to do, and yellow pulses while
// the link is still on its way up - the one reason that resolves
// without the person doing anything. Three fixtures, because what a
// person learns from this control is entirely which of the three they
// are looking at, and a screen that can only draw one of them says
// nothing about the other two.
//
// It is the only control on this route that reads the city instead of
// its props, so each fixture hands it one through `Stand`. Every dot
// here is the shipped component: what differs between the three cases
// is the city behind it and nothing else.

import type { AxError } from "../../wire";
import { Presence } from "../notices";
import { Case } from "./case";
import { ONE_QUESTION } from "./conversation";
import { Stand } from "./stand";

// One refusal nobody has opened. The words are the city's own - an
// action, a subject and the way back - which is what a refusal carries
// on the wire and what the panel under the dot reads back.
const UNREAD: AxError = {
  action: "attach an endpoint",
  code: "E_CREDENTIAL_MISSING",
  gate: null,
  nearby: [],
  recovery: "file a key for this provider, then attach it again",
  retriable: false,
  subject: "zenmux",
};

export function Presences() {
  return (
    <>
      <Case label="presence · grey, nothing to do">
        <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} effort={null}>
          <Presence />
        </Stand>
      </Case>

      <Case label="presence · yellow, a question waiting and a refusal unread">
        <Stand
          link={{ kind: "live", city: "sprawling" }}
          unread={[UNREAD]}
          waiting={[ONE_QUESTION]}
          effort={null}
        >
          <Presence />
        </Stand>
      </Case>

      {/* Still on its way up, which is the state that resolves by
          itself: the dot pulses rather than asking for anything. */}
      <Case label="presence · yellow and pulsing, the link is still connecting">
        <Stand link={{ kind: "handshaking" }} unread={[]} waiting={[]} effort={null}>
          <Presence />
        </Stand>
      </Case>
    </>
  );
}
