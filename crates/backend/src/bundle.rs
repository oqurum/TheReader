use include_dir::{include_dir, Dir};

use crate::Result;

static PUBLIC_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR../../../app/public");

// TODO: Use a virtual directory?

// TODO: Improve. Very basic.

pub async fn export() -> Result<()> {
    std::fs::create_dir_all("./app/public/dist");

    for entry in std::fs::read_dir("./app/public/dist")? {
        let entry = entry?;

        if !PUBLIC_DIR.contains(format!("dist/{}", entry.file_name().to_str().unwrap())) {
            tracing::info!("Removing Existing /dist Folder");

            std::fs::remove_dir_all("./app/public/dist")?;

            break;
        }
    }

    PUBLIC_DIR.extract("./app/public")?;

    Ok(())
}