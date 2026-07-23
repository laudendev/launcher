use ed25519_dalek::VerifyingKey;
use quartermaster_license::{fingerprint::fingerprint, verify_any};
use serde::Serialize;
use std::fs;
use std::io::Write;

const PUBLIC_KEYS_HEX: &[&str] = &[
    "eba29494abda910c3670ab0aab126cbca5062130f54c3ad0bcbc9d5aa8d6b9ca",
];

const DOWNLOAD_URL: &str = "https://quartermaster.lauden.dev/license/download";

#[derive(Serialize)]
struct DownloadRequest {
    license_key: String,
    fingerprint: String,
}

fn load_public_keys() -> Vec<VerifyingKey> {
    PUBLIC_KEYS_HEX
        .iter()
        .map(|hex_str| {
            let bytes = hex::decode(hex_str).expect("invalid public key hex in binary");
            let arr: [u8; 32] = bytes.try_into().expect("public key must be 32 bytes");
            VerifyingKey::from_bytes(&arr).expect("invalid public key bytes")
        })
        .collect()
}

fn prompt_for_license_key() -> String {
    print!("Enter your license key: ");
    std::io::stdout().flush().ok();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).expect("could not read input");
    input.trim().to_string()
}

fn main() {
    let pubs = load_public_keys();

    // Testing mode: always prompt fresh, never load or save a stored
    // key, so every run exercises the full verify + download path
    // for whatever key is entered, regardless of product.
    let raw_key = prompt_for_license_key();

    let license = verify_any(&pubs, &raw_key).expect("license failed to verify");

    println!(
        "License verified — product: {}, seats: {}",
        license.product, license.seats
    );

    let fp = fingerprint(&license.product).expect("could not compute machine fingerprint");

    let body = DownloadRequest {
        license_key: raw_key,
        fingerprint: fp,
    };

    println!("Requesting download...");
    let response = ureq::post(DOWNLOAD_URL)
        .send_json(&body)
        .expect("download request failed");

    let mut file = fs::File::create(format!("{}.zip", license.product))
        .expect("could not create output file");
    let mut reader = response.into_reader();
    std::io::copy(&mut reader, &mut file).expect("could not write downloaded file");

    println!("Downloaded {}.zip successfully.", license.product);
}
