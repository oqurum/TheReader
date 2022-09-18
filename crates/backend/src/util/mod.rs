pub mod config;
pub mod events;
pub mod image;

pub use self::image::store_image;



pub fn sort_by_similarity<V, I, F>(match_with: &str, input: I, func: F) -> Vec<(f64, V)>
    where
        I: IntoIterator<Item = V>,
        F: Fn(&V) -> Option<&str>,
{
    let mut items = Vec::new();

    for item in input.into_iter() {
        let score = match func(&item) {
            Some(v) => strsim::jaro_winkler(match_with, v),
            None => 0.0,
        };

        items.push((score, item));
    }

    items.sort_unstable_by(|(a, _), (b, _)| b.partial_cmp(a).unwrap());

    items
}