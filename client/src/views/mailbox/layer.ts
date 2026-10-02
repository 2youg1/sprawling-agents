// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The properties this module must hold are proved in `client/spec/Views/Workspace.lean`
// (`stepMail`, client D60).
//
// The mailbox is an ordinary layer under the three edge keys rather than
// a popover, so the platform no longer decides what closes it and where
// the focus goes: this step does. `mailbox.svelte` feeds it the key and
// Accel-B (`toggle`), Escape, a press outside the column (the other two
// edge keys are outside), the focus entering the column, and a followed
// row; it reads back whether the column is shown and whether the focus
// belongs on the key.

export type MailFocus = "key" | "inside" | "elsewhere";

export interface Mail {
  readonly shown: boolean;
  readonly focus: MailFocus;
}

export type MailInput = "toggle" | "escape" | "outside" | "enter" | "follow";

// Putting the column away: a focus inside it goes back to the key, and a
// focus anywhere else stays where it is.
function stow(mail: Mail): Mail {
  return { shown: false, focus: mail.focus === "inside" ? "key" : mail.focus };
}

export function stepMail(mail: Mail, input: MailInput): Mail {
  switch (input) {
    case "toggle":
      return mail.shown ? stow(mail) : { ...mail, shown: true };
    case "escape":
    case "follow":
      return stow(mail);
    case "outside":
      return { shown: false, focus: "elsewhere" };
    case "enter":
      return mail.shown ? { ...mail, focus: "inside" } : mail;
  }
}
