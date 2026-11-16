// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the way into a room's conversation is given (client D95:
// `directory.svelte` is the seat, `talk_link.look.svelte` draws it). It
// is a link, because it goes to another page, drawn as a control because
// it is the one thing a room's heading offers.

export interface TalkLinkLook {
  readonly label: string;
  readonly wire: { readonly href: string };
}
