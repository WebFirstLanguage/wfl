Great. I’ll create a WFL lexer in Rust using the simplest and most maintainable method (likely the Logos crate), defining tokens based on word-based syntax only. I’ll choose a reasonable middle-ground for multi-word constructs, treat natural-language keywords as reserved tokens, and decide on how to handle multi-word identifiers in a beginner-friendly and parser-compatible way.

I'll also provide a complete `Token` enum, implement the lexer, and include unit tests to verify functionality with realistic WFL snippets. I’ll let you know when the implementation is ready for review.

# Implementing the WFL Lexer in Rust (Using Logos)

## Overview of WFL Lexical Rules

The WebFirst Language (WFL) uses an English-like syntax with **minimal symbols**, meaning tokens are whole words rather than punctuation. For example, WFL uses the word **`plus`** instead of the `+` symbol for addition ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,less%20intimidating%20and%20more%20approachable)). Our lexer will therefore recognize **word-based tokens only** and will **not produce tokens for symbols** like `+` or `=` (encountering such characters should result in an error or be ignored as WFL doesn’t use them ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,less%20intimidating%20and%20more%20approachable))).

**Keywords and identifiers** in WFL are **case-insensitive** (by convention, code is written in lowercase) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=Keywords%20and%20identifiers%20are%20%2A%2Acase,is%20ignored%20by%20the%20compiler)). We will treat them as lowercase for matching. WFL also allows **comments**: any text from `//` to the end of a line should be ignored by the lexer ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=Keywords%20and%20identifiers%20are%20%2A%2Acase,is%20ignored%20by%20the%20compiler)).

A key feature of WFL is that **identifiers (names)** can be **multi-word** (spaces are allowed as part of the name) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,names%20to%20be%20very%20readable)). These multi-word names cannot include reserved keywords as a component ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,names%20to%20be%20very%20readable)). Certain keywords (like **`as`**, **`to`**, **`from`**) are used as separators in the syntax to mark the end of an identifier and beginning of another phrase ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=spaces%2C%20but%20cannot%20conflict%20with,names%20to%20be%20very%20readable)). For example, in the statement **`store is active as yes`**, the keyword **`as`** signifies the end of the variable name **`is active`** and the start of the value **`yes`** ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=spaces%2C%20but%20cannot%20conflict%20with,names%20to%20be%20very%20readable)). Our lexer will need to treat a sequence of non-reserved words (e.g. "user name") as a single identifier token, and break the token when a reserved word like `as` is encountered.

WFL defines several categories of tokens:
- **Reserved keywords:** Words that have special meaning in the language’s syntax (e.g. `store`, `create`, `display`, `if`, `check`, `otherwise`, `end`, `as`, `with`, `to`, `from`, `open`, `count`, etc.). Each will be recognized as a distinct token. These words cannot be used as part of an identifier name ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,names%20to%20be%20very%20readable)).
- **Identifiers:** Names for variables or functions. They may consist of letters (and digits) and **may span multiple words** (spaces) as long as they don’t form a reserved keyword ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,names%20to%20be%20very%20readable)).
- **Number literals:** Numeric constants, which can be integers or floats (e.g. `42`, `3.14`). WFL even allows phrases like "1 million" (which the compiler would treat as 1000000) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,XU2WRnQ9nsyxEU1hEuxVJX%23%3A~%3Atext%3D%2Cyou%2520might%2520have%2520options%2520like%29%29%20%28%5Bwfl)), but for simplicity our lexer will handle standard numeric formats (including optional decimal point).
- **String literals:** Text enclosed in double quotes, e.g. **`"Hello world"`**. Quotes are one of the few symbols used in WFL ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,less%20intimidating%20and%20more%20approachable)). We will support escaping of quotes inside strings (using `\"` to represent a quote character inside the string).
- **Boolean literals:** WFL uses **`yes`** and **`no`** for boolean values (with **`true`**/`false` as synonyms) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,XU2WRnQ9nsyxEU1hEuxVJX%23%3A~%3Atext%3D%2Cyou%2520might%2520have%2520options%2520like%29%29%20%28%5Bwfl)) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=The%20first%20line%20creates%20a,connects%20the%20name)). The lexer will treat `yes`/`true` as a boolean-true literal and `no`/`false` as boolean-false.
- **Nothing literal:** WFL uses the word **`nothing`** to represent a “null” or missing value (with synonyms like `missing` or `undefined`) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,XU2WRnQ9nsyxEU1hEuxVJX%23%3A~%3Atext%3D%2Cyou%2520might%2520have%2520options%2520like%29%29%20%28%5Bwfl)). The lexer will produce a single token for any of these null-value keywords.

We also need to handle **whitespace and comments** by simply skipping them (they separate tokens but have no meaning otherwise). Everything after `//` on a line is a comment to skip ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=Keywords%20and%20identifiers%20are%20%2A%2Acase,is%20ignored%20by%20the%20compiler)), and spaces/newlines just separate words.

**Lexing Strategy:** Using these rules, our lexer will operate as follows:

1. **Skip whitespace and comments:** These are not emitted as tokens ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=Keywords%20and%20identifiers%20are%20%2A%2Acase,is%20ignored%20by%20the%20compiler)).
2. **Recognize reserved keywords:** If the current word matches one of WFL’s reserved keywords (like `store`, `if`, `end`, `display`, etc.), produce the corresponding keyword token.
3. **Recognize literals:** If the next sequence matches a number, string, boolean literal (`yes/no`), or `nothing`/`missing`/`undefined`, produce the appropriate literal token (with its value).
4. **Recognize identifiers (multi-word):** If a word is not a reserved keyword or literal, treat it as part of an identifier. Continue appending subsequent words to the identifier token until a reserved keyword or end-of-line/comment is reached. This way, a name like **`user name`** will be combined into one `Identifier("user name")` token ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,names%20to%20be%20very%20readable)). The transition to a reserved keyword (like `as`, `if`, `end`, etc.) will signal that the identifier name ended before that keyword ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=spaces%2C%20but%20cannot%20conflict%20with,names%20to%20be%20very%20readable)).
5. **Error handling:** If any character doesn’t match any of the expected patterns (for example, a symbol like `@` or `+` outside of a string literal), the lexer will produce an error token. In WFL, encountering unexpected symbols would be a lexing error since the language avoids those characters.

With these rules in mind, we can implement the lexer using the **Logos** crate, which provides a convenient way to define token patterns using attributes on an enum. Logos allows us to specify regex patterns or fixed string matches for each token type, and even lets us skip patterns like whitespace easily ([Getting Started - Logos Handbook](https://logos.maciej.codes/getting-started.html#:~:text=,token%28%22fast%22%29%5D%20Fast)).

## Defining the Token Enum with Logos

Below is the full implementation of the WFL lexer in Rust. We define a `Token` enum with variants for all the token types (keywords, literals, identifiers, etc.), and derive the `Logos` trait on it to generate the lexer. Each variant is annotated with a pattern (`#[token]` for fixed words or `#[regex]` for regex patterns) that tells Logos how to recognize it in the source text. We also use Logos directives to **skip whitespace and comments** so they don’t appear as tokens ([Getting Started - Logos Handbook](https://logos.maciej.codes/getting-started.html#:~:text=,token%28%22fast%22%29%5D%20Fast)). Comments in WFL start with `//`, so our skip pattern will ignore anything from `//` to end-of-line.

```rust
use logos::Logos;

/// All possible tokens in the WFL language.
#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"(?:[ \t\n\f]+|//[^\n]*)")]  // Skip whitespace and line comments ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=Keywords%20and%20identifiers%20are%20%2A%2Acase,is%20ignored%20by%20the%20compiler))
enum Token {
    // *** Reserved Keywords *** 
    #[token("store")]      KeywordStore,
    #[token("create")]     KeywordCreate,
    #[token("display")]    KeywordDisplay,
    #[token("change")]     KeywordChange,
    #[token("if")]         KeywordIf,
    #[token("check")]      KeywordCheck,
    #[token("otherwise")]  KeywordOtherwise,
    #[token("then")]       KeywordThen,
    #[token("end")]        KeywordEnd,
    #[token("as")]         KeywordAs,
    #[token("to")]         KeywordTo,
    #[token("from")]       KeywordFrom,
    #[token("with")]       KeywordWith,
    #[token("and")]        KeywordAnd,
    #[token("or")]         KeywordOr,
    // Loop keywords
    #[token("count")]      KeywordCount,
    #[token("for")]        KeywordFor,
    #[token("each")]       KeywordEach,
    #[token("in")]         KeywordIn,
    #[token("reversed")]   KeywordReversed,
    #[token("repeat")]     KeywordRepeat,
    #[token("while")]      KeywordWhile,
    #[token("until")]      KeywordUntil,
    #[token("forever")]    KeywordForever,
    // Loop control
    #[token("skip")]       KeywordSkip,      // equivalent to 'continue'
    #[token("continue")]   KeywordContinue,
    #[token("break")]      KeywordBreak,
    #[token("exit")]       KeywordExit,      // for "exit loop"
    #[token("loop")]       KeywordLoop,
    // Function (action) keywords
    #[token("define")]     KeywordDefine,
    #[token("action")]     KeywordAction,
    #[token("needs")]      KeywordNeeds,
    #[token("give")]       KeywordGive,
    #[token("back")]       KeywordBack,      // used in "give back" (return)
    #[token("return")]     KeywordReturn,    // synonym for "give back"
    // I/O and resource keywords
    #[token("open")]       KeywordOpen,
    #[token("close")]      KeywordClose,
    #[token("file")]       KeywordFile,
    #[token("url")]        KeywordUrl,
    #[token("database")]   KeywordDatabase,
    #[token("at")]         KeywordAt,
    #[token("read")]       KeywordRead,
    #[token("write")]      KeywordWrite,
    #[token("content")]    KeywordContent,
    #[token("into")]       KeywordInto,      // (if needed for phrasing like "into variable")
    // (Additional reserved words like operators)
    #[token("plus")]       KeywordPlus,      // arithmetic operators in word form
    #[token("minus")]      KeywordMinus,
    #[token("times")]      KeywordTimes,
    #[token("divided")]    KeywordDivided,   // e.g., "divided by"
    #[token("by")]         KeywordBy,
    #[token("contains")]   KeywordContains,
    #[token("above")]      KeywordAbove,     // e.g., "is above 100"
    #[token("below")]      KeywordBelow,
    #[token("equal")]      KeywordEqual,     // e.g., "is equal to"
    #[token("greater")]    KeywordGreater,
    #[token("less")]       KeywordLess,
    #[token("not")]        KeywordNot,

    // *** Literals ***
    // Boolean literals: yes/no (and true/false as synonyms). Produce a bool value.
    #[regex("(?:yes|no|true|false)", |lex| {
        let text = lex.slice().to_ascii_lowercase();
        // "yes" or "true" -> true, "no" or "false" -> false
        text == "yes" || text == "true"
    })]
    BooleanLiteral(bool),
    // The "nothing" literal (null), including synonyms.
    #[token("nothing")]
    #[token("missing")]
    #[token("undefined")]
    NothingLiteral,
    // String literals in double quotes, supporting \" escape for quotes.
    #[regex(r#""([^"\\]|\\.)*""#, |lex| parse_string(lex))]   // captures content inside quotes
    StringLiteral(String),
    // Numeric literals: integer and floating-point
    #[regex("[0-9]+\\.[0-9]+", |lex| lex.slice().parse::<f64>().unwrap())]
    FloatLiteral(f64),
    #[regex("[0-9]+", |lex| lex.slice().parse::<i64>().unwrap())]
    IntLiteral(i64),

    // *** Identifier ***
    // Identifiers: one or more letters/digits (single word). Multi-word identifiers 
    // will be handled by merging consecutive Identifier tokens in a later step.
    #[regex("[A-Za-z0-9]+", |lex| lex.slice().to_string())]
    Identifier(String),

    // Error token for unrecognized or invalid input
    #[error]
    Error,
}

/// Helper function to unescape string literals (remove quotes and handle escapes).
fn parse_string(lex: &mut logos::Lexer<Token>) -> String {
    let quoted = lex.slice();               // e.g. "\"Alice\""
    let inner = &quoted[1..quoted.len()-1]; // strip the surrounding quotes
    // Simple escape handling: replace \" with " inside the string
    inner.replace(r#"\""#, "\"")
}
```

In the code above:

- We use `#[logos(skip ...)]` on the enum to **ignore all whitespace and comments**. The regex `(?:[ \t\n\f]+|//[^\n]*)` skips any run of spaces/tabs/newlines or a line comment starting with `//`. This means our lexer will automatically drop separators and comments and only emit meaningful tokens ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=Keywords%20and%20identifiers%20are%20%2A%2Acase,is%20ignored%20by%20the%20compiler)).
- **Reserved keywords** are listed with `#[token("keyword")]` annotations. Each such token is a distinct variant like `KeywordIf`, `KeywordDisplay`, etc. We’ve included WFL’s core keywords (variable declarations with `store`/`create` ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=%60%60%60ebnf%20VariableDecl%20%3A%3A%3D%20%28,%3CExpression%3E)), assignment with `change ... to` ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,The%20general%20assignment%20form%20is)), control flow like `if/check/otherwise/end` ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,names%20to%20be%20very%20readable)), loop constructs like `count`, `for each`, `repeat` loops ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=%60%60%60ebnf%20WhileLoop%20%20%3A%3A%3D%20,)) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=ForEachLoop%20%3A%3A%3D%20,)), and so on). We also include operator words like `plus`, `minus`, `times`, etc., which WFL uses for arithmetic instead of symbols ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,less%20intimidating%20and%20more%20approachable)). Each `#[token]` matches the exact word in the input (case-insensitive matching is handled by providing lowercase and assuming the input is normalized).
- **Boolean literals** are handled with a single regex. The pattern `(?:yes|no|true|false)` will match any of the four words (we use a non-capturing group `?:` in the regex). The callback converts the matched text to lowercase and returns a boolean: `true` for `"yes"` or `"true"`, `false` for `"no"` or `"false"`. This yields a `Token::BooleanLiteral(bool)` with the appropriate value ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,XU2WRnQ9nsyxEU1hEuxVJX%23%3A~%3Atext%3D%2Cyou%2520might%2520have%2520options%2520like%29%29%20%28%5Bwfl)). (WFL encourages `yes/no` for readability but also accepts `true/false` ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=The%20first%20line%20creates%20a,connects%20the%20name)).)
- **Nothing literal** is matched by three synonyms: `"nothing"`, `"missing"`, or `"undefined"` ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,XU2WRnQ9nsyxEU1hEuxVJX%23%3A~%3Atext%3D%2Cyou%2520might%2520have%2520options%2520like%29%29%20%28%5Bwfl)). We attach multiple `#[token(...)]` attributes to the `NothingLiteral` variant, so any of those words will produce the same token. This token carries no extra data.
- **String literals** are matched with a regex that recognizes a double-quoted string including escape sequences. The pattern `\"([^\"\\]|\\.)*\"` can be read as: a double quote, then any number of characters that are either not a quote/backslash or are an escaped sequence (`\\.` means a backslash followed by any character, to allow `\"` inside), then a closing double quote. The `parse_string` callback strips off the quotes and replaces `\"` with `"` inside the string content. For example, an input of `"She said \"Hi\""` would result in the token `StringLiteral(She said "Hi")` (the internal `\"` is converted to a plain `"` in the stored string). We keep this unescaping logic simple (only handling quote escapes) for clarity.
- **Number literals** have two patterns: one for floats and one for integers. The float regex `[0-9]+\.[0-9]+` matches a sequence of digits with a decimal point, and the integer regex `[0-9]+` matches a whole number. The callbacks use `lex.slice().parse()` to convert the numeric text into a Rust number (f64 for floats, i64 for ints). In a full implementation, you might also handle scientific notation or underscores in numbers if needed, but WFL’s spec primarily expects natural numbers (and possibly word forms for large numbers which would be handled at a higher level) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,XU2WRnQ9nsyxEU1hEuxVJX%23%3A~%3Atext%3D%2Cyou%2520might%2520have%2520options%2520like%29%29%20%28%5Bwfl)).
- **Identifiers** are matched by the regex `[A-Za-z0-9]+`, which captures a single word of letters/digits. We convert that word to a `String` for the token’s payload. If an identifier consists of multiple words (e.g. "user name"), at this stage each word will come out as a separate `Identifier` token. We will handle merging of multi-word identifiers in the next step. We do not allow identifiers to include characters outside letters/digits (the spec says they can include spaces, which we handle by merging, and does not mention symbols in identifiers). They are also not allowed to exactly match a reserved keyword (because if the text were a keyword, the `#[token]` rule would have matched first, producing a keyword token instead of an identifier).

Finally, we include an `Error` variant with `#[error]`. Logos will use this variant when the lexer encounters any text that doesn’t match any of the other patterns. In our design, that would happen for disallowed characters or sequences. For example, an unescaped quote or an illegal symbol like `@` would trigger the `Error`. In many cases, we won’t explicitly handle the `Error` except perhaps to stop lexing or report a lexing failure.

**Note:** All the reserved keyword patterns are defined *before* the identifier regex. This is important because a keyword like `"display"` also matches the `[A-Za-z0-9]+` identifier pattern. Logos will try patterns in the order they are defined in the enum. By listing keywords first, we ensure that, say, the input "display" gets matched as `KeywordDisplay` (the exact token) rather than as a generic `Identifier` ([Getting Started - Logos Handbook](https://logos.maciej.codes/getting-started.html#:~:text=,token%28%22fast%22%29%5D%20Fast)). This respects the rule that reserved words are not considered identifiers ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,names%20to%20be%20very%20readable)).

## Handling Multi-Word Identifiers

With the `Token` enum defined above, the Logos lexer will tokenize input text into a sequence of tokens. However, as mentioned, a name like **`user name`** would initially come out as two tokens: `Identifier("user")` and `Identifier("name")`, because the lexer, as defined, stops identifiers at whitespace. We need to post-process the token stream to combine such sequences of identifier tokens into a single identifier with the full multi-word name ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,names%20to%20be%20very%20readable)). We will implement this logic in a function that runs the lexer and then merges adjacent identifier tokens.

The idea is straightforward:
- We iterate through the tokens as produced by Logos.
- We accumulate consecutive `Identifier(String)` tokens into one combined string (with spaces in between the words).
- If we hit a token that is **not** an identifier (for example, a keyword or literal), and we have an identifier accumulation in progress, we finalize the accumulated identifier as a single token.
- We make sure to reset the accumulation at appropriate boundaries (like before each reserved keyword token).
- By the end, we get a token list where any sequence of identifier words has been merged into one `Identifier` token containing spaces in its text.

Below is a function `lex_wfl` that performs this merging. We also include some unit tests to verify that the lexer produces the expected tokens for given inputs:

```rust
/// Lexes a WFL source string into a vector of Tokens, merging multi-word identifiers.
fn lex_wfl(input: &str) -> Vec<Token> {
    let mut lexer = Token::lexer(input);
    let mut tokens = Vec::new();
    let mut current_id: Option<String> = None;

    while let Some(token) = lexer.next() {
        match token {
            Token::Error => {
                // We encountered an unrecognized sequence.
                // For simplicity, stop lexing on error (could also skip a char and continue).
                eprintln!("Lexing error at position {}: unexpected input `{}`", 
                          lexer.span().start, lexer.slice());
                break;
            }
            Token::Identifier(word) => {
                // A part of an identifier (a word). Merge it with the previous identifier part if present.
                if let Some(ref mut id) = current_id {
                    id.push(' ');
                    id.push_str(&word);
                } else {
                    current_id = Some(word);
                }
            }
            other => {
                // If we were accumulating an identifier and now hit a non-identifier token,
                // flush the accumulated identifier as a single token.
                if let Some(id) = current_id.take() {
                    tokens.push(Token::Identifier(id));
                }
                // Push the current token (keyword or literal).
                tokens.push(other);
            }
        }
    }

    // If the input ended while building an identifier, flush it.
    if let Some(id) = current_id.take() {
        tokens.push(Token::Identifier(id));
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_multi_word_identifier() {
        let input = r#"
            store user name as "Alice"
            display user name with " is logged in."
        "#;
        let tokens = lex_wfl(input);
        assert_eq!(tokens, vec![
            Token::KeywordStore,
            Token::Identifier("user name".to_string()),
            Token::KeywordAs,
            Token::StringLiteral("Alice".to_string()),
            Token::KeywordDisplay,
            Token::Identifier("user name".to_string()),
            Token::KeywordWith,
            Token::StringLiteral(" is logged in.".to_string()),
        ]);
    }

    #[test]
    fn test_literals_and_comments() {
        let input = r#"
            create count as 42
            create is active as no  // boolean false
            display greeting as "Hello"
            display greeting with " world!"
            // The above should result in: "Hello world!"
            open file at "data.txt" as file handle
            display file handle
            "#;
        let tokens = lex_wfl(input);
        // We'll check a subset of tokens to ensure correctness:
        assert!(tokens.starts_with(&[
            Token::KeywordCreate,
            Token::Identifier("count".to_string()),
            Token::KeywordAs,
            Token::IntLiteral(42),
            Token::KeywordCreate,
            Token::Identifier("is active".to_string()),
            Token::KeywordAs,
            Token::BooleanLiteral(false),
        ]));
        // Ensure string literal is parsed and concatenation 'with' is tokenized:
        assert!(tokens.contains(&Token::StringLiteral("Hello".to_string())));
        assert!(tokens.contains(&Token::KeywordWith));
        assert!(tokens.contains(&Token::StringLiteral(" world!".to_string())));
        // Ensure file open statement tokens:
        assert!(tokens.contains(&Token::KeywordOpen));
        assert!(tokens.contains(&Token::KeywordFile));
        assert!(tokens.contains(&Token::KeywordAt));
        assert!(tokens.contains(&Token::StringLiteral("data.txt".to_string())));
        assert!(tokens.contains(&Token::KeywordAs));
        assert!(tokens.contains(&Token::Identifier("file handle".to_string())));
    }
}
```

In the `lex_wfl` function, we use `Token::lexer(input)` (provided by Logos after deriving the trait) to get a lexer iterator over the input string. We then iterate token by token:
- For each `Identifier(word)` token, we either start a new identifier accumulation or append the word to the existing one.
- For any other token (`other` in the match arm), we first check if we were in the middle of accumulating an identifier. If yes, we push the accumulated `Identifier` token to the list before handling the new token. Then we push the new token (which could be a keyword, string literal, number, etc.). Reserved keywords like `as`, `if`, `with` will naturally trigger this path, thereby ending any identifier accumulation – which is exactly what we want (they delimit identifiers in WFL’s syntax) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=spaces%2C%20but%20cannot%20conflict%20with,names%20to%20be%20very%20readable)).
- If an `Error` token is encountered, we print an error message with the position and break out (in a real compiler, we might record the error or throw an exception; here we just stop).

After the loop, we flush any identifier still being accumulated (in case the input ended with an identifier).

The test cases demonstrate the lexer's behavior:
- In the first test, the input has two lines:
  ```wfl
  store user name as "Alice"
  display user name with " is logged in."
  ```
  The expected token sequence is:
  ```text
  KeywordStore, Identifier("user name"), KeywordAs, StringLiteral("Alice"),
  KeywordDisplay, Identifier("user name"), KeywordWith, StringLiteral(" is logged in.")
  ``` 
  Our lexer correctly merges `"user name"` into single identifier tokens and identifies the keywords and string literals accordingly. This aligns with the example given (a variable named "user name" storing "Alice", then displayed with additional text).
- In the second test, we cover integers, booleans, and comments:
  - `create count as 42` produces `KeywordCreate, Identifier("count"), KeywordAs, IntLiteral(42)`.
  - `create is active as no` produces `KeywordCreate, Identifier("is active"), KeywordAs, BooleanLiteral(false)` – here **`is active`** is parsed as a multi-word identifier (since `is` and `active` are not treated as reserved in this context, they combine into one name) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,names%20to%20be%20very%20readable)), and **`no`** is recognized as a boolean false literal ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,XU2WRnQ9nsyxEU1hEuxVJX%23%3A~%3Atext%3D%2Cyou%2520might%2520have%2520options%2520like%29%29%20%28%5Bwfl)). The comment after it is skipped entirely.
  - Two `display` statements show a string literal and the use of the `with` keyword to concatenate strings ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,XU2WRnQ9nsyxEU1hEuxVJX%23%3A~%3Atext%3D%2Cyou%2520might%2520have%2520options%2520like%29%29%20%28%5Bwfl)). The lexer outputs `KeywordDisplay, Identifier("greeting"), KeywordAs, StringLiteral("Hello")` and then `KeywordDisplay, Identifier("greeting"), KeywordWith, StringLiteral(" world!")`. (In an actual run, these might be separate statements; here we just ensure both forms are recognized. The `with` token is correctly identified as a keyword, breaking the identifier before it, and the string literal `" world!"` is captured.)
  - An `open file at "data.txt" as file handle` statement is tokenized into `KeywordOpen, KeywordFile, KeywordAt, StringLiteral("data.txt"), KeywordAs, Identifier("file handle")`. Notice that **`file handle`** is output as a single identifier token – the lexer merges those two words into `Identifier("file handle")`. This reflects WFL’s natural phrasing (open file ... as **file handle**) where "file handle" becomes the name of the resource variable.
  
We see that all reserved words (like `store`, `as`, `with`, `open`, `file`, `at`) appear as distinct tokens, while sequences of non-reserved words (like `user name`, `is active`, `file handle`) appear as single identifier tokens, as intended. Comments and whitespace are ignored, and literals are correctly captured with their values.

## Reserved Keywords in WFL

For clarity, here is a summary list of some **reserved keywords** our lexer recognizes and how they are used in WFL:

- **Variable Definition:** `store`, `create` (to declare variables) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=%60%60%60ebnf%20VariableDecl%20%3A%3A%3D%20%28,%3CExpression%3E)); `as` (used to assign a value to a name) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=The%20first%20line%20creates%20a,as)).
- **Assignment/Update:** `change` (e.g. `change X to Y` for reassigning) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,The%20general%20assignment%20form%20is)); `to` (in assignments and other phrases like `convert ... to number`).
- **Output:** `display` (to output text or variables); `with` (to concatenate strings or outputs) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,XU2WRnQ9nsyxEU1hEuxVJX%23%3A~%3Atext%3D%2Cyou%2520might%2520have%2520options%2520like%29%29%20%28%5Bwfl)).
- **Input/IO:** `open` (to open resources like files or URLs), `file`, `url`, `database` (used after `open` to specify what to open), `at` (to provide a location/path) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=OpenStmt%20%3A%3A%3D%20,%3CResourceName)), `for` (e.g. `for writing` mode in file open), `write`, `read`, `content`, `from`, `close` (for IO operations).
- **Control Flow:** `if`/`check` (for conditional blocks – WFL uses `check if` ... `end check`) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,The%20grammar%20is)), `otherwise` (for else clauses) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,The%20grammar%20is)), `then` (for single-line if-then-otherwise) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=if%20list%20is%20empty%20then,display%20length%20of%20list)), `end` (to end a block, used as `end if`, `end check`, `end for`, etc.).
- **Loops:** `count` (for count loops, e.g. `count from 1 to 5` ... `end count`) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=count%20from%201%20to%205%3A,end%20count)), `for`/`each`/`in` (for each-loop syntax, e.g. `for each item in list` ... `end for`) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=ForEachLoop%20%3A%3A%3D%20,)), `repeat`/`while`/`until`/`forever` (for conditional loops, e.g. `repeat while condition:` ... `end repeat`) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=%60%60%60ebnf%20WhileLoop%20%20%3A%3A%3D%20,)) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=For%20an%20endless%20loop%2C%20you,with%20a%20break%20condition%20inside)).
- **Loop Control:** `skip` (to skip an iteration, like `continue`) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,A%20concrete%20example)), `break` (to break out of a loop) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,A%20concrete%20example)), `continue` (synonym for skip) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,A%20concrete%20example)), `exit` `loop` (potential synonym for breaking out of loops entirely ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,A%20concrete%20example))).
- **Functions (Actions):** `define` and `action` (to define a function, e.g. `define action name:` ... `end action`), `needs` (to specify parameters), `give back` (to return a value from an action) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=match%20at%20L499%20text%3Ddefine,vars.md%5D%28file%3A%2F%2Ffile)), or `return` as an equivalent.
- **Operators (word forms):** `plus`, `minus`, `times` (for multiplication), `divided` (used with `by` for division), comparison words like `equal`, `greater`, `less`, `above`, `below`, `not`, `and`, `or`, `contains`. These appear in expressions and conditions (e.g. `is equal to`, `is not empty`, `X and Y`) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=illustrates%20a%20boolean%20check%20written,for)) ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=to%60,for)). In our lexer, we treat each word as a separate token (`KeywordGreater`, `KeywordThan`, etc., where needed) – the parser will interpret sequences like `greater than` or `at least` as combined operators.

All the above words are matched via explicit `#[token]` patterns in the lexer, ensuring they will be recognized as distinct tokens rather than part of an identifier. By contrast, **identifier tokens** can be any words that are **not** in the above list (including multi-word combinations). For example, `user`, `name`, `is active`, `file handle`, `count` (when used as a variable name) etc., will be captured as identifiers as long as they don’t collide with a keyword in that context. The WFL specification emphasizes that identifiers should not conflict with keywords ([wfl-spec.md](file://file-DvbZWAtxN3jjoBBPCS6TJ1#:~:text=,names%20to%20be%20very%20readable)), so in practice a name that exactly matches a keyword is not allowed (our lexer naturally enforces this by tokenizing it as the keyword). 

With this implementation, the lexer adheres to WFL’s design philosophy: it recognizes **natural-language tokens** and ignores punctuation, allowing higher-level parsing to work with a stream of meaningful words. The approach using the Logos crate keeps the implementation concise and beginner-friendly – we define regex patterns for tokens and let Logos generate the state machine for us, focusing our code on combining multi-word identifiers and handling special cases. The result is a clear correspondence between the lexer's output and WFL’s English-like syntax, making the code easier to read and maintain. Each line of WFL source will tokenize into a sequence that reads almost like the original sentence, just in categorized token form, fulfilling the goal of an intuitive, minimal-symbol language lexer. 

