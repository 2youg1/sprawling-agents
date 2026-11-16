// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export type MailFocus = "key" | "inside" | "elsewhere";

export interface Mail {
  readonly shown: boolean;
  readonly focus: MailFocus;
}

export type MailInput = "toggle" | "escape" | "outside" | "enter" | "follow";

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

export interface LetterSide<Row> {
  readonly mail: Mail;
  readonly opener: Row | null;
  readonly row: Row | null;
}

export type LetterInput<Row> =
  | { readonly kind: "mail"; readonly input: MailInput }
  | { readonly kind: "open"; readonly row: Row }
  | { readonly kind: "close" };

export function stepLetter<Row>(side: LetterSide<Row>, input: LetterInput<Row>): LetterSide<Row> {
  switch (input.kind) {
    case "mail":
      return { ...side, mail: stepMail(side.mail, input.input) };
    case "open":
      return { mail: { shown: false, focus: "elsewhere" }, opener: input.row, row: null };
    case "close":
      return side.opener === null
        ? side
        : { mail: { shown: true, focus: "inside" }, opener: null, row: side.opener };
  }
}
