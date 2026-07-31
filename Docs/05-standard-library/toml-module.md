# TOML Module

The TOML module reads and writes [TOML](https://toml.io) — the format a large share of configuration files are written in. Parsing is the common case: a program reads a config file someone else wrote.

WFL also has JSON functions with exactly the same shape (`parse_json`, `stringify_json`, `stringify_json_pretty`), so switching a program between the two formats is a matter of changing the function name.

## How TOML maps onto WFL values

| TOML | WFL |
| --- | --- |
| Table (`[section]`) | Object — read keys with `config["section"]` |
| Array of tables (`[[item]]`) | List of objects |
| Array (`[1, 2, 3]`) | List |
| String | Text |
| Integer, float | Number |
| Boolean | Boolean (`yes` / `no`) |
| Date, time, datetime | Text, in the format the file used |

Dates and times come back as **text**, in exactly the form the file used. TOML distinguishes offset datetimes, local datetimes, local dates and local times, and collapsing those into a single WFL date value would lose the distinction.

The cost is that a date does not survive a *write* as a date. Reading `released = 2026-07-31` and writing it back produces `released = "2026-07-31"` — same characters, but now a TOML string rather than a TOML date. If a consumer of your file cares about the difference, read the value and construct the output yourself instead of round-tripping.

## Functions

### parse_toml

**Purpose:** Read TOML text into a WFL value.

**Signature:**
```wfl
parse_toml of <text>
```

**Parameters:**
- `text` (Text): TOML document text

**Returns:** Object — the document's top-level table

**Raises:** An error if the text is not valid TOML. Duplicate keys are an error, not last-wins, as the TOML specification requires.

**Example:**
```wfl
store config_text as "title = \"my project\"
listen_port = 8080
debug_mode = true

[database]
host = \"localhost\"
"

store config as parse_toml of config_text
display config["title"]                    // Output: my project
display config["listen_port"]              // Output: 8080

store db_section as config["database"]
display db_section["host"]                 // Output: localhost
```

Reading a config file from disk is the usual case:

```wfl
open file at "settings.toml" for reading as settings_file
store settings_text as read content from settings_file
close file settings_file

store settings as parse_toml of settings_text
```

**Use Cases:**
- Reading application configuration
- Reading a tool's config file whose format is fixed by a specification
- Reading manifests and metadata files

---

### stringify_toml

**Purpose:** Convert a WFL value to TOML text.

**Signature:**
```wfl
stringify_toml of <value>
```

**Parameters:**
- `value` (Object): The value to write. It must be an object — see "Writing rules" below.

**Returns:** Text — a TOML document

**Example:**
```wfl
store config as parse_toml of "name = \"wfl\"
listen_port = 8080
"

store rendered as stringify_toml of config
display rendered
// Output:
// listen_port = 8080
// name = "wfl"
```

Keys come out in alphabetical order, not the order they appeared in the original
file. The output is deterministic — the same value always produces the same text,
which matters if you write a config file into version control — but round-tripping
a hand-written file will reorder it, and comments are not preserved.

---

### stringify_toml_pretty

**Purpose:** Convert a WFL value to TOML text with more readable formatting for nested structures.

**Signature:**
```wfl
stringify_toml_pretty of <value>
```

**Parameters:**
- `value` (Object): The value to write

**Returns:** Text — a TOML document

**Example:**
```wfl
store config as parse_toml of "[server_config]
host = \"localhost\"
ports = [1, 2]
"

store rendered as stringify_toml_pretty of config
store reparsed as parse_toml of rendered
store section as reparsed["server_config"]
display section["host"]                    // Output: localhost
```

## Writing rules

Two things about TOML differ from JSON, and WFL is explicit about both rather than guessing.

**A TOML document is always a table.** There is no valid TOML file whose top level is a list or a bare string, so `stringify_toml` accepts only an object and raises an error otherwise. Writing something that would not parse back would be worse than refusing.

```wfl
store rendered as stringify_toml of [1 and 2 and 3]   // Error: a TOML document is always a table
```

**TOML has no null.** Absence is spelled by leaving the key out, so a key whose value is `nothing` is simply omitted:

```wfl
store settings as parse_toml of "present = \"yes\"\n"
store rendered as stringify_toml of settings          // the key is written; a nothing-valued key would not be
```

Inside a *list* there is no way to leave a hole — dropping an entry would change the list's length — so `nothing` in a list is an error instead.

**Whole numbers stay integers.** A number with no fractional part is written as a TOML integer, so a config round-trips as `listen_port = 8080` rather than `listen_port = 8080.0`.

**Writing is not editing.** Parsing and re-writing a file produces a valid, equivalent document, but not the same bytes: keys are sorted alphabetically, comments are dropped, and dates become strings (see above). If you need to preserve someone's hand-written file exactly, read it and write your own output elsewhere rather than round-tripping theirs.

**Very large integers lose precision.** WFL numbers are 64-bit floating point, which represents whole numbers exactly only up to 9,007,199,254,740,991 (2⁵³−1). A TOML integer above that is rounded on the way in — `9007199254740993` reads back as `9007199254740992` — so a parse-and-rewrite cycle can silently change it. This applies to every number in WFL, JSON included, not just TOML; if a config carries identifiers that large, keep them as quoted strings.

## Related

- [Filesystem Module](filesystem-module.md) — reading the file the TOML text comes from
- [Crypto Module](crypto-module.md) — `seal` and `unseal` for secrets you store in a config file
- [File I/O](../04-advanced-features/file-io.md) — opening and reading files
