pub fn native_touppercase(args: Vec<Value>) -> Result<Value, RuntimeError> {
    // Optimization: avoid string allocation if string is already uppercase
    unary_text_op_arc("touppercase", args, |text| {
        // fast path: check if it changes when converted to uppercase
        let is_uppercase = text.chars().all(|c| {
            let mut iter = c.to_uppercase();
            iter.next() == Some(c) && iter.next().is_none()
        });
        if is_uppercase {
            text
        } else {
            Arc::from(text.to_uppercase())
        }
    })
}

pub fn native_tolowercase(args: Vec<Value>) -> Result<Value, RuntimeError> {
    // Optimization: avoid string allocation if string is already lowercase
    unary_text_op_arc("tolowercase", args, |text| {
        // fast path: check if it changes when converted to lowercase
        let is_lowercase = text.chars().all(|c| {
            let mut iter = c.to_lowercase();
            iter.next() == Some(c) && iter.next().is_none()
        });
        if is_lowercase {
            text
        } else {
            Arc::from(text.to_lowercase())
        }
    })
}
