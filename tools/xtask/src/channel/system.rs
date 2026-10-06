// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Homebrew and AUR projections of the release archives (xtask D32).

use std::path::Path;

use crate::platform::Platform;
use crate::report::XtaskError;

pub(super) struct Archive<'a> {
    pub(super) platform: &'a Platform,
    pub(super) name: String,
    pub(super) sha256: String,
}

pub(super) fn write(
    stem: &super::Stem<'_>,
    tag: &str,
    archives: &[Archive<'_>],
    out: &Path,
) -> Result<(), XtaskError> {
    let mut branches = String::new();
    for archive in archives {
        let row = archive.platform;
        let url = format!(
            "{}/releases/download/{tag}/{}",
            stem.repository, archive.name
        );
        let sha256 = &archive.sha256;
        if row.os == "darwin" && row.cpu == "arm64" {
            branches.push_str(&format!(
                "  on_macos do\n    on_arm do\n      url \"{url}\"\n      sha256 \"{sha256}\"\n    end\n  end\n"
            ));
        } else if row.os == "linux" && row.cpu == "x64" {
            branches.push_str(&format!(
                "  on_linux do\n    on_intel do\n      url \"{url}\"\n      sha256 \"{sha256}\"\n    end\n  end\n"
            ));
            aur(stem, &url, archive, out)?;
        }
    }
    if !branches.is_empty() {
        let version = stem.version;
        let repository = stem.repository;
        super::write(
            &out.join("homebrew/sprawling.rb"),
            format!(
                r#"class Sprawling < Formula
  desc "Local agent harness"
  homepage "{repository}"
  version "{version}"
  license "MPL-2.0"
{branches}
  def install
    bin.install "sprawling"
    libexec.install Dir["*"]
  end

  test do
    system bin/"sprawling", "--version"
  end
end
"#
            )
            .as_bytes(),
        )?;
    }
    Ok(())
}

fn aur(
    stem: &super::Stem<'_>,
    url: &str,
    archive: &Archive<'_>,
    out: &Path,
) -> Result<(), XtaskError> {
    let version = stem.version.replace('-', "_");
    let repository = stem.repository;
    let binary = archive.platform.binary;
    let sha256 = &archive.sha256;
    let source = format!("{}::{url}", archive.name);
    let description = "Local agent harness (release binary)";
    let package = "sprawling-bin";
    super::write(
        &out.join("aur/PKGBUILD"),
        format!(
            r#"pkgname={package}
pkgver={version}
pkgrel=1
pkgdesc='{description}'
arch=('x86_64')
url='{repository}'
license=('MPL-2.0')
makedepends=('unzip')
source=('{source}')
sha256sums=('{sha256}')

package() {{
  local application
  application=$(dirname "$(find "$srcdir" -type f -name '{binary}' -print -quit)")
  install -d "$pkgdir/usr/lib/$pkgname" "$pkgdir/usr/bin"
  cp -a "$application/." "$pkgdir/usr/lib/$pkgname/"
  chmod 755 "$pkgdir/usr/lib/$pkgname/{binary}"
  ln -s "/usr/lib/$pkgname/{binary}" "$pkgdir/usr/bin/{binary}"
  install -Dm644 "$application/LICENSE" "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}}
"#
        )
        .as_bytes(),
    )?;
    super::write(
        &out.join("aur/.SRCINFO"),
        format!(
            "pkgbase = {package}\n\tpkgdesc = {description}\n\tpkgver = {version}\n\tpkgrel = 1\n\turl = {repository}\n\tarch = x86_64\n\tlicense = MPL-2.0\n\tmakedepends = unzip\n\tsource = {source}\n\tsha256sums = {sha256}\n\npkgname = {package}\n"
        )
        .as_bytes(),
    )
}
