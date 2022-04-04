
/// Truncate string based off of char indices instead of bytes.
pub fn truncate_on_indices(s: &mut String, max_chars: usize) {
    if let Some((new_len, _)) = s.char_indices().nth(max_chars) {
        s.truncate(new_len);
    }
}