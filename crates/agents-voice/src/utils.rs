pub fn get_sentence_based_splitter(text: &str) -> Vec<String> {
    text.split_terminator(['.', '!', '?'])
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_text_into_sentences() {
        assert_eq!(
            get_sentence_based_splitter("One. Two!"),
            vec!["One".to_owned(), "Two".to_owned()]
        );
    }
}
