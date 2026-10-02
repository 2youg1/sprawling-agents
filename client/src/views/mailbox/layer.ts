// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The properties this module must hold are proved in `client/spec/Views/Workspace.lean`
// (`stepMail`, client D60).

export type MailFocus = "key" | "inside" | "elsewhere";

export interface Mail {
  readonly shown: boolean;
  readonly focus: MailFocus;
}

export type MailInput = "toggle" | "escape" | "outside" | "enter" | "follow";

export function stepMail(mail: Mail, _input: MailInput): Mail {
  return mail;
}
