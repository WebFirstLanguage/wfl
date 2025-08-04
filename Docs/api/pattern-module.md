## Pattern Module (Text Pattern Matching)

**⚠️ DEPRECATION NOTICE**: This document describes the legacy regex-based pattern system which is now deprecated. Please use the new `create pattern` block syntax documented in [`docs/patterns.md`](../docs/patterns.md). The legacy pattern system will be removed in a future version of WFL.

**For new projects, use the modern pattern syntax:**
```wfl
create pattern my_pattern:
    // Natural language pattern description
end pattern
```

---

## Legacy Pattern Module Documentation (Deprecated)

The **pattern module** provides a natural language approach to text pattern matching, allowing users to work with patterns in a more intuitive way than traditional regular expressions.This module implements the "Not Your Father's Regex" concept, making pattern matching accessible to beginners while still being powerful enough for complex text processing tasks.

Pattern operations use plain English expressions instead of cryptic symbols, making them easier to read, write, and understand. The module includes functions for matching, finding, replacing, and splitting text using patterns.

### Pattern Literals

Pattern literals are defined using the `pattern` keyword followed by a string that describes the pattern in natural language:

```wfl
store email pattern as pattern "{one or more letters or digits}@{one or more letters or digits}.{2 or 3 letters}"
```

Pattern literals can include:
- Basic patterns: `digit(s)`, `letter(s)`, `whitespace`
- Quantifiers: `optional X`, `one or more X`, `exactly N X`, `between N and M X`
- Placeholders: `{name}` for capturing parts of the text
- Alternation: `X or Y` for matching either X or Y
- Anchors: `begins with X`, `ends with X`

### Pattern Matching

The `matches pattern` operation checks if a text string matches a given pattern:

```wfl
store phone number as "555-123-4567"
store phone pattern as pattern "{3 digits}-{3 digits}-{4 digits}"

check if phone number matches pattern phone pattern:
    display "Valid phone number format!"
otherwise:
    display "Invalid phone number format!"
end check
```

The `contains pattern` operation is similar but checks if the pattern appears anywhere in the text.

### Finding Patterns

The `find pattern` operation extracts parts of text that match placeholders in the pattern:

```wfl
store date as "12/25/2023"
store date parts as find pattern "{month}/{day}/{year}" in date

display "Month: " with date parts["month"]
display "Day: " with date parts["day"]
display "Year: " with date parts["year"]
```

This returns a map where keys are the placeholder names and values are the matching text segments.

### Replacing Patterns

The `replace pattern` operation replaces text matching a pattern with a replacement string:

```wfl
store credit card as "Credit card: 1234-5678-9012-3456"
store censored as replace pattern "{4 digits}-{4 digits}-{4 digits}-{4 digits}" with "XXXX-XXXX-XXXX-****" in credit card

display censored // Outputs: "Credit card: XXXX-XXXX-XXXX-****"
```

### Splitting Text

The `split by pattern` operation divides text into segments using a pattern as the delimiter:

```wfl
store csv line as "Smith,John,42,Engineer"
store values as split csv line by pattern ","

display "Name: " with values[1] with " " with values[0]
display "Age: " with values[2]
display "Job: " with values[3]
```

### Implementation Details

- *Behavior:* Pattern operations follow deterministic, left-to-right, first-match semantics. Quantifiers default to non-greedy unless explicitly asked for "greedy".
- *Implementation:* Patterns are parsed into an internal representation that can efficiently match text. Error messages are clear and actionable, explaining what went wrong and how to fix it.
- *Types:* 
  - `matches pattern` returns a Boolean
  - `find pattern` returns a Map<String, Text> or Nothing if no match
  - `replace pattern` returns Text
  - `split by pattern` returns a List<Text>

The pattern module makes text processing more accessible by using natural language constructs instead of cryptic regular expression syntax, aligning with WFL's goal of being beginner-friendly while still powerful.
