//! Specifications
//!
//! v89a: https://www.w3.org/Graphics/GIF/spec-gif89a.txt
//! v87a: https://www.w3.org/Graphics/GIF/spec-gif87.txt

use giffer::{decoder, Version};
use std::fs;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let path = "/home/agnipau/Downloads/Telegram Desktop/fiero.gif";
    let orig_data = fs::read(path)?;
    let parsed_data = decoder::decode(&orig_data, false)?;
    let data = parsed_data.encode(&Version::V89a, false);
    assert_eq!(orig_data, data);
    fs::write("/home/agnipau/Downloads/Telegram Desktop/fiero2.gif", data)?;

    Ok(())
}
