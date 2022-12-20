use include_dir::{include_dir, Dir};

use crate::Result;

static PUBLIC_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR../../../app/public");

pub async fn export() -> Result<()> {
    std::fs::create_dir_all("./app/public")?;

    PUBLIC_DIR.extract("./app/public")?;

    Ok(())
}