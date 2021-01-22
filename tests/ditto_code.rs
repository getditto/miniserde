extern crate miniserde_ditto as serde;
use ::miniserde_ditto::cbor as serde_cbor;

use ::serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct V2AttachmentChunk {
    #[serde(rename = "id")]
    id: [u8; 24],
    #[serde(rename = "o")]
    offset: u64,
    #[serde(with = "serde_bytes")]
    #[serde(rename = "c")]
    chunk: Vec<u8>,
}

#[rustfmt::skip]
#[test]
fn v2_attachment_chunk() {
    const BYTES: &[u8] = &[
        0xa3, // 3-long map
            0x62, // 2-long string
                b'i', b'd',
            0x58, // byte seq with non-small-u8 length
                0x18, // length: 0x18 = 24
                0, 0, 0, 0,  0, 0, 0, 0,
                0, 0, 0, 0,  0, 0, 0, 0,
                0, 0, 0, 0,  0, 0, 0, 0,

            0x61, // 1-long string
                b'o',
            0x18, // non-small-u8
                0x2a, // 0x2a = 42

            0x61, // 1-long string
                b'c',
            0x42, // 2-long byte-seq
                0xde, 0xad,
    ];

    let ref instance = V2AttachmentChunk {
        id: [0; 24],
        offset: 42,
        chunk: vec![0xde, 0xad]
    };

    assert_eq!(serde_cbor::to_vec(instance).unwrap(), BYTES);
    assert_eq!(serde_cbor::from_slice::<V2AttachmentChunk>(BYTES).unwrap(), *instance)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct _Generic<T> {
    _it: T,
}

// // Enum
// #[derive(Debug, Serialize, Deserialize)]
// #[serde(untagged)]
// pub enum NonExhaustive<T> {
//     Known(T),
//     #[serde(skip_serializing)]
//     Unknown(serde::de::IgnoredAny),
// }
