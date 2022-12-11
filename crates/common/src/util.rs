pub const FILE_SIZE_IDENTIFIERS: [&str; 4] = ["B", "KB", "MB", "GB"];

pub fn file_size_bytes_to_readable_string(value: i64) -> String {
    let mut size = value as f64;

    // 1024

    let mut index = 0;
    while size > 1024.0 && index != 3 {
        size /= 1024.0;
        index += 1;
    }

    if index + 1 == FILE_SIZE_IDENTIFIERS.len() {
        format!(
            "{}{}",
            (size * 100.0).floor() / 100.0,
            FILE_SIZE_IDENTIFIERS[index]
        )
    } else {
        format!("{}{}", size.floor(), FILE_SIZE_IDENTIFIERS[index])
    }
}

pub fn take_from_and_swap<V, P: Fn(&V) -> bool>(array: &mut Vec<V>, predicate: P) -> Vec<V> {
    let mut ret = Vec::new();

    for i in (0..array.len()).rev() {
        if predicate(&array[i]) {
            ret.push(array.swap_remove(i));
        }
    }

    ret.reverse();

    ret
}