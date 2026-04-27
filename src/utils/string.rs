pub fn is_snake_case(s: &str) -> bool {
    // Fast path for ASCII strings
    if s.is_ascii() {
        !s.bytes().any(|b| b.is_ascii_uppercase() || b == b' ')
    } else {
        !s.contains(char::is_uppercase) && !s.contains(' ')
    }
}

pub fn to_snake_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    let mut previous_char_is_lowercase = false;

    for (i, c) in s.char_indices() {
        if c.is_uppercase() {
            if i > 0 && previous_char_is_lowercase {
                result.push('_');
            }
            result.extend(c.to_lowercase());
        } else if c == ' ' {
            result.push('_');
        } else {
            result.push(c);
        }

        previous_char_is_lowercase = c.is_lowercase();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_snake_case() {
        assert!(is_snake_case("snake_case"));
        assert!(is_snake_case("simple"));
        assert!(!is_snake_case("camelCase"));
        assert!(!is_snake_case("PascalCase"));
        assert!(!is_snake_case("with space"));
        assert!(!is_snake_case("Mixed_Style"));
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("camelCase"), "camel_case");
        assert_eq!(to_snake_case("PascalCase"), "pascal_case");
        assert_eq!(to_snake_case("snake_case"), "snake_case");
        assert_eq!(to_snake_case("with space"), "with_space");
        assert_eq!(to_snake_case("Mixed_Style"), "mixed_style");
    }
}
