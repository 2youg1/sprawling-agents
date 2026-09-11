// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The toolkits this page offers to connect, and how each of them signs
// a person in.
//
// **This is a stand-in, and the shape of the answer the city owes.**
// Composio publishes the real directory and it changes weekly; a
// browser cannot ask for it, because the request needs the api key and
// a key must not leave the vault to a page. So the list is written here
// until a frame exists that has the city fetch it, and the fields below
// are the fields that frame must carry.

export interface Toolkit {
  // What Composio calls it in an id: lowercase, no spaces.
  readonly slug: string;
  // What its own vendor calls it, which is the same word in both
  // languages and therefore not in the phrase table.
  readonly name: string;
  readonly auth: "oauth" | "api_key";
}

export const TOOLKITS: readonly Toolkit[] = [
  { slug: "github", name: "GitHub", auth: "oauth" },
  { slug: "gmail", name: "Gmail", auth: "oauth" },
  { slug: "googlecalendar", name: "Google Calendar", auth: "oauth" },
  { slug: "googledrive", name: "Google Drive", auth: "oauth" },
  { slug: "slack", name: "Slack", auth: "oauth" },
  { slug: "notion", name: "Notion", auth: "oauth" },
  { slug: "linear", name: "Linear", auth: "oauth" },
  { slug: "jira", name: "Jira", auth: "oauth" },
  { slug: "hubspot", name: "HubSpot", auth: "oauth" },
  { slug: "sendgrid", name: "SendGrid", auth: "api_key" },
  { slug: "stripe", name: "Stripe", auth: "api_key" },
  { slug: "perplexityai", name: "Perplexity", auth: "api_key" },
];
