use k1tty_core::pack::{inspect_k1tty, pack_chat, unpack_chat};
use serde_json::Value;

const EXPECTED: &str = include_str!("fixtures/pack/expected.json");
const FIXTURES: &[(&str, &[u8])] = &[
    ("fast", include_bytes!("fixtures/pack/python_fast.k1tty")),
    (
        "balanced",
        include_bytes!("fixtures/pack/python_balanced.k1tty"),
    ),
    ("ultra", include_bytes!("fixtures/pack/python_ultra.k1tty")),
    ("fast", include_bytes!("fixtures/pack/python_lzma2.k1tty")),
];

#[test]
fn reads_original_python_archives_in_all_presets_and_codecs() {
    let expected: Value = serde_json::from_str(EXPECTED).unwrap();
    for (preset, data) in FIXTURES {
        let actual = unpack_chat(data).unwrap();
        assert_eq!(actual["meta"], expected["meta"]);
        assert_eq!(actual["entries"], expected["entries"]);
        assert_eq!(inspect_k1tty(data).unwrap()["preset"], *preset);
        let (repacked, _) = pack_chat(&actual, preset, true).unwrap();
        assert_eq!(
            unpack_chat(&repacked).unwrap()["entries"],
            expected["entries"]
        );
    }
}

#[test]
fn rejects_truncated_and_checksum_corrupted_archives() {
    for (_, bytes) in FIXTURES {
        for length in [0, 7, 72, 103, bytes.len() - 1] {
            assert!(unpack_chat(&bytes[..length]).is_err());
        }
        let mut corrupt = bytes.to_vec();
        corrupt[104] ^= 0x80;
        assert!(unpack_chat(&corrupt).is_err());
    }
}
