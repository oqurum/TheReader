use std::fs;

fn main() {
    let contents = grass::from_path(
        "../../global_common/scss/main.scss",
        &grass::Options::default(),
    )
    .expect("Unable to parse SCSS");

    fs::write("../../app/public/css/gcommon.css", contents).expect("Failed writing to file");
}
