# WFL Documentation Examples

This directory contains all code examples used in the WFL documentation. Every example is validated using a 5-layer validation pipeline to ensure accuracy and quality.

## Directory Structure

```
docs_examples/
├── README.md                 # This file
├── basic_syntax/             # Variables, control flow, functions
├── stdlib/                   # Standard library demonstrations
├── web_features/             # Async, web servers, HTTP
├── error_examples/           # Intentional errors for documentation
└── _meta/                    # Validation metadata
    ├── manifest.json         # Registry of all examples
    ├── expected_errors.json  # Expected error patterns
    └── validation_cache.json # Validation results cache
```

## File Naming Conventions

### Sequential Examples
Use `<topic>_<number>.wfl` for sequential examples:
- `variables_01.wfl` - First variables example
- `variables_02.wfl` - Second variables example
- `control_flow_01.wfl` - First control flow example

### Standalone Examples
Use `<feature>_example.wfl` for standalone demonstrations:
- `email_validation_example.wfl`
- `simple_server_example.wfl`
- `file_operations_example.wfl`

### Error Examples
Use `<error>_error.wfl` for intentional errors:
- `type_mismatch_error.wfl`
- `undefined_variable_error.wfl`
- `runtime_error.wfl`

### Special Prefixes

#### `_snippet_` Prefix
For incomplete code that demonstrates a concept but isn't a complete program:
- `_snippet_variable_declaration.wfl`
- Only validated through parse/analyze/typecheck layers
- **Not executed** (Layer 5 skipped)

#### `_interactive_` Prefix
For examples that require user interaction:
- `_interactive_user_input.wfl`
- `_interactive_web_server.wfl`
- Manual testing required
- **Execution skipped** in automated validation

#### `_expected_fail_` Prefix
For examples that should produce specific errors:
- `_expected_fail_type_error.wfl`
- Must specify `expected_failure_layer` and `expected_error_pattern` in manifest

## Validation Pipeline

Every example goes through up to 5 validation layers:

```
Layer 1: Parse (mcp__wfl-lsp__parse_wfl)
    ↓ Validates syntax correctness
Layer 2: Semantic Analysis (mcp__wfl-lsp__analyze_wfl)
    ↓ Validates program structure and semantics
Layer 3: Type Checking (mcp__wfl-lsp__typecheck_wfl)
    ↓ Validates type correctness
Layer 4: Code Quality (mcp__wfl-lsp__lint_wfl)
    ↓ Checks code style and best practices
Layer 5: Runtime Execution (wfl CLI)
    ↓ Actually runs the program
✅ PASS - Example is valid for documentation
```

### Quality Gates

**BLOCKING (must pass):**
- ✅ Layer 1: Parse - All examples must be syntactically valid
- ✅ Layer 2: Analyze - All examples must be semantically valid
- ✅ Layer 3: Typecheck - All complete programs must type-check
- ✅ Layer 4: Lint - All examples should follow style guidelines
- ✅ Layer 5: Execute - Executable examples must run successfully OR match expected error pattern

**NON-BLOCKING (warnings only):**
- ⚠️ Lint warnings (recorded but don't fail validation)

## Example Types

### 1. Executable (`type: "executable"`)
Complete, runnable programs:
- Must pass all 5 layers
- `expected_exit_code: 0` (default)
- Used for: Complete tutorials, full demonstrations

**Example manifest entry:**
```json
{
  "basic_syntax/variables_01.wfl": {
    "doc_section": "Docs/03-language-basics/variables-and-types.md#declaration",
    "type": "executable",
    "validate_layers": [1, 2, 3, 4, 5],
    "expected_exit_code": 0,
    "tags": ["beginner", "variables"]
  }
}
```

### 2. Snippet (`type: "snippet"`)
Partial code demonstrating a concept:
- Validates layers 1-4 only
- `skip_execution: true`
- Wrapped in minimal context for validation
- Used for: Syntax examples, quick references

**Example manifest entry:**
```json
{
  "basic_syntax/_snippet_variable_declaration.wfl": {
    "doc_section": "Docs/03-language-basics/variables-and-types.md#syntax",
    "type": "snippet",
    "validate_layers": [1, 2, 3, 4],
    "skip_execution": true,
    "tags": ["syntax", "variables"]
  }
}
```

### 3. Error Example (`type: "error_example"`)
Intentionally incorrect code to demonstrate errors:
- Must fail at specified layer
- Requires `expected_failure_layer` (1-4)
- Requires `expected_error_pattern` (regex)
- Used for: Error handling docs, type safety demos

**Example manifest entry:**
```json
{
  "error_examples/type_mismatch_error.wfl": {
    "doc_section": "Docs/03-language-basics/variables-and-types.md#type-safety",
    "type": "error_example",
    "validate_layers": [1, 2, 3],
    "expected_failure_layer": 3,
    "expected_error_pattern": "Type mismatch.*cannot add.*String.*Number",
    "tags": ["error", "type-safety"],
    "doc_purpose": "Demonstrates type safety and clear error messages",
    "fix_suggestion": "Convert both values to the same type before adding"
  }
}
```

### 4. Interactive (`type: "interactive"`)
Examples requiring user interaction or long-running services:
- Validates layers 1-4
- `skip_execution: true` for automated validation
- Requires manual testing
- Used for: Web servers, REPL examples, user input

**Example manifest entry:**
```json
{
  "web_features/_interactive_web_server.wfl": {
    "doc_section": "Docs/04-advanced-features/web-servers.md#basic",
    "type": "interactive",
    "validate_layers": [1, 2, 3, 4],
    "skip_execution": true,
    "tags": ["web-server", "interactive", "manual-test"],
    "doc_purpose": "Demonstrates basic web server setup"
  }
}
```

## Adding New Examples

### Step 1: Write the Example
Create the `.wfl` file in the appropriate directory:
```bash
# For basic syntax examples
vi TestPrograms/docs_examples/basic_syntax/loops_01.wfl

# For stdlib examples
vi TestPrograms/docs_examples/stdlib/text_functions.wfl
```

### Step 2: Register in Manifest
Add entry to `_meta/manifest.json`:
```json
{
  "basic_syntax/loops_01.wfl": {
    "doc_section": "Docs/03-language-basics/loops-and-iteration.md#count-loop",
    "type": "executable",
    "validate_layers": [1, 2, 3, 4, 5],
    "tags": ["beginner", "loops", "count"],
    "doc_purpose": "Demonstrates basic count loop syntax"
  }
}
```

### Step 3: Validate
Run validation script:
```bash
# Validate single file
python scripts/validate_docs_examples.py --file TestPrograms/docs_examples/basic_syntax/loops_01.wfl

# Validate entire category
python scripts/validate_docs_examples.py --category basic_syntax

# Validate all examples
python scripts/validate_docs_examples.py
```

### Step 4: Use in Documentation
Reference the example in your markdown file:
````markdown
## Count Loops

WFL provides natural language loops for iteration:

```wfl
count from 1 to 10:
    display "Number: " with the current count
end count
```

This example demonstrates the basic `count` loop syntax.
````

## Validation Tools

### Python Script
```bash
# Validate all examples
python scripts/validate_docs_examples.py

# Validate specific category
python scripts/validate_docs_examples.py --category basic_syntax

# Validate single file
python scripts/validate_docs_examples.py --file path/to/example.wfl

# CI mode (strict, no prompts)
python scripts/validate_docs_examples.py --ci

# Update manifest after validation
python scripts/validate_docs_examples.py --update-manifest

# Detailed report
python scripts/validate_docs_examples.py --report --verbose
```

### Cross-Platform Wrappers
```bash
# Windows
.\scripts\validate_docs_examples.ps1

# Linux/macOS
./scripts/validate_docs_examples.sh
```

## Best Practices

### 1. Keep Examples Focused
- One concept per example
- Avoid mixing multiple topics
- Clear, descriptive variable names

### 2. Add Comments
```wfl
// This demonstrates variable declaration
store name as "Alice"

// Variables can be changed
change name to "Bob"

// Display the result
display name  // Output: Bob
```

### 3. Use Realistic Examples
- Prefer practical use cases over contrived examples
- Show common patterns developers will actually use
- Include error handling where appropriate

### 4. Follow WFL Style Guide
- Use natural language syntax
- Prefer words over symbols
- 4-space indentation
- Max 100 characters per line
- No trailing whitespace

### 5. Test Before Documenting
- Always validate examples before using in docs
- Ensure examples actually run
- Check error messages match documentation

### 6. Tag Appropriately
Use consistent tags:
- **Skill level**: `beginner`, `intermediate`, `advanced`
- **Category**: `variables`, `loops`, `functions`, `web-server`, etc.
- **Purpose**: `tutorial`, `reference`, `error`, `pattern`

### 7. Update Manifest Metadata
- Keep `content_hash` current
- Update `last_validated` timestamps
- Record `validation_result` accurately

## Error Example Guidelines

When creating error examples:

1. **Clearly mark as error**
   - Use `error_examples/` directory
   - Set `type: "error_example"` in manifest

2. **Specify expected failure**
   ```json
   "expected_failure_layer": 3,
   "expected_error_pattern": "Type mismatch.*cannot add.*String.*Number"
   ```

3. **Document the error**
   - Add `doc_purpose`: Why showing this error matters
   - Add `fix_suggestion`: How to fix it
   - Include comments in the code explaining the error

4. **Example error file structure**
   ```wfl
   // error_examples/type_mismatch_error.wfl
   // Demonstrates type safety - intentionally fails at type checking

   store age as 30
   store name as "Alice"

   // This will fail type checking (expected!)
   display age plus name  // ERROR: Cannot add Number and String

   // FIX: Convert to same type first
   // display age plus " " plus name  // This would work
   ```

## Maintenance

### Regular Tasks
- **Weekly**: Run full validation suite
- **Per PR**: Validate changed examples
- **Per Release**: Update `wfl_version` in cache
- **Monthly**: Review and prune obsolete examples

### Validation Cache
The `validation_cache.json` file stores validation results to avoid re-validating unchanged files:
- Cache is updated after each validation run
- Files with unchanged `content_hash` skip validation
- Cache expires after 7 days for safety
- Clear cache: `rm _meta/validation_cache.json`

### CI/CD Integration
Examples are automatically validated in CI:
- GitHub Actions workflow: `.github/workflows/docs-validation.yml`
- Runs on every PR touching documentation or examples
- Blocks merge if validation fails
- Uploads validation report as artifact

## Troubleshooting

### Example won't validate
1. Check syntax: `wfl --parse example.wfl`
2. Check semantics: `wfl --analyze example.wfl`
3. Check types: Run typecheck via MCP
4. Check execution: `wfl example.wfl`
5. Check manifest entry for typos

### Error example passes when it should fail
1. Verify `expected_failure_layer` is correct
2. Check `expected_error_pattern` regex
3. Ensure example actually contains the error
4. Test manually: `wfl example.wfl`

### Validation is slow
1. Use `--category` to validate specific categories
2. Check `validation_cache.json` is being used
3. Use `--file` for single file validation
4. Consider parallel validation (automatic in script)

## Resources

- **Validation Script**: `scripts/validate_docs_examples.py`
- **Manifest Schema**: `_meta/manifest.json` (top-level `$schema`)
- **Documentation Policy**: `Docs/wfl-documentation-policy.md`
- **Style Guide**: `Docs/06-best-practices/code-style-guide.md`
- **WFL Specification**: `Docs/reference/language-specification.md`

## Questions?

- Open an issue on GitHub
- Check the documentation
- Ask in discussions

---

**Remember**: Every example in documentation must be validated! No exceptions!
