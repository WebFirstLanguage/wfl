# WFL Unicode Pattern Support

This document provides comprehensive guidance on using Unicode features in WFL pattern matching system.

## Overview

WFL's pattern matching system provides full Unicode support, allowing you to match characters from any writing system or character category. The system works with Unicode code points and character indices, ensuring proper handling of international text.

## Unicode Categories

Unicode organizes characters into general categories. WFL supports all major Unicode categories:

### Letter Categories
```wfl
create pattern letters:
    unicode category "Letter"
end pattern

create pattern uppercase:
    unicode category "Uppercase_Letter"
end pattern

create pattern lowercase:
    unicode category "Lowercase_Letter"
end pattern
```

**Supported Letter Categories:**
- `Letter` (L) - All letters
- `Uppercase_Letter` (Lu) - Uppercase letters
- `Lowercase_Letter` (Ll) - Lowercase letters  
- `Titlecase_Letter` (Lt) - Titlecase letters
- `Modifier_Letter` (Lm) - Modifier letters
- `Other_Letter` (Lo) - Other letters

### Number Categories
```wfl
create pattern numbers:
    unicode category "Number"
end pattern

create pattern decimal_digits:
    unicode category "Decimal_Number"
end pattern
```

**Supported Number Categories:**
- `Number` (N) - All numbers
- `Decimal_Number` (Nd) - Decimal digits (0-9, ٠-٩, etc.)
- `Letter_Number` (Nl) - Letter-like numbers (Ⅰ, Ⅱ, etc.)
- `Other_Number` (No) - Other numbers (½, ¼, etc.)

### Symbol Categories
```wfl
create pattern symbols:
    unicode category "Symbol"
end pattern

create pattern currency:
    unicode category "Currency_Symbol"
end pattern
```

**Supported Symbol Categories:**
- `Symbol` (S) - All symbols
- `Math_Symbol` (Sm) - Math symbols (+, =, etc.)
- `Currency_Symbol` (Sc) - Currency symbols ($, €, ¥, etc.)
- `Modifier_Symbol` (Sk) - Modifier symbols
- `Other_Symbol` (So) - Other symbols

### Punctuation Categories
```wfl
create pattern punctuation:
    unicode category "Punctuation"
end pattern

create pattern open_punctuation:
    unicode category "Open_Punctuation"
end pattern
```

**Supported Punctuation Categories:**
- `Punctuation` (P) - All punctuation
- `Connector_Punctuation` (Pc) - Connectors (_, ‿, etc.)
- `Dash_Punctuation` (Pd) - Dashes (-, –, —, etc.)
- `Open_Punctuation` (Ps) - Opening punctuation ((, [, {, etc.)
- `Close_Punctuation` (Pe) - Closing punctuation (), ], }, etc.)
- `Initial_Punctuation` (Pi) - Initial quotes (", ', etc.)
- `Final_Punctuation` (Pf) - Final quotes (", ', etc.)
- `Other_Punctuation` (Po) - Other punctuation (!, ?, etc.)

### Mark Categories
```wfl
create pattern marks:
    unicode category "Mark"
end pattern

create pattern combining_marks:
    unicode category "Nonspacing_Mark"
end pattern
```

**Supported Mark Categories:**
- `Mark` (M) - All marks
- `Nonspacing_Mark` (Mn) - Nonspacing marks (◌́, ◌̃, etc.)
- `Spacing_Mark` (Mc) - Spacing combining marks
- `Enclosing_Mark` (Me) - Enclosing marks

### Separator Categories
```wfl
create pattern separators:
    unicode category "Separator"
end pattern

create pattern spaces:
    unicode category "Space_Separator"
end pattern
```

**Supported Separator Categories:**
- `Separator` (Z) - All separators
- `Space_Separator` (Zs) - Space characters
- `Line_Separator` (Zl) - Line separators
- `Paragraph_Separator` (Zp) - Paragraph separators

### Other Categories
```wfl
create pattern control:
    unicode category "Control"
end pattern

create pattern format:
    unicode category "Format"
end pattern
```

**Other Categories:**
- `Control` (Cc) - Control characters
- `Format` (Cf) - Format characters
- `Surrogate` (Cs) - Surrogate characters
- `Private_Use` (Co) - Private use characters
- `Unassigned` (Cn) - Unassigned characters

## Unicode Scripts

Unicode scripts represent different writing systems. WFL supports major Unicode scripts:

### Latin Scripts
```wfl
create pattern latin_text:
    unicode script "Latin"
end pattern

create pattern extended_latin:
    unicode script "Latin_Extended_A"
end pattern
```

### Greek Script
```wfl
create pattern greek_letters:
    unicode script "Greek"
end pattern

// Example: Match Greek letters
store text as "Αγαπώ τη γλώσσα WFL"
store matches as find_all greek_letters in text
```

### Cyrillic Script
```wfl
create pattern cyrillic_text:
    unicode script "Cyrillic"
end pattern

// Example: Match Russian text
store text as "Привет мир"
check if text matches cyrillic_text:
    display "Found Cyrillic text!"
end check
```

### Arabic Script
```wfl
create pattern arabic_text:
    unicode script "Arabic"
end pattern

// Example: Match Arabic text
store text as "مرحبا بالعالم"
check if text matches arabic_text:
    display "Found Arabic text!"
end check
```

### CJK Scripts
```wfl
// Chinese Han characters
create pattern han_characters:
    unicode script "Han"
end pattern

// Japanese Hiragana
create pattern hiragana:
    unicode script "Hiragana"
end pattern

// Japanese Katakana  
create pattern katakana:
    unicode script "Katakana"
end pattern

// Korean Hangul
create pattern hangul:
    unicode script "Hangul"
end pattern
```

### Other Scripts
```wfl
// Hebrew
create pattern hebrew_text:
    unicode script "Hebrew"  
end pattern

// Thai
create pattern thai_text:
    unicode script "Thai"
end pattern

// Devanagari (Hindi, Sanskrit)
create pattern devanagari:
    unicode script "Devanagari"
end pattern
```

**Supported Scripts Include:**
- `Latin` - Latin alphabet and extensions
- `Greek` - Greek and Coptic
- `Cyrillic` - Cyrillic alphabet
- `Arabic` - Arabic script
- `Hebrew` - Hebrew script
- `Devanagari` - Devanagari script (Hindi, Sanskrit)
- `Han` - Chinese Han characters
- `Hiragana` - Japanese Hiragana
- `Katakana` - Japanese Katakana
- `Hangul` - Korean Hangul
- `Thai` - Thai script
- And many more...

## Unicode Properties

Unicode properties provide additional character classification:

### Alphabetic Property
```wfl
create pattern alphabetic:
    unicode property "Alphabetic"
end pattern
```

### Case Properties
```wfl
create pattern uppercase_property:
    unicode property "Uppercase"
end pattern

create pattern lowercase_property:
    unicode property "Lowercase"
end pattern
```

### Numeric Properties
```wfl
create pattern numeric_property:
    unicode property "Numeric"
end pattern
```

**Supported Properties Include:**
- `Alphabetic` - Alphabetic characters
- `Uppercase` - Uppercase characters  
- `Lowercase` - Lowercase characters
- `Numeric` - Numeric characters
- `Hex_Digit` - Hexadecimal digits
- `ASCII_Hex_Digit` - ASCII hexadecimal digits
- `White_Space` - Whitespace characters
- And more...

## Practical Examples

### Email Validation with Unicode
```wfl
create pattern unicode_email:
    one or more {unicode category "Letter" or unicode category "Number" or "_" or "-"}
    "@"
    one or more {unicode category "Letter" or unicode category "Number" or "-"}
    "."
    two or more {unicode category "Letter"}
end pattern

// Test with international domains
store email as "用户@example.中国"
check if email matches unicode_email:
    display "Valid international email!"
end check
```

### Multilingual Text Processing
```wfl
create pattern multilingual_word:
    one or more {unicode category "Letter"}
end pattern

create pattern mixed_script_text:
    capture "english": one or more {unicode script "Latin"}
    whitespace
    capture "chinese": one or more {unicode script "Han"}  
    whitespace
    capture "arabic": one or more {unicode script "Arabic"}
end pattern

store text as "Hello 世界 مرحبا"
store matches as find mixed_script_text in text
check if matches is not null:
    display "English: " with captured "english" from matches
    display "Chinese: " with captured "chinese" from matches  
    display "Arabic: " with captured "arabic" from matches
end check
```

### Currency Detection
```wfl
create pattern currency_amount:
    unicode category "Currency_Symbol"
    one or more digit
    optional {
        "."
        exactly 2 digit
    }
end pattern

store prices as ["$10.50", "€25.00", "¥1000", "£15.99"]
for each price in prices:
    check if price matches currency_amount:
        display price with " is a valid currency amount"
    end check
end for
```

### Name Validation (International)
```wfl
create pattern international_name:
    one or more {
        unicode category "Letter" or 
        unicode category "Mark" or
        "'" or "-" or "."
    }
end pattern

store names as ["José", "François", "Müller", "李小明", "محمد"]
for each name in names:
    check if name matches international_name:
        display name with " is a valid international name"  
    end check
end for
```

## Best Practices

### 1. Use Appropriate Granularity
```wfl
// Good: Specific category for the use case
create pattern letters_only:
    unicode category "Letter"
end pattern

// Less efficient: Overly broad matching
create pattern any_chars:
    unicode category "Any"
end pattern
```

### 2. Combine Categories When Needed
```wfl
create pattern alphanumeric_unicode:
    unicode category "Letter" or unicode category "Number"
end pattern
```

### 3. Consider Script Boundaries
```wfl
// Match complete words within a script
create pattern greek_word:
    word boundary
    one or more {unicode script "Greek"}
    word boundary
end pattern
```

### 4. Handle Mixed Scripts Properly
```wfl
create pattern mixed_content:
    capture "latin": zero or more {unicode script "Latin" or whitespace}
    capture "non_latin": zero or more {
        not {unicode script "Latin" or whitespace}
    }
end pattern
```

## Performance Considerations

1. **Category Matching**: More specific categories are generally faster
2. **Script Matching**: Script checks are optimized for common scripts
3. **Property Matching**: Some properties may require more computation
4. **Caching**: Patterns are compiled once and can be reused efficiently

## Limitations and Notes

1. **Incomplete Symbol Ranges**: Some Symbol category ranges may not be complete (e.g., Euro symbol €)
2. **Script Extensions**: Some characters may belong to multiple scripts
3. **Version Differences**: Unicode support may vary based on the Unicode version
4. **Normalization**: Text normalization is not automatically applied

## Migration from ASCII Patterns

When migrating from ASCII-only patterns to Unicode-aware patterns:

```wfl
// Old ASCII-only approach
create pattern old_letters:
    "a" through "z" or "A" through "Z"
end pattern

// New Unicode-aware approach  
create pattern new_letters:
    unicode category "Letter"
end pattern

// Mixed approach for backwards compatibility
create pattern mixed_letters:
    {"a" through "z" or "A" through "Z"} or
    {unicode category "Letter" and not {unicode script "Latin"}}
end pattern
```

## Testing Unicode Patterns

```wfl
// Test with various Unicode text samples
store test_cases as [
    "Hello",           // ASCII
    "Café",            // Latin with accents
    "Αγάπη",           // Greek
    "Любовь",          // Cyrillic
    "愛",              // CJK
    "حب",              // Arabic
    "אהבה",            // Hebrew
    "प्रेम"             // Devanagari
]

create pattern any_letter:
    one or more {unicode category "Letter"}
end pattern

for each text in test_cases:
    check if text matches any_letter:
        display text with " contains letters: ✓"
    otherwise:
        display text with " no letters found: ✗"
    end check
end for
```

This comprehensive Unicode support makes WFL patterns suitable for international applications and multilingual text processing.