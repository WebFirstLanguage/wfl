pub fn is_snake_case(s: &str) -> bool {
    if s.is_ascii() {
        let bytes = s.as_bytes();
        !bytes.iter().any(|&b| b.is_ascii_uppercase() || b == b' ')
    } else {
        !s.contains(char::is_uppercase) && !s.contains(' ')
    }
}

pub fn to_snake_case(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }

    if s.is_ascii() {
        let bytes = s.as_bytes();
        // Calculate max possible length (every char is uppercase + 1 for underscore)
        let mut result = String::with_capacity(bytes.len() * 2);
        let mut previous_char_is_lowercase = false;

        for (i, &b) in bytes.iter().enumerate() {
            if b.is_ascii_uppercase() {
                if i > 0 && previous_char_is_lowercase {
                    result.push('_');
                }
                result.push(b.to_ascii_lowercase() as char);
                previous_char_is_lowercase = false;
            } else if b == b' ' {
                result.push('_');
                previous_char_is_lowercase = false; // spaces aren't lowercase
            } else {
                result.push(b as char);
                previous_char_is_lowercase = b.is_ascii_lowercase();
            }
        }
        return result;
    }

    // Fallback to slow unicode path
    let mut result = String::with_capacity(s.len() + s.len() / 4);
    let mut previous_char_is_lowercase = false;

    for (i, c) in s.char_indices() {
        if c.is_uppercase() {
            if i > 0 && previous_char_is_lowercase {
                result.push('_');
            }
            result.extend(c.to_lowercase());
            previous_char_is_lowercase = false;
        } else if c == ' ' {
            result.push('_');
            previous_char_is_lowercase = false;
        } else {
            result.push(c);
            previous_char_is_lowercase = c.is_lowercase();
        }
    }

    result
}
