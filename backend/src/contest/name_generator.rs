use random_word::gen;

/// Capitalizes the first letter of a word
fn capitalize(word: &str) -> String {
    let mut c = word.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

/// Generates a random contest name using an adjective and noun
pub fn generate_contest_name() -> String {
    let adjective = capitalize(&gen());
    let noun = capitalize(&gen());
    format!("{} {}", adjective, noun)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_contest_name() {
        let name = generate_contest_name();
        assert!(!name.is_empty());
        assert!(name.contains(' '));
        // Check that both words are capitalized
        let words: Vec<&str> = name.split(' ').collect();
        assert_eq!(words.len(), 2);
        for word in words {
            assert!(word.chars().next().unwrap().is_uppercase());
        }
    }

    #[test]
    fn test_generate_contest_name_multiple_calls() {
        let name1 = generate_contest_name();
        let name2 = generate_contest_name();
        let name3 = generate_contest_name();
        // Names should be different (though there's a small chance they could be the same)
        assert!(name1 != name2 || name2 != name3 || name1 != name3);
    }
}
