# Built-in Functions Reference

Complete alphabetical list of all 181+ WFL built-in functions.

## Core Module (3 functions)

| Function | Signature | Returns | Module |
|----------|-----------|---------|--------|
| `display` | `display <value>` | None | Core |
| `isnothing` | `isnothing of <value>` | Boolean | Core |
| `typeof` | `typeof of <value>` | Text | Core |

**[Core Module Details →](../05-standard-library/core-module.md)**

## Math Module (5 functions)

| Function | Signature | Returns | Description |
|----------|-----------|---------|-------------|
| `abs` | `abs of <number>` | Number | Absolute value |
| `ceil` | `ceil of <number>` | Number | Round up |
| `clamp` | `clamp of <value> between <min> and <max>` | Number | Constrain to range |
| `floor` | `floor of <number>` | Number | Round down |
| `round` | `round of <number>` | Number | Round to nearest |

**[Math Module Details →](../05-standard-library/math-module.md)**

## Text Module (8 functions)

| Function | Signature | Returns | Description |
|----------|-----------|---------|-------------|
| `contains` | `contains of <text> and <search>` | Boolean | Check substring |
| `ends_with` | `ends_with of <text> and <suffix>` | Boolean | Check suffix |
| `length` | `length of <text>` | Number | Character count |
| `starts_with` | `starts_with of <text> and <prefix>` | Boolean | Check prefix |
| `string_split` | `split of <text> by <delimiter>` | List | Split into list |
| `substring` | `substring of <text> from <start> length <len>` | Text | Extract portion |
| `tolowercase` | `tolowercase of <text>` | Text | Convert to lowercase |
| `touppercase` | `touppercase of <text>` | Text | Convert to uppercase |
| `trim` | `trim of <text>` | Text | Remove whitespace |

**[Text Module Details →](../05-standard-library/text-module.md)**

## List Module (5 functions)

| Function | Signature | Returns | Description |
|----------|-----------|---------|-------------|
| `indexof` | `indexof of <list> and <value>` | Number | Find position (-1 if not found) |
| `length` | `length of <list>` | Number | Item count |
| `pop` | `pop from <list>` | Any | Remove from end |
| `push` | `push with <list> and <value>` | None | Add to end |

**[List Module Details →](../05-standard-library/list-module.md)**

## Filesystem Module (20+ functions)

| Function | Signature | Returns | Description |
|----------|-----------|---------|-------------|
| `copy_file` | `copy_file from <src> to <dest>` | None | Copy file |
| `count_lines` | `count_lines at <path>` | Number | Count lines in file |
| `file_size` | `file size at <path>` | Number | Get file size in bytes |
| `is_dir` | `is_dir at <path>` | Boolean | Check if directory |
| `is_file` | `is_file at <path>` | Boolean | Check if file |
| `list files in` | `list files in <path>` | List | List directory files |
| `makedirs` | `makedirs <path>` | None | Create directory tree |
| `move_file` | `move_file from <src> to <dest>` | None | Move/rename file |
| `path_basename` | `path basename of <path>` | Text | Get filename |
| `path_dirname` | `path dirname of <path>` | Text | Get directory |
| `path_exists` | `path exists at <path>` | Boolean | Check existence |
| `path_extension` | `path extension of <path>` | Text | Get extension |
| `path_join` | `path_join of <p1> and <p2>...` | Text | Join paths |
| `path_stem` | `path stem of <path>` | Text | Filename without extension |
| `remove_dir` | `remove_dir at <path>` | None | Delete directory |
| `remove_file` | `remove_file at <path>` | None | Delete file |

**[Filesystem Module Details →](../05-standard-library/filesystem-module.md)**

## Time Module (14 functions)

| Function | Signature | Returns | Description |
|----------|-----------|---------|-------------|
| `add_days` | `add_days of <date> and <days>` | Date | Add days to date |
| `create_date` | `create_date of <year> and <month> and <day>` | Date | Create date |
| `create_time` | `create_time of <hour> and <min> and <sec>` | Time | Create time |
| `current time in milliseconds` | `current time in milliseconds` | Number | Unix timestamp |
| `datetime_now` | `datetime_now` | DateTime | Current datetime |
| `day` | `day of <date>` | Number | Day component |
| `days_between` | `days_between of <date1> and <date2>` | Number | Days difference |
| `dayofweek` | `dayofweek of <date>` | Number | Day of week |
| `format_date` | `format_date of <date> and <format>` | Text | Format date |
| `hour` | `hour of <time>` | Number | Hour component |
| `minute` | `minute of <time>` | Number | Minute component |
| `month` | `month of <date>` | Number | Month component |
| `now` | `now` | Time | Current time |
| `second` | `second of <time>` | Number | Second component |
| `subtract_days` | `subtract_days of <date> and <days>` | Date | Subtract days |
| `today` | `today` | Date | Current date |
| `year` | `year of <date>` | Number | Year component |

**[Time Module Details →](../05-standard-library/time-module.md)**

## Random Module (6 functions)

| Function | Signature | Returns | Description |
|----------|-----------|---------|-------------|
| `random` | `random` | Number | Random 0-1 |
| `random_between` | `random_between of <min> and <max>` | Number | Random in range |
| `random_boolean` | `random_boolean` | Boolean | Random true/false |
| `random_from` | `random_from of <list>` | Any | Random from list |
| `random_int` | `random_int between <min> and <max>` | Number | Random integer |
| `random_seed` | `random_seed of <seed>` | None | Set RNG seed |

**[Random Module Details →](../05-standard-library/random-module.md)**

## Crypto Module (4 functions)

| Function | Signature | Returns | Description |
|----------|-----------|---------|-------------|
| `wflhash256` | `wflhash256 of <text>` | Text | 256-bit hash |
| `wflhash256_with_salt` | `wflhash256_with_salt of <text> and <salt>` | Text | Salted 256-bit hash |
| `wflhash512` | `wflhash512 of <text>` | Text | 512-bit hash |
| `wflmac256` | `wflmac256 of <message> and <key>` | Text | Message auth code |

**[Crypto Module Details →](../05-standard-library/crypto-module.md)**

**Security:** WFLHASH not externally audited. Use SHA-256/SHA-3 for production.

## Pattern Module (3 functions)

| Function | Signature | Returns | Description |
|----------|-----------|---------|-------------|
| `pattern_find` | `find <pattern> in <text>` | Match object | Find first match |
| `pattern_find_all` | `find all <pattern> in <text>` | List | Find all matches |
| `pattern_matches` | `<text> matches <pattern>` | Boolean | Test if matches |

**[Pattern Module Details →](../05-standard-library/pattern-module.md)**

---

## Quick Lookup

**Need to find a function?** Use Ctrl+F to search this page.

**By category:** See [Standard Library Index](../05-standard-library/index.md)

**Detailed docs:** Click module links above.

---

**Previous:** [← Operator Reference](operator-reference.md) | **Next:** [Error Codes →](error-codes.md)
