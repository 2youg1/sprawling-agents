// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use kernel::{B3Hash, ImageRef, ImageType, Locator};

fn encoded_picture(format: image::ImageFormat) -> Vec<u8> {
    let mut bytes = Vec::new();
    image::DynamicImage::new_rgb8(1, 1)
        .write_to(&mut std::io::Cursor::new(&mut bytes), format)
        .unwrap();
    bytes
}

#[test]
fn pictures_come_back_as_attachments_instead_of_utf8_errors() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("lab")).unwrap();
    let (tool, _catalog) = tool(dir.path());
    for (extension, format, media_type) in [
        ("png", image::ImageFormat::Png, ImageType::Png),
        ("jpg", image::ImageFormat::Jpeg, ImageType::Jpeg),
        ("webp", image::ImageFormat::WebP, ImageType::Webp),
        ("gif", image::ImageFormat::Gif, ImageType::Gif),
    ] {
        let bytes = encoded_picture(format);
        let asked = format!("lab/picture.{extension}");
        std::fs::write(dir.path().join(&asked), &bytes).unwrap();

        let outcome = tool.invoke(&call(&asked)).unwrap();
        assert!(
            outcome.result.as_map()["text"]
                .as_str()
                .unwrap()
                .contains("picture attached"),
            "{asked}"
        );
        let hash = B3Hash::digest(&bytes);
        assert_eq!(
            outcome.attachments,
            vec![ImageRef {
                locator: Locator::cas(hash),
                media_type,
                width: 1,
                height: 1,
            }],
            "{asked}"
        );
        assert_eq!(
            storage::Cas::open(&kernel::layout::CityLayout::new(dir.path()).cas())
                .unwrap()
                .get(&hash)
                .unwrap(),
            bytes,
            "{asked}"
        );
    }
}

#[test]
fn a_cas_picture_is_recognized_by_its_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let store = kernel::layout::CityLayout::new(dir.path()).cas();
    let bytes = encoded_picture(image::ImageFormat::Png);
    let origin = storage::BlockOrigin {
        run: kernel::RunId::from_bytes([7; 16]),
        building: kernel::Address::parse("lab").unwrap(),
    };
    let hash = storage::Cas::open(&store)
        .unwrap()
        .put_for(&bytes, &origin)
        .unwrap();
    let (tool, _catalog) = tool(dir.path());

    let outcome = tool.invoke(&call(&format!("cas:b3-{hash}"))).unwrap();
    assert_eq!(outcome.attachments.len(), 1);
    assert_eq!(outcome.attachments[0].locator, Locator::cas(hash));
}

#[test]
fn a_picture_one_byte_past_four_mib_is_refused_by_the_picture_cap() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("lab")).unwrap();
    let mut bytes = encoded_picture(image::ImageFormat::Png);
    bytes.resize(4_194_305, 0);
    std::fs::write(dir.path().join("lab/large.png"), bytes).unwrap();
    let (tool, _catalog) = tool(dir.path());

    let err = tool.invoke(&call("lab/large.png")).unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    assert!(err.recovery().contains("4194304"), "{err}");
}
