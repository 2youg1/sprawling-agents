// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import WelcomeBody from "./welcome/body.solid";
import GuidePage, { type GuidePageProps } from "./welcome/guide_page";

export type WelcomeProps = Omit<GuidePageProps, "body">;

export default function Welcome(props: WelcomeProps) {
  return <GuidePage {...props} body={(step) => <WelcomeBody step={step} />} />;
}
