use lazy_static::lazy_static;
use regex::{Regex, RegexBuilder};

const WORD_DELIMITER: [char; 8] = [' ', '_', '-', '.', ',', '=', '\'', '|'];
const SPECIAL_CHARS: [char; 4] = ['&', ':', '\\', '/'];
const USELESS: [char; 3] = [' ', '_', '-'];

// TODO: Remember, usually the first word is the title, so we should be able to use that to determine the series name too.

lazy_static! {
    /// t01, t1, t01-t02, t1-t2
    static ref TOME_NUMBER: Regex = RegexBuilder::new(r"t#?(\d+)(?:-t?(\d+))?").case_insensitive(true).build().unwrap();
    /// tome 01, tome 1, tome 01-02, tome 1-2
    static ref TOME_NUMBER_WITH_TEXT: Regex = RegexBuilder::new(r"tome #?(\d+)(?:-(\d+))?").case_insensitive(true).build().unwrap();

    /// p01, p1, p01-p02, p1-p2
    static ref PART_NUMBER: Regex = RegexBuilder::new(r"p#?(\d+)(?:-p?(\d+))?").case_insensitive(true).build().unwrap();
    /// part 01, part 1, part 01-02, part 1-2
    static ref PART_NUMBER_WITH_TEXT: Regex = RegexBuilder::new(r"part #?(\d+)(?:-(\d+))?").case_insensitive(true).build().unwrap();

    /// ch(apter)01, ch(apter) 1
    static ref CHAPTER_NUMBER: Regex = RegexBuilder::new(r"ch(?:apter)?\s?(\d+)").case_insensitive(true).build().unwrap();

    /// prologue01, prologue 1
    static ref PROLOGUE_NUMBER: Regex = RegexBuilder::new(r"prologue?\s?(\d+)").case_insensitive(true).build().unwrap();

    /// vol(ume)1, vol1-2, vol 1, vol 1-vol2, vol 1-vol 2, vol #1, vol #1-vol#2, vol #1-vol #2, vol. 1
    static ref VOLUME_NUMBER: Regex = RegexBuilder::new(r"vol\.?(?:ume)?\s?#?(\d+)(?:-(?:vol\.?(?:ume)?)?\s?#?(\d+))?").case_insensitive(true).build().unwrap();

    /// v01, v1, v01-v02, v1-v2
    static ref VOLUME_NUMBER_SHORT: Regex = RegexBuilder::new(r"v\.?\s?#?(\d+)(?:-v?(\d+))?").case_insensitive(true).build().unwrap();

    /// .cbz, .cbr, .cb7, .cbt, .cba
    static ref ARCHIVE_EXTENSION: Regex = RegexBuilder::new(r"\.?(?:cbz|cbr|cb7|cbt|cba)(?:/[a-zA-Z0-9]+)?").case_insensitive(true).build().unwrap();

    /// 001-9999
    static ref MULTIPLE_CHAPTERS: Regex = RegexBuilder::new(r"#?\d+-\d+").case_insensitive(true).build().unwrap();

    /// Language code. E.g. EN, FR, DE, ES, JP, CN, KR, etc.
    static ref LANGUAGE_CODE: Regex = RegexBuilder::new(r"\s[A-Z]{2}(?:\s|$)").case_insensitive(false).build().unwrap();

    /// Disk letter. E.g. C:, D:, etc.
    static ref DISK_LETTER: Regex = RegexBuilder::new(r"[A-Z]:/").case_insensitive(false).build().unwrap();
}

pub fn extract_comic_volume(value: &str) -> Option<VolumeType> {
    fn find_one(value: &str) -> Option<VolumeType> {
        let regerts = [
            (0, &*PROLOGUE_NUMBER),
            (1, &*VOLUME_NUMBER),
            (1, &*VOLUME_NUMBER_SHORT),
            (1, &*CHAPTER_NUMBER),
            (1, &*TOME_NUMBER),
            (1, &*TOME_NUMBER_WITH_TEXT),
            (1, &*PART_NUMBER),
            (1, &*PART_NUMBER_WITH_TEXT),
        ];

        for (t, regex) in regerts {
            if let Some(cap) = regex.captures(value) {
                // TODO: Handle multiple chapter parts. Eg. ch01-ch02
                if cap.get(0)?.as_str().contains('-') {
                    continue;
                }

                if let Some(num) = cap.get(1) {
                    return Some(match t {
                        0 => VolumeType::Prologue(num.as_str().parse().unwrap()),
                        1 => VolumeType::Volume(num.as_str().parse().unwrap()),
                        _ => unreachable!(),
                    });
                }
            }
        }

        None
    }

    // Try to find a volume number.
    match find_one(value) {
        Some(v) => Some(v),
        None => {
            // Ensure we aren't dealing with multiple chapters.
            if MULTIPLE_CHAPTERS.is_match(value) {
                return None;
            }

            let mut value = value.to_string();
            remove_filler(&mut value);

            lazy_static! {
                static ref NUMBER: Regex = Regex::new(r"\d+").unwrap();
            }

            let mut last_found = None;

            // We want the last number. It's most likely the volume number.
            // Otherwise we could end up with a number from the Comic Book Title.
            for found in NUMBER.find_iter(&value) {
                let lower = value.to_lowercase();

                if lower.contains("prologue") {
                    last_found = Some(VolumeType::Prologue(found.as_str().parse().unwrap()));
                } else if lower.contains("chapter")
                    || lower.contains("volume")
                    || lower.contains("tome")
                {
                    last_found = Some(VolumeType::Volume(found.as_str().parse().unwrap()));
                } else {
                    last_found = Some(VolumeType::Unknown(found.as_str().parse().unwrap()));
                }
            }

            last_found
        }
    }
}

pub fn extract_name_from_path<V: AsRef<str>>(value: V) -> String {
    // Remove disk letter.
    let mut value = DISK_LETTER.replace(value.as_ref(), "").to_string();

    if value.starts_with('/') {
        value = value[1..].to_string();
    }

    // Detect if we are dealing with a file path. I.e. if the string contains a slash. Work backwards.
    if value.contains('/') {
        let mut paths = value.split('/').collect::<Vec<_>>();

        let file_name = strip_text(paths.pop().unwrap());
        let mut folder_name = strip_text(paths.pop().unwrap());

        if paths.is_empty() {
            // Will also remove a date. E.g. 2020-01-01. Only call on a folder name.
            folder_name = MULTIPLE_CHAPTERS.replace_all(&folder_name, "").to_string();
            // println!("- {file_name:?} || {folder_name:?}");

            if file_name.contains(&folder_name)
                || ((!folder_name.is_empty()) && folder_name.len() < file_name.len())
            {
                return folder_name;
            } else {
                return file_name;
            }
        } else {
            // the `folder_name` may not be the name of the book. E.g. "one piece/001-099/one piece - 001.cbz"

            // Will also remove a date. E.g. 2020-01-01. Only call on a folder name.
            folder_name = MULTIPLE_CHAPTERS.replace_all(&folder_name, "").to_string();

            while folder_name.trim().is_empty() && !paths.is_empty() {
                folder_name = strip_text(paths.pop().unwrap());
                folder_name = MULTIPLE_CHAPTERS.replace_all(&folder_name, "").to_string();
            }

            println!("++++ {file_name:?} || {folder_name:?}");

            if file_name.contains(&folder_name)
                || ((!folder_name.is_empty()) && folder_name.len() < file_name.len())
            {
                return folder_name;
            } else {
                return file_name;
            }
        }
    }

    // println!("Default strip_text");

    strip_text(value)
}

fn strip_text<V: ToString>(value: V) -> String {
    let mut value = value.to_string();

    // Replace underscores with spaces.
    value = value.replace('_', " ");

    // Regex Replaces
    value = TOME_NUMBER.replace_all(&value, "").to_string();
    value = TOME_NUMBER_WITH_TEXT.replace_all(&value, "").to_string();
    value = PART_NUMBER.replace_all(&value, "").to_string();
    value = PART_NUMBER_WITH_TEXT.replace_all(&value, "").to_string();
    value = VOLUME_NUMBER_SHORT.replace_all(&value, "").to_string();
    value = VOLUME_NUMBER.replace_all(&value, "").to_string();
    value = CHAPTER_NUMBER.replace_all(&value, "").to_string();
    value = ARCHIVE_EXTENSION.replace_all(&value, "").to_string();
    value = LANGUAGE_CODE.replace_all(&value, "").to_string();
    value = PROLOGUE_NUMBER.replace_all(&value, "").to_string();

    remove_filler(&mut value);

    // Remove double spaces.
    value = value.replace("  ", " ");

    // Remove concurrent dashes.
    value = value.replace("- -", "-");

    value
}

/// Remove filler words from the value.
///
/// For Example: Bracketed text, text in parentheses, etc.
fn remove_filler(value: &mut String) {
    // Remove text in brackets.
    while let Some((l_index, r_index)) = value
        .find('[')
        .and_then(|index| Some((index, value.chars().skip(index).position(|c| c == ']')?)))
    {
        value.drain(l_index..=l_index + r_index);
    }

    // Remove text in parentheses.
    while let Some((l_index, r_index)) = value
        .find('(')
        .and_then(|index| Some((index, value.chars().skip(index).position(|c| c == ')')?)))
    {
        value.drain(l_index..=l_index + r_index);
    }

    // Remove unnecessary characters at the end of the string.
    if let Some(amount) = value.chars().rev().position(|v| !USELESS.contains(&v)) {
        value.drain(value.len() - amount..);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeType {
    /// Prologues are before the first chapter.
    Prologue(u32),
    Volume(u32),
    /// We don't know what the volume type it is.
    Unknown(u32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_file_name() {
        assert_eq!(
            "Sports-Illustrated-1954-08-16",
            extract_name_from_path("Sports-Illustrated-1954-08-16")
        );
        assert_eq!(
            "No. 159 January 3rd 1992",
            extract_name_from_path("No. 159 January 3rd 1992")
        );
        assert_eq!("One Piece", extract_name_from_path("One Piece - Tome 01"));
        assert_eq!("One Piece", extract_name_from_path("One Piece T2-23"));
        assert_eq!(
            "Naruto",
            extract_name_from_path("Naruto - Tome #002 - [V1]")
        );
        assert_eq!("013 - Golf", extract_name_from_path("001-100/013 - Golf"));
        assert_eq!("JoJo's Bizarre Adventure", extract_name_from_path("JoJo's Bizarre Adventure - Part 01 - Phantom Blood T01 (Araki) [Digital-1920] [Manga FR]"));
        assert_eq!("JoJo's Bizarre Adventure", extract_name_from_path("JoJo's Bizarre Adventure - Part 01 - Phantom Blood T02 (Araki) [Digital-1920] [Manga FR]"));
        assert_eq!("JoJo's Bizarre Adventure", extract_name_from_path("JoJo's Bizarre Adventure - Part 02 - Battle Tendency T01 (Araki) [Digital-1920] [Manga FR]"));
        assert_eq!(
            "Dream Team",
            extract_name_from_path(
                "Dream Team T02 (Hinata) (2011) [Digital-1598] [Manga FR] (PapriKa)"
            )
        );
        assert_eq!("fairygirls", extract_name_from_path("fairygirls_vol1"));
        assert_eq!("fairytail", extract_name_from_path("fairytail_vol1"));
        assert_eq!(
            "Name Here",
            extract_name_from_path("Name Here (115 tomes) FR CBZ/002 - Name Here (Info 1994)")
        );
        assert_eq!(
            "Name Here",
            extract_name_from_path(
                "Name Here (115 tomes) EN CBZ/001-100/002 - Name Here (Info 1994)"
            )
        );
    }

    #[test]
    fn extract_volume() {
        // Volume Tests
        assert_eq!(
            Some(VolumeType::Volume(1)),
            extract_comic_volume("One Piece - Tome 01")
        );
        assert_eq!(
            Some(VolumeType::Volume(1)),
            extract_comic_volume("One Piece - T1")
        );
        assert_eq!(
            Some(VolumeType::Volume(1)),
            extract_comic_volume("One Piece - Vol. 1")
        );
        assert_eq!(
            Some(VolumeType::Volume(1)),
            extract_comic_volume("One Piece - Volume 1")
        );

        // Prologue Tests
        assert_eq!(
            Some(VolumeType::Prologue(1)),
            extract_comic_volume("One Piece - Prologue 1")
        );

        // Unknown Tests
        assert_eq!(
            Some(VolumeType::Unknown(1)),
            extract_comic_volume("One Piece - Item 1")
        );
        assert_eq!(
            Some(VolumeType::Unknown(1)),
            extract_comic_volume(
                "One Piece - Item 1 (Hinata) (2011) [Digital-1598] [Manga FR] (PapriKa)"
            )
        );

        // Invalid Multiple Volumes
        assert_eq!(None, extract_comic_volume("One Piece - T2-3"));
    }
}
