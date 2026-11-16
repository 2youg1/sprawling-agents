// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use base64::Engine as _;
use kernel::consts_policy::IMAGE_MAX_BYTES;
use kernel::{AxCode, AxError, B3Hash, ImageRef, ImageType, Locator, Payload, ToolOutcome};
use serde_json::{Map, Value};

use crate::offload::OffloadSite;

use super::{PackContext, package};

pub const CONNECTOR_CAP_BYTES: u64 = 16_384;

pub fn package_connector(
    outcome: ToolOutcome,
    mut offload: OffloadSite<'_>,
) -> Result<ToolOutcome, AxError> {
    let paged = windowed(
        outcome,
        OffloadSite {
            cas: &mut *offload.cas,
            city_root: offload.city_root,
            room: offload.room,
            origin: offload.origin.clone(),
        },
    )?;
    with_media_stored(paged, &mut offload)
}

fn with_media_stored(
    outcome: ToolOutcome,
    offload: &mut OffloadSite<'_>,
) -> Result<ToolOutcome, AxError> {
    let Some(Value::Array(blocks)) = outcome.result.as_map().get("content") else {
        return Ok(outcome);
    };
    if !blocks
        .iter()
        .any(|block| is_picture(block) || is_sound(block))
    {
        return Ok(outcome);
    }
    let mut attachments = outcome.attachments.clone();
    let mut content = Vec::with_capacity(blocks.len());
    for block in blocks {
        if is_picture(block) {
            let (words, picture) = pictured(block, offload)?;
            content.push(words);
            attachments.extend(picture);
        } else if is_sound(block) {
            content.push(heard(block, offload)?);
        } else {
            content.push(block.clone());
        }
    }
    let mut result = outcome.result.as_map().clone();
    result.insert("content".to_owned(), Value::Array(content));
    Ok(ToolOutcome {
        result: Payload::new(result)?,
        attachments,
    })
}

fn windowed(outcome: ToolOutcome, offload: OffloadSite<'_>) -> Result<ToolOutcome, AxError> {
    let Some(Value::Array(blocks)) = outcome.result.as_map().get("content") else {
        return Ok(outcome);
    };
    let (texts, others): (Vec<&Value>, Vec<&Value>) =
        blocks.iter().partition(|block| text_of(block).is_some());
    let text = texts
        .iter()
        .filter_map(|block| text_of(block))
        .collect::<Vec<&str>>()
        .join("\n");
    if u64::try_from(text.len()).is_ok_and(|len| len <= CONNECTOR_CAP_BYTES) {
        return Ok(outcome);
    }
    let packaged = package(
        text.as_bytes(),
        PackContext {
            cap_bytes: CONNECTOR_CAP_BYTES,
            stamp: None,
            net_notice: false,
            steer: None,
            reminder: None,
            offload: Some(offload),
            sieve: None,
            adviser: None,
        },
    )?;
    let content = std::iter::once(text_block(packaged.content))
        .chain(others.into_iter().cloned())
        .collect();
    let accounts = packaged
        .events
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<Value>, _>>()
        .map_err(|err| {
            AxError::failure(
                AxCode::InvalidArgs,
                "encode offload account",
                err.to_string(),
            )
            .with_recovery(
                "report this against runtime::pipeline::connector: an offload account \
                     holds names and counts, and JSON refuses neither",
            )
        })?;
    let mut result = outcome.result.as_map().clone();
    result.insert("content".to_owned(), Value::Array(content));
    result.insert(super::CONNECTOR_ACCOUNTS.to_owned(), Value::Array(accounts));
    Ok(ToolOutcome {
        result: Payload::new(result)?,
        attachments: outcome.attachments,
    })
}

fn pictured(
    block: &Value,
    offload: &mut OffloadSite<'_>,
) -> Result<(Value, Option<ImageRef>), AxError> {
    let (bytes, picture) = match measured(block) {
        Ok(measured) => measured,
        Err(why) => return Ok((text_block(format!("[picture left out: {why}]")), None)),
    };

    offload
        .cas
        .put_for(&bytes, &offload.origin)
        .map_err(storage::StorageError::into_ax)?;
    let words = picture_line(&picture);
    Ok((text_block(words), Some(picture)))
}

fn measured(block: &Value) -> Result<(Vec<u8>, ImageRef), String> {
    if block.get("mimeType").and_then(Value::as_str) != Some(ImageType::Png.mime()) {
        return Err("only png is measured here; ask the tool for png".to_owned());
    }
    let data = block
        .get("data")
        .and_then(Value::as_str)
        .ok_or_else(|| "it carries no data".to_owned())?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|err| format!("its bytes are not base64: {err}"))?;
    let picture = png_picture(&bytes)
        .map_err(|refused| format!("{}; {}", refused.subject(), refused.recovery()))?;
    Ok((bytes, picture))
}

pub fn png_picture(bytes: &[u8]) -> Result<ImageRef, AxError> {
    let locator = picture_locator(bytes)?;
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let (width, height) = decoder
        .read_header_info()
        .map(|info| (info.width, info.height))
        .map_err(|err| {
            AxError::failure(
                AxCode::InvalidArgs,
                "read a picture",
                format!("its png header does not read: {err}"),
            )
            .with_recovery("only png is read here; ask for the picture as png")
        })?;
    Ok(ImageRef {
        locator,
        media_type: ImageType::Png,
        width,
        height,
    })
}

pub(crate) fn picture_line(picture: &ImageRef) -> String {
    format!(
        "[picture attached: {} {}x{}, {}]",
        picture.media_type.mime(),
        picture.width,
        picture.height,
        picture.locator
    )
}

pub(crate) fn read_picture(bytes: &[u8]) -> Result<Option<ImageRef>, AxError> {
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|err| {
            AxError::failure(
                AxCode::InvalidArgs,
                "read a picture",
                format!("its format cannot be read: {err}"),
            )
            .with_recovery("pass a complete PNG, JPEG, WebP or GIF")
        })?;
    let Some(media_type) = reader.format().and_then(picture_type) else {
        return Ok(None);
    };
    let (width, height) = reader.into_dimensions().map_err(|err| {
        AxError::failure(
            AxCode::InvalidArgs,
            "read a picture",
            format!("its {} header does not read: {err}", media_type.mime()),
        )
        .with_recovery("pass a complete PNG, JPEG, WebP or GIF within the picture size limit")
    })?;
    Ok(Some(ImageRef {
        locator: picture_locator(bytes)?,
        media_type,
        width,
        height,
    }))
}

#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "image::ImageFormat is upstream and non-exhaustive; unsupported formats return None"
)]
fn picture_type(format: image::ImageFormat) -> Option<ImageType> {
    match format {
        image::ImageFormat::Png => Some(ImageType::Png),
        image::ImageFormat::Jpeg => Some(ImageType::Jpeg),
        image::ImageFormat::WebP => Some(ImageType::Webp),
        image::ImageFormat::Gif => Some(ImageType::Gif),
        _ => None,
    }
}

fn picture_locator(bytes: &[u8]) -> Result<Locator, AxError> {
    let locator = Locator::cas(B3Hash::digest(bytes));
    IMAGE_MAX_BYTES.admit(&locator, bytes.len())?;
    Ok(locator)
}

fn heard(block: &Value, offload: &mut OffloadSite<'_>) -> Result<Value, AxError> {
    let (bytes, media) = match sound_of(block) {
        Ok(sound) => sound,
        Err(why) => return Ok(text_block(format!("[recording left out: {why}]"))),
    };
    let hash = offload
        .cas
        .put_for(&bytes, &offload.origin)
        .map_err(storage::StorageError::into_ax)?;
    Ok(text_block(format!(
        "[recording attached: {media}, {} bytes, {}]",
        bytes.len(),
        Locator::cas(hash)
    )))
}

fn sound_of(block: &Value) -> Result<(Vec<u8>, &str), String> {
    let media = block
        .get("mimeType")
        .and_then(Value::as_str)
        .filter(|media| media.starts_with("audio/"))
        .ok_or_else(|| "it names no audio media type".to_owned())?;
    let data = block
        .get("data")
        .and_then(Value::as_str)
        .ok_or_else(|| "it carries no data".to_owned())?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|err| format!("its bytes are not base64: {err}"))?;
    if bytes.is_empty() {
        return Err("it carries no bytes".to_owned());
    }
    Ok((bytes, media))
}

fn is_sound(block: &Value) -> bool {
    block.get("type").and_then(Value::as_str) == Some("audio")
}

fn is_picture(block: &Value) -> bool {
    block.get("type").and_then(Value::as_str) == Some("image")
}

fn text_block(text: String) -> Value {
    let mut block = Map::new();
    block.insert("type".to_owned(), Value::String("text".to_owned()));
    block.insert("text".to_owned(), Value::String(text));
    Value::Object(block)
}

fn text_of(block: &Value) -> Option<&str> {
    (block.get("type").and_then(Value::as_str) == Some("text"))
        .then(|| block.get("text").and_then(Value::as_str))
        .flatten()
}

#[cfg(test)]
mod tests;
