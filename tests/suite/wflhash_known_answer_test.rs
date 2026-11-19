// WFLHASH known-answer tests (characterization).
//
// The other WFLHASH tests check properties (deterministic, different inputs
// give different outputs, salts and keys matter), but none of them pins the
// actual output. A change inside the permutation, padding, or squeeze could
// therefore change every hash while all of them still pass, and silently
// invalidate hashes and MACs that programs have already stored.
//
// These vectors were captured from the unchanged `dev` implementation
// (base 6ad047b) before the Rust 1.100 clippy refactor of
// `WflHashState::permute`. Any change to them is a breaking change to stored
// WFLHASH/WFLMAC values and needs the documented deprecation path, not an
// update of the expected strings.
//
// Inputs cover: empty input, short text, the padding boundaries around one
// 64-byte block (55, 56, 63, 64, 65 bytes), a long multi-block input,
// non-ASCII text, non-UTF-8 bytes, WFLHASH-512, salts (empty, short, and
// longer than the 16 bytes that are used), and WFLMAC-256 with short and
// long keys.

use std::sync::Arc;
use wfl::interpreter::value::Value;
use wfl::stdlib::crypto::{
    native_wflhash256, native_wflhash256_binary, native_wflhash256_with_salt, native_wflhash512,
    native_wflmac256, wflmac256_verify,
};

use crate::common::{expect_text_result as expect_text, run_wfl_code};

fn text(s: &str) -> Value {
    Value::Text(Arc::from(s))
}

fn hex_of(result: Result<Value, wfl::interpreter::error::RuntimeError>) -> String {
    match result.expect("hash function returned an error") {
        Value::Text(t) => t.to_string(),
        other => panic!("expected a hex text value, got {other:?}"),
    }
}

fn wflhash256(input: &str) -> String {
    hex_of(native_wflhash256(vec![text(input)]))
}

fn wflhash512(input: &str) -> String {
    hex_of(native_wflhash512(vec![text(input)]))
}

fn wflhash256_with_salt(input: &str, salt: &str) -> String {
    hex_of(native_wflhash256_with_salt(vec![text(input), text(salt)]))
}

fn wflmac256(input: &str, key: &str) -> String {
    hex_of(native_wflmac256(vec![text(input), text(key)]))
}

const QUICK_FOX: &str = "The quick brown fox jumps over the lazy dog";

#[test]
fn wflhash256_known_answers() {
    let cases: [(&str, String, &str); 9] = [
        (
            "empty",
            String::new(),
            "6c435e5e2f34d5403c9499616b0ab4df5fea69a9b1cbbd1590cbf64c406cc97b",
        ),
        (
            "abc",
            "abc".to_string(),
            "811b0ca8e11aaa45adb9dd69adcca767b53f12571c0e0477ec52930440fff477",
        ),
        (
            "quick fox",
            QUICK_FOX.to_string(),
            "edb679ecbd9eadac1e152698c14a62d8e8d63eeb54559b52d81915ed5556bfda",
        ),
        (
            "55 bytes",
            "a".repeat(55),
            "23efb2228538cd02fe4fb4f1b13b14d73334e3f029df652bc27c205a5bf1f920",
        ),
        (
            "56 bytes",
            "a".repeat(56),
            "ed2ec657d3ad571fdc72150575a604f455ebb666cfa9e86947979d08ec1ac2d3",
        ),
        (
            "63 bytes",
            "a".repeat(63),
            "717a2cc77aa6f03f8b7f9f2ea23e8328ce34f33686f795668e31317eaacaad20",
        ),
        (
            "64 bytes",
            "a".repeat(64),
            "53f51cf48b14f232c94b707ed1c71d5e19d1a3246d4a890f05404f44abf97f4a",
        ),
        (
            "65 bytes",
            "a".repeat(65),
            "8b63e2e31fb1a12bddf0263e64101bcdd8058d5883dc7eb89d6eb50834dcbbbe",
        ),
        (
            "1000 bytes",
            "a".repeat(1000),
            "3d1060c919f082b0205dee19d0bc82b9a31de5849f22380ac6c57d688705df9e",
        ),
    ];
    for (label, input, expected) in cases {
        assert_eq!(wflhash256(&input), expected, "WFLHASH-256 of {label}");
    }
}

#[test]
fn wflhash256_known_answer_for_non_ascii_text() {
    assert_eq!(
        wflhash256("héllo wörld ✓"),
        "314e1e30fc4880a9434c6722b49b9aa58468c8dc36c14931fc7f832cc11832a0",
        "WFLHASH-256 of non-ASCII text"
    );
}

#[test]
fn wflhash256_binary_known_answers() {
    let all_bytes: Vec<u8> = (0u8..=255).collect();
    assert_eq!(
        native_wflhash256_binary(&all_bytes).unwrap(),
        "02b80bfb9d12334b5f98dc997b67de6ac1170a3796838b32e7cc9ea2fcc5e97f",
        "WFLHASH-256 of the bytes 0..=255 (not valid UTF-8)"
    );
    assert_eq!(
        native_wflhash256_binary(b"abc").unwrap(),
        wflhash256("abc"),
        "binary and text WFLHASH-256 agree on the same bytes"
    );
}

#[test]
fn wflhash512_known_answers() {
    let cases: [(&str, String, &str); 3] = [
        (
            "empty",
            String::new(),
            "9664f1da98b8ecf1fc3b98096ede6813c4a63863446e7d49999b7d4cba8c6093853cf2075e3b6ebf6d56f9356d45627eaeb46f8f09f2652bb5d3d2ddd0902a55",
        ),
        (
            "abc",
            "abc".to_string(),
            "433334b63fdc664c5be708143e2612e253cf9f1047788a3b36c1205665613383325d52e45c187f6197b4f6b60e79633424eeaf0281023dc53a4de83fb7451f20",
        ),
        (
            "1000 bytes",
            "a".repeat(1000),
            "70b62991fd95a44c265682fb895f197220d2dd7627850adb0e03ee832f72b8ad3949aa6165d5f138d963965ab7bbdd9e08a9391364bcfcb92f28b0fefbefae3d",
        ),
    ];
    for (label, input, expected) in cases {
        assert_eq!(wflhash512(&input), expected, "WFLHASH-512 of {label}");
    }
}

#[test]
fn wflhash256_with_salt_known_answers() {
    let cases: [(&str, &str, &str); 4] = [
        (
            "empty salt",
            "",
            "03bf13a4e2bc617ddef5f537a364cfc819ea02e9b044d0543ee17477db4d2727",
        ),
        (
            "short salt",
            "salt",
            "f86bcd20c3d9dfe714c55b1f5f156215aed6b5e049c6700353184080ceadb036",
        ),
        (
            "16-byte salt",
            "0123456789abcdef",
            "7d506df74a43d4b52c7874660a142508b50812fd42510163a83140c2833927f3",
        ),
        (
            "salt longer than 16 bytes",
            "0123456789abcdefXYZ",
            "7d506df74a43d4b52c7874660a142508b50812fd42510163a83140c2833927f3",
        ),
    ];
    for (label, salt, expected) in cases {
        assert_eq!(
            wflhash256_with_salt("abc", salt),
            expected,
            "WFLHASH-256 of abc with {label}"
        );
    }
}

#[test]
fn wflmac256_known_answers() {
    let long_key = "k".repeat(100);
    let cases: [(&str, &str, &str, &str); 3] = [
        (
            "abc with a short key",
            "abc",
            "key",
            "9eff88883e11b3ee44e9074244ff22f3cb7335207d81081c3a982fe56afb5ac4",
        ),
        (
            "empty message",
            "",
            "key",
            "59395b59239b80d08b0ae90cdc5698a73dedefac1e75759d4c10a95a56532836",
        ),
        (
            "abc with a 100-byte key",
            "abc",
            &long_key,
            "a2b244d85e4ef6841e35129e81c31837285b58e5357db1510b877992f5893ba3",
        ),
    ];
    for (label, message, key, expected) in cases {
        assert_eq!(wflmac256(message, key), expected, "WFLMAC-256 of {label}");
    }
}

#[test]
fn wflmac256_verify_accepts_the_known_mac_and_rejects_others() {
    let mac = "9eff88883e11b3ee44e9074244ff22f3cb7335207d81081c3a982fe56afb5ac4";
    assert!(
        wflmac256_verify(b"abc", b"key", mac).unwrap(),
        "the known WFLMAC-256 of abc under key must verify"
    );

    let mut wrong = mac.to_string();
    let last = if wrong.ends_with('0') { "1" } else { "0" };
    wrong.replace_range(wrong.len() - 1.., last);
    assert!(
        !wflmac256_verify(b"abc", b"key", &wrong).unwrap(),
        "a MAC differing in one hex digit must not verify"
    );
    assert!(
        !wflmac256_verify(b"abd", b"key", mac).unwrap(),
        "the MAC of abc must not verify for a different message"
    );
    assert!(
        !wflmac256_verify(b"abc", b"kez", mac).unwrap(),
        "the MAC of abc must not verify under a different key"
    );
}

#[tokio::test]
async fn wflhash256_known_answer_through_wfl_code() {
    let code = r#"
        store result as wflhash256 of "abc"
    "#;
    assert_eq!(
        expect_text(run_wfl_code(code).await),
        wflhash256("abc"),
        "the WFL builtin returns the same WFLHASH-256 as the native function"
    );
}
