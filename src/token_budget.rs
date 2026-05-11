pub fn estimate_tokens(text: &str) -> usize {
    let chars = text.chars().count();
    let words = text.split_whitespace().count();
    chars.saturating_add(words) / 4 + 1
}

pub fn fits(text: &str, budget: usize) -> bool {
    estimate_tokens(text) <= budget
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_estimate_is_nonzero_for_text() {
        assert!(estimate_tokens("hello world") > 0);
    }
}
