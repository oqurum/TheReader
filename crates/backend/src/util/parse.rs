use lazy_static::lazy_static;
use regex::Regex;


const WORD_DELIMITER: [char; 8] = [' ', '_', '-', '.', ',', '=', '\'', '|'];
const SPECIAL_CHARS: [char; 4] = ['&', ':', '\\', '/'];
const USELESS: [char; 3] = [' ', '_', '-'];

lazy_static! {
    // t01, t1, t01-t02, t1-t2
    static ref TOME_NUMBER: Regex = Regex::new(r"t#?\d+(?:-t?\d+)?").unwrap();
    // tome 01, tome 1, tome 01-02, tome 1-2
    static ref TOME_NUMBER_WITH_TEXT: Regex = Regex::new(r"tome #?\d+(?:-\d+)?").unwrap();

    // p01, p1, p01-p02, p1-p2
    static ref PART_NUMBER: Regex = Regex::new(r"p#?\d+(?:-p?\d+)?").unwrap();
    // part 01, part 1, part 01-02, part 1-2
    static ref PART_NUMBER_WITH_TEXT: Regex = Regex::new(r"part #?\d+(?:-\d+)?").unwrap();

    // vol1, vol01, vol1-2, vol01-02
    static ref VOLUME_NUMBER: Regex = Regex::new(r"vol#?\d+(?:-vol?\d+)?").unwrap();

    // cbz, cbr, cb7, cbt, cba
    static ref ARCHIVE_EXTENSION: Regex = Regex::new(r"(?:cbz|cbr|cb7|cbt|cba)(?:/[a-zA-Z0-9]+)?").unwrap();

    // 001-9999
    static ref MULTIPLE_CHAPTERS: Regex = Regex::new(r"#?\d+-\d+").unwrap();
}

pub fn extract_name(value: &str) {
    let mut value = value.to_ascii_lowercase();

    // Replace underscores with spaces.
    value = value.replace('_', " ");

    // Regex Replaces
    value = TOME_NUMBER.replace_all(&value, "").to_string();
    value = TOME_NUMBER_WITH_TEXT.replace_all(&value, "").to_string();
    value = PART_NUMBER.replace_all(&value, "").to_string();
    value = PART_NUMBER_WITH_TEXT.replace_all(&value, "").to_string();
    value = VOLUME_NUMBER.replace_all(&value, "").to_string();
    value = ARCHIVE_EXTENSION.replace_all(&value, "").to_string();

    // TODO: Will also remove a date. E.g. 2020-01-01. Only call on a folder name.
    // value = MULTIPLE_CHAPTERS.replace_all(&value, "").to_string();

    // TODO: Detect if we are dealing with a file path. I.e. if the string contains a slash. Work backwards.

    // Remove text in brackets.
    while let Some((l_index, r_index)) = value.find('[').and_then(|index| Some((index, value.chars().skip(index).position(|c| c == ']')?))) {
        value.drain(l_index..=l_index + r_index);
    }

    // Remove text in parentheses.
    while let Some((l_index, r_index)) = value.find('(').and_then(|index| Some((index, value.chars().skip(index).position(|c| c == ')')?))) {
        value.drain(l_index..=l_index + r_index);
    }

    // Remove unnecessary characters at the end of the string.
    if let Some(amount) = value.chars().rev().position(|v| !USELESS.contains(&v)) {
        value.drain(value.len() - amount..);
    }

    // Remove double spaces.
    value = value.replace("  ", " ");

    // Remove concurrent dashes.
    value = value.replace("- -", "-");

    println!("- {value:?}");
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_file_name() {
        extract_name("Sports-Illustrated-1954-08-16");
        extract_name("No. 159 January 3rd 1992");
        extract_name("One Piece - Tome 01");
        extract_name("One Piece T2-23");
        extract_name("Naruto - Tome #002 - [V1]");
        extract_name("001-100/013 - Golf");
        extract_name("JoJo's Bizarre Adventure - Part 01 - Phantom Blood T01 (Araki) [Digital-1920] [Manga FR]");
        extract_name("JoJo's Bizarre Adventure - Part 01 - Phantom Blood T02 (Araki) [Digital-1920] [Manga FR]");
        extract_name("JoJo's Bizarre Adventure - Part 02 - Battle Tendency T01 (Araki) [Digital-1920] [Manga FR]");
        extract_name("Dream Team T02 (Hinata) (2011) [Digital-1598] [Manga FR] (PapriKa)");
        extract_name("fairygirls_vol1");
        extract_name("fairytail_vol1");
        extract_name("Name Here (115 tomes) FR CBZ/002 - Name Here (Info 1994)");
    }
}