// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One MCP server's answer as the model reads it (`crates/runtime/spec/Pipeline.lean`
//! §8-27-10): text that fits passes untouched, text that does not is
//! stored whole and replaced by a window the model pages with `read`,
//! a PNG picture is stored in the content store and travels as an
//! attachment, with a line of text where it stood, and a sound is stored
//! there too and named by its locator in a line of its own.

use base64::Engine as _;
use kernel::consts_policy::IMAGE_MAX_BYTES;
use kernel::{AxCode, AxError, B3Hash, ImageRef, ImageType, Locator, Payload, ToolOutcome};
use serde_json::{Map, Value};

use crate::offload::OffloadSite;

use super::{PackContext, package};

/// The most text one connector answer puts into the window.
pub const CONNECTOR_CAP_BYTES: u64 = 16_384;

/// Packages one connector answer for the window.
///
/// An answer without a `content` array comes back untouched. Otherwise
/// the text comes first: if it fits [`CONNECTOR_CAP_BYTES`] it stays,
/// and if it does not, the text blocks, joined in order, are stored
/// whole through [`package`] and replaced by one text block holding the
/// substitute, every other block following it in its original order,
/// with the account of the move in the result's `offload` field. Then
/// every `image` block is replaced where it stands: a PNG this city can
/// measure is stored in the content store and added to the attachments,
/// and any other picture becomes a sentence saying why it was left out.
/// Every `audio` block is replaced the same way, by a line naming the
/// locator it was stored under, and is no attachment: the model cannot
/// hear it, and a tool that reads recordings takes the locator. The
/// pictures and sounds come second so that the line standing for one is
/// never folded into text the model has to page to find, and no base64
/// reaches the window or the ledger either way.
///
/// # Errors
/// Propagates whatever [`package`] reports about the store, and a
/// content store that will not take a picture.
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

/// Every `image` and `audio` block of an answer replaced by the words
/// that stand for it, with the pictures this city could measure added as
/// attachments.
///
/// # Errors
/// Propagates a content store that will not take the bytes.
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

/// The text step: an answer whose text fits comes back as it is, and
/// one whose text does not is stored and paged.
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

/// One `image` block as the words that stand in its place, and the
/// attachment it became when this city could measure it.
///
/// # Errors
/// Propagates a content store that will not take the bytes.
fn pictured(
    block: &Value,
    offload: &mut OffloadSite<'_>,
) -> Result<(Value, Option<ImageRef>), AxError> {
    let (bytes, picture) = match measured(block) {
        Ok(measured) => measured,
        Err(why) => return Ok((text_block(format!("[picture left out: {why}]")), None)),
    };
    // The store hashes what it takes the way `png_picture` named it, so
    // the locator the picture already carries is the one it is kept under.
    offload
        .cas
        .put_for(&bytes, &offload.origin)
        .map_err(storage::StorageError::into_ax)?;
    let words = format!(
        "[picture attached: image/png {}x{}, {}]",
        picture.width, picture.height, picture.locator
    );
    Ok((text_block(words), Some(picture)))
}

/// The bytes of a picture this city can carry and the picture they are,
/// or the reason they cannot be carried, as the words the model reads in
/// its place.
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

/// The picture `bytes` are, as this city carries one: a PNG inside
/// [`IMAGE_MAX_BYTES`] whose header gives its two sides, referred to by
/// the `cas:` locator its bytes hash to (`crates/runtime/spec/Tools/BoundReader.lean` §8-59).
///
/// The one recognition a picture gets, whether it arrived in a
/// connector's answer or was named to a tool. Only PNG is measured, for
/// the reason the browser's screenshots give: another format needs a
/// second decoder to read its sides, and a side read wrong is worse than
/// none.
///
/// # Errors
/// The refusal [`IMAGE_MAX_BYTES`] gives bytes past it, and
/// `E_INVALID_ARGS` when the bytes are not a PNG whose header reads.
pub fn png_picture(bytes: &[u8]) -> Result<ImageRef, AxError> {
    let locator = Locator::cas(B3Hash::digest(bytes));
    IMAGE_MAX_BYTES.admit(&locator, bytes.len())?;
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

/// One `audio` block as the line that names where it was stored, or the
/// reason it was left out.
///
/// # Errors
/// Propagates a content store that will not take the bytes.
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

/// The bytes of a sound block and the media type it names, or the
/// reason they cannot be carried. The container is not judged here: the
/// tool that reads a recording judges it (runtime D15).
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

/// Whether a block is one MCP marks `"type": "audio"`.
fn is_sound(block: &Value) -> bool {
    block.get("type").and_then(Value::as_str) == Some("audio")
}

/// Whether a block is one MCP marks `"type": "image"`.
fn is_picture(block: &Value) -> bool {
    block.get("type").and_then(Value::as_str) == Some("image")
}

/// One text block holding `text`.
fn text_block(text: String) -> Value {
    let mut block = Map::new();
    block.insert("type".to_owned(), Value::String("text".to_owned()));
    block.insert("text".to_owned(), Value::String(text));
    Value::Object(block)
}

/// The text of a block MCP marks `"type": "text"`, and nothing else.
fn text_of(block: &Value) -> Option<&str> {
    (block.get("type").and_then(Value::as_str) == Some("text"))
        .then(|| block.get("text").and_then(Value::as_str))
        .flatten()
}

#[cfg(test)]
mod tests;
