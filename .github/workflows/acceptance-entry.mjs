// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import words from "../../RefRain/src/lang.json";

export async function enterWorkspace(page, opening, { observeGuide } = {}) {
  const guide = page.getByRole("button", { name: words.guide_put_off_rest.en, exact: true });
  const recovery = page.getByRole("heading", { name: words.pair_title.en, exact: true });
  const composer = page.locator("main textarea").first();
  await page.goto(opening);
  const landing = await page.evaluate(() => performance.now());
  await guide.or(recovery).or(composer).first().waitFor({ timeout: 60_000 });
  if (await recovery.isVisible()) throw new Error("The browser opened pairing recovery.");
  const firstView = (await guide.isVisible()) ? "welcome" : "session";
  if (firstView === "session" && observeGuide !== undefined) {
    await page.goto(new URL("/#/welcome", opening).href);
    await guide.waitFor({ timeout: 60_000 });
  }
  if (await guide.isVisible()) {
    await observeGuide?.();
    await guide.click();
  }
  await composer.waitFor({ timeout: 60_000 });
  return { firstView, landing };
}

export async function capture(page, path) {
  if (process.env.REFRAIN_SCREENSHOTS !== "0") await page.screenshot({ path });
}
