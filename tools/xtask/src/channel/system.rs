// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! AUR projections of the release archives (xtask D32).

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
    for archive in archives {
        if archive.platform.os == "linux" && archive.platform.cpu == "x64" {
            let url = format!(
                "{}/releases/download/{tag}/{}",
                stem.repository, archive.name
            );
            aur(stem, &url, archive, out)?;
        }
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
    let package = kernel::release::AUR_PACKAGE_NAME;
    let directory = kernel::release::aur_install_directory();
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
  install -d "$pkgdir/{directory}" "$pkgdir/usr/bin"
  cp -a "$application/." "$pkgdir/{directory}/"
  chmod 755 "$pkgdir/{directory}/{binary}"
  ln -s "/{directory}/{binary}" "$pkgdir/usr/bin/{binary}"
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
