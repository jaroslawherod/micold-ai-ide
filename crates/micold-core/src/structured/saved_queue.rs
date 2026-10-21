//! The `.queue` file payload (feature 941, research R9, data-model *SavedQueue*): a structured
//! session's queued messages, pictures included, saved under **Save terminal history** so they
//! survive a daemon restart. Framed as a saved terminal history is, with its own magic and version.

use serde::{Deserialize, Serialize};

use super::attachment::{check_images, ImageAttachment};
use super::input::Message;
use crate::terminal_history::format::{frame, unframe};
use crate::terminal_history::DamageReason;

/// The version written into every `.queue` file; a file of any other version is damaged.
pub const QUEUE_FORMAT_VERSION: u32 = 1;

/// The largest `.queue` file a read accepts, checked from the file's length before it is read.
pub const MAX_QUEUE_FILE_BYTES: u64 = 1024 * 1024 * 1024;

const MAGIC: [u8; 8] = *b"MICOLDSQ";

#[derive(Serialize, Deserialize)]
struct SavedQueue {
    entries: Vec<(String, Vec<ImageAttachment>)>,
}

/// The `.queue` file bytes for `entries`, in delivery order.
pub fn encode_queue(entries: &[Message]) -> Vec<u8> {
    let saved = SavedQueue {
        entries: entries
            .iter()
            .map(|m| (m.text.clone(), m.images.clone()))
            .collect(),
    };
    // Writing strings and bytes into a `Vec` has no failure of its own.
    let payload = postcard::to_allocvec(&saved).expect("a saved queue serialises into memory");
    frame(&MAGIC, QUEUE_FORMAT_VERSION, &payload)
}

/// The queued messages `bytes` hold, or why they are not restored. A picture over the limits or
/// not of the format it names makes the file damaged, as a file edited by hand would be.
pub fn decode_queue(bytes: &[u8]) -> Result<Vec<Message>, DamageReason> {
    let payload = unframe(bytes, &MAGIC, QUEUE_FORMAT_VERSION, MAX_QUEUE_FILE_BYTES)?;
    let saved = match postcard::take_from_bytes::<SavedQueue>(payload) {
        Ok((saved, [])) => saved,
        _ => return Err(DamageReason::Malformed),
    };
    saved
        .entries
        .into_iter()
        .map(|(text, images)| {
            check_images(&images).map_err(|_| DamageReason::Malformed)?;
            Ok(Message { text, images })
        })
        .collect()
}
