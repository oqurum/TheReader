/// Numeric sort, just re-labeled.
pub fn filename_sort<T: AsRef<str>>(arr: &mut [T]) {
    numeric_sort::sort(arr);
}


#[cfg(test)]
mod sorting_test {
    use super::*;

    #[test]
    fn test_sorting_1() {
        let sorted = vec![
            "01 - The Beginning",
            "02 - The Continuation",
            "03 - The Conclusion",
            "04 - The End",
            "05 - The Beginning",
            "06 - The Continuation",
            "07 - The Conclusion",
            "08 - The End",
            "09 - The Beginning",
            "10 - The Continuation",
            "11 - The Conclusion",
            "12 - The End",
        ];

        let mut re_sorted = sorted.clone();

        filename_sort(&mut re_sorted);

        assert_eq!(sorted, re_sorted);
    }

    #[test]
    fn test_sorting_2() {
        let sorted = vec![
            "1 - The Beginning",
            "2 - The Continuation",
            "3 - The Conclusion",
            "4 - The End",
            "05 - The Beginning",
            "6 - The Continuation",
            "7 - The Conclusion",
            "8 - The End",
            "9 - The Beginning",
            "10 - The Continuation",
            "011 - The Conclusion",
            "012 - The End",
            "20 - The Beginning",
            "21 - The Continuation",
            "30 - The Conclusion",
            "40 - The End",
            "50 - The Beginning",
            "51 - The Continuation",
            "60 - The Continuation",
            "70 - The Conclusion",
            "80 - The End",
            "90 - The Beginning",
            "100 - The Continuation",
            "110 - The Conclusion",
            "120 - The End",
            "130 - The Beginning",
        ];

        let mut re_sorted = sorted.clone();

        filename_sort(&mut re_sorted);

        assert_eq!(sorted, re_sorted);
    }
}