use crate::shared::errors::ApiError;

pub fn is_valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

pub fn validate_slug(slug: &str) -> Result<(), ApiError> {
    if is_valid_slug(slug) {
        return Ok(());
    }

    Err(ApiError::unprocessable_entity(
        "Invalid slug: only lowercase letters, numbers, hyphens, and underscores are allowed",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_valid_slug_accepts_lowercase_alphanumeric_hyphen_and_underscore() {
        for slug in ["abc", "abc-123", "abc_123", "a1-b2_c3"] {
            assert!(is_valid_slug(slug), "{slug} should be valid");
        }
    }

    #[test]
    fn is_valid_slug_rejects_empty_or_disallowed_characters() {
        for slug in ["", "ABC", "abc def", "abc.def", "abc/def", "cafeé"] {
            assert!(!is_valid_slug(slug), "{slug} should be invalid");
        }
    }
}
