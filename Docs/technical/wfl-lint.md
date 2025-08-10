Awesome — I’ll design how to integrate a strict linter, deep static analyzer, and comprehensive code fixer directly into the `wfl` binary, all triggered via command-line flags. I’ll look into how ESLint, Clippy, and Black approach their tooling and borrow best practices for rule strictness, architecture, and formatting behavior.

I’ll get back to you with a detailed architecture, implementation plan, and CLI integration approach.

# Design of a Comprehensive Code Quality Suite for WFL

## Introduction  
WFL (Work Flow Language) can greatly benefit from an integrated code quality suite to improve code consistency, catch errors early, and ease maintenance. Modern development practices use linters, static analyzers, and code formatters to enforce standards and flag potential issues before runtime. This proposal outlines a **strict code quality suite** built into the WFL binary, comprising three components: 

- **WFL Linter** – Enforces code style and structural rules (inspired by ESLint, Rust’s Clippy, and Python’s Black) to catch stylistic inconsistencies and potential pitfalls.  
- **WFL Static Analyzer** – Performs deeper analysis beyond basic parsing/type-checking to detect unused code, unreachable statements, inconsistent return paths, and other logical issues.  
- **WFL Code Fixer** – Auto-formats and refactors WFL code into a standardized style, and can suggest or apply idiomatic improvements for better structure and readability.  

Each tool is accessible via dedicated command-line flags (`--lint`, `--analyze`, `--fix`) separate from normal script execution. In the sections below, we discuss the architecture for integrating these tools into the WFL compiler/interpreter, the design of each tool’s rule engine, expected output format with examples, testing strategies, and lessons drawn from ESLint, Clippy, and Black.

## Architecture and Integration Plan  
**Integration Overview:** The code quality suite will be embedded directly into the WFL binary, leveraging the existing compiler front-end (lexer, parser, and semantic analyzer) to avoid duplication. The WFL binary will interpret command-line flags to decide whether to execute a script normally or run one of the quality tools. For example: 

- `wfl --lint program.wfl` will run the linter on `program.wfl` and report style issues without executing it.  
- `wfl --analyze program.wfl` will perform static analysis checks and output any warnings about unused or unreachable code.  
- `wfl --fix program.wfl` will format and refactor the code, printing the corrected code (or writing to a file, if an option like `--fix --in-place` is specified).  

**Pipeline Reuse:** In all modes, the front-end pipeline is similar: the source is lexed into tokens and parsed into an Abstract Syntax Tree (AST) representing the program. The AST (and associated symbol tables/type info) is then passed to the respective tool: 

- **Linter mode:** After parsing (and possibly a lightweight semantic pass for symbol information), the AST is walked by the linter’s rule engine to check for style and structural conformance. There is no execution of the program.  
- **Analyzer mode:** After parsing, the existing semantic analysis (symbol resolution and type checking) is performed to build symbol tables and detect basic errors. Then, additional static analysis passes are run on the AST/Control Flow Graph to find issues like dead code or unused definitions.  
- **Fix mode:** After parsing (and optional semantic checks to ensure the code is valid), the AST is pretty-printed or transformed by the code fixer according to the style guide. The code fixer may also utilize analysis results (e.g. symbol table or lint suggestions) to refactor certain patterns. A final parse of the fixed code can be done to verify it’s syntactically correct and equivalent in meaning to the original (similar to how Black verifies the AST equivalence after formatting ([Black 25.1.0 documentation](https://black.readthedocs.io/#:~:text=Also%2C%20as%20a%20safety%20measure,fast))). 

All three tools will use a common **diagnostics reporting** subsystem (integrated with WFL’s `DiagnosticReporter`) to format messages consistently. This subsystem can provide file/line/column information and even code snippets with highlighted sections for context, akin to modern compilers. Internally, we will represent each finding (lint warning, analysis warning, etc.) as a diagnostic with a severity level (e.g. Warning or Note) and an associated source code span. The existing `SimpleFiles` and `codespan_reporting` infrastructure in WFL will be reused to emit these messages with color and source context for clarity ([src.md](file://file-777ATHkn91MKSCavSXGfnn#:~:text=pub%20fn%20report_diagnostic,clone)) ([src.md](file://file-777ATHkn91MKSCavSXGfnn#:~:text=let%20writer%20%3D%20StandardStream%3A%3Astderr,config%20%3D%20term%3A%3AConfig%3A%3Adefault)).

**Command-Line Interface:** The WFL binary's argument parser recognizes the quality-check flags. These flags are mutually exclusive with normal execution; if a quality-check flag is present, the binary will not run the program but instead run the corresponding tool. The `--lint` and `--fix` flags can be combined (as `--lint --fix`) to apply auto-fixes to lint findings, with additional options `--in-place` to modify the file directly or `--diff` to show changes as a diff. The `--fix` flag must always be combined with `--lint`. The exit code indicates success or the presence of warnings/errors (e.g. returning a non-zero code if any issues were found, which can be useful in CI pipelines).

**Performance Considerations:** Because the tools reuse the compiler front-end, running a lint or analysis is efficient – the overhead is just the analysis of the AST, which is typically much faster than executing the program. For large WFL scripts, we will ensure that the static analyses are optimized (e.g. using efficient AST traversal and avoiding exponential algorithms) so that these tools can run frequently (such as on each commit or as an IDE plugin) without noticeable lag.

## WFL Linter Design  
The **WFL Linter** focuses on enforcing strict code style and structure conventions across WFL programs. Its goal is to make code more uniform and readable, and to catch potential code smells or confusing constructs. Drawing inspiration from **ESLint** (JavaScript), **Rust Clippy**, and **Black** (Python’s formatter), the linter will check for a variety of issues: naming conventions, consistent formatting choices, and complexity of code structure. Importantly, the linter will restrict itself to *reporting* issues and not reformat code (that is the job of the code fixer) – this follows the ESLint philosophy of using a linter to highlight issues and better ways to code, rather than for mechanical formatting ([Formatters, linters, and compilers: Oh my! · GitHub](https://github.com/readme/guides/formatters-linters-compilers#:~:text=One%20of%20the%20most%20common,when%20compared%20to%20dedicated%20formatters)).

### Linting Rule Engine  
The linter is implemented as a set of pluggable **lint rules**, each targeting a specific pattern or style guideline. Under the hood, the linter will traverse the AST of a WFL program and invoke rule checks on relevant nodes. This design is similar to ESLint’s, where during AST traversal each rule can inspect nodes of interest and report violations ([Architecture - ESLint - Pluggable JavaScript Linter](https://eslint.org/docs/latest/contribute/architecture/#:~:text=Once%20the%20AST%20is%20available%2C,the%20appropriate%20AST%20node%20available)) ([Architecture - ESLint - Pluggable JavaScript Linter](https://eslint.org/docs/latest/contribute/architecture/#:~:text=Individual%20rules%20are%20the%20most,the%20AST%20and%20report%20warnings)). Each rule in WFL’s linter will implement an interface (e.g. a trait in Rust) with methods like `check_node(node)` or `exit_node(node)` to analyze the AST node and possibly emit a warning. This modular design makes it easy to add or remove rules without affecting others and allows selective enabling of rules if needed. 

Key lint rules for WFL might include:  

- **Naming Conventions:** Enforce consistent naming schemes for identifiers. For example, variable and function names could be required to be in snake_case (all lowercase with underscores) or lowerCamelCase – the choice can be configured, but it must be consistent. The linter will flag names that don’t match the convention (e.g. using `MixedCase` or `camelCase` when the standard is snake_case). It can also catch Hungarian notation or other undesirable naming patterns. This is inspired by common ESLint rules and Clippy lints that enforce naming styles (Rust’s style suggests snake_case for variables/functions).  
- **Indentation & Formatting Consistency:** Ensure that the code’s indentation and spacing adhere to the style guide. For instance, enforce 4 spaces per indentation level (no tabs) and flag any lines with misaligned indent (either too many or too few spaces). Also check for common formatting issues like trailing whitespace or missing a newline at end of file. These are typically concerns for a formatter, but the linter can detect them as well to ensure even code that isn’t auto-formatted is flagged. (In practice, the code fixer will auto-fix these, so the linter’s role here is mainly to double-check or to handle cases where the fixer isn’t used).  
- **Braces/Block Structure:** Enforce rules about block structure. For example, if WFL uses keywords like `if ... then ... end`, the linter can require that `end` is present for every block and that no extraneous `end` exists (though the parser likely handles missing/extra `end` as errors). Another rule might be that the `otherwise` (else) clause in an if-statement should align with the `if` (no extra indent) and that the code inside each block is indented one level. Any deviation (such as an `otherwise` or `end` mis-indented) would be reported.  
- **Excessive Nesting & Complexity:** Warn when code is nested too deeply or is too complex. For example, if there are more than N nested conditional/loop levels, the linter could suggest refactoring for clarity. High cyclomatic complexity or deeply nested if-else ladders make code hard to read; Clippy, for instance, has a “cognitive complexity” lint in development to catch overly complex functions ([Suggestion for new lint: Cognitive Complexity · Issue #3793 - GitHub](https://github.com/rust-lang/rust-clippy/issues/3793#:~:text=Suggestion%20for%20new%20lint%3A%20Cognitive,way%20to%20measure%20the)). WFL’s linter can similarly flag functions that exceed a certain complexity threshold or contain deeply nested blocks, indicating the code may need simplification.  
- **Stylistic Best Practices:** A catch-all category for other idiomatic suggestions. For example, if WFL allows certain synonyms or multiple ways to do something, choose one as the recommended style. (In Rust Clippy, there are many lints suggesting more idiomatic alternatives to certain code patterns ([GitHub - rust-lang/rust-clippy: A bunch of lints to catch common mistakes and improve your Rust code. Book: https://doc.rust-lang.org/clippy/](https://github.com/rust-lang/rust-clippy#:~:text=There%20are%20over%20750%20lints,included%20in%20this%20crate)).) In WFL, this might include things like preferring a built-in operation over a verbose manual sequence. Another example: if a boolean expression is written in a convoluted way (like `if flag == yes then ...` when `flag` is already boolean), the linter might suggest writing `if flag then ...` directly. Or if the language allows both `x plus y` and `x + y`, the style guide might designate one form (probably the more English-like `plus`) as preferred, and the linter would flag usage of the other.  

Each rule will produce a **diagnostic message** that includes the location and a clear description of the issue, possibly with a suggestion. For example, a naming violation might produce a warning: *“Variable `MyVar` should be in snake_case (e.g. `my_var`).”* A nesting warning might say: *“Function `processData` exceeds allowed nesting depth (5); consider refactoring to reduce complexity.”* The linter’s outputs are classified as **warnings** (since style issues don’t stop execution, but should be addressed). However, in strict mode (if introduced), these could be escalated to errors to enforce zero-tolerance in CI.

We will allow configuration to some extent (for example, selecting the naming convention or max line length). This could be via a config file (like an `.wfllint.json`) or simple command-line options. By default, all rules are enabled (“strict” mode). If needed, users can suppress a specific lint for a line or block – for instance, by a special comment or pragma (similar to ESLint’s `// eslint-disable` comments or Rust’s `#[allow(lint_name)]` attribute). This ensures that in rare cases where a rule isn’t applicable, the developer can override it, but such occurrences can be minimized.

Notably, we heed the advice that **linters should not handle code formatting when a formatter is available** ([Formatters, linters, and compilers: Oh my! · GitHub](https://github.com/readme/guides/formatters-linters-compilers#:~:text=One%20of%20the%20most%20common,when%20compared%20to%20dedicated%20formatters)). Thus, while the linter may detect indentation issues or the like, it won’t attempt to fix them; it primarily alerts the developer. The presence of the code fixer means that many purely formatting-related issues can be auto-corrected, so the linter’s emphasis should be on more semantic or logical style issues (naming, complexity, best practices) rather than battling whitespace. 

### Linter Output and Example  
Linter warnings will be reported through the diagnostic system as **“Warning”** severity messages. The output format will be reader-friendly and consistent with how WFL reports parse or type errors. For each issue, the tool will output the file name, line and column of the problematic code, a lint code or category, and a message. For example:

```
test.wfl:2:7: **Warning (Style/Naming)** – Variable `Counter` is not in snake_case. Rename to `counter` for consistency.
test.wfl:3:1: **Warning (Style/Indentation)** – Indentation is off by 2 spaces. Expected 4 spaces for this block level.
```

If the diagnostic reporter is configured to show source snippets, it could display something like:

```text
warning: Variable `Counter` is not in snake_case (inconsistent naming)
 --> test.wfl:2:7
  |
2 | store Counter as 5
  |       ^^^^^^^ help: use lowercase_with_underscores, e.g. `counter`
``` 

Each warning would similarly highlight the relevant code. In this example, the variable name “Counter” is flagged and a suggested fix (`counter`) is shown as a note. Another warning for indentation might highlight the line start or misaligned token with a message about expected indentation.

**Example:** Consider a snippet of WFL code with a couple of style issues: 
```wfl
store Counter as  5
if Counter > 0 then
    display "Positive"
otherwise
display "Non-positive"
end
```
Here, the variable `Counter` starts with an uppercase letter (violating a lowercase naming convention), and the `otherwise` keyword is not indented to match the `if`. Running `wfl --lint` on this file would produce warnings like: 

- *Line 1:* naming convention warning for `Counter`.  
- *Line 4:* indentation/structure warning that `otherwise` should align with the `if`.  

For instance: 
```
example.wfl:1:7: Warning [LINT001] Variable "Counter" should be lowercase (snake_case).
example.wfl:4:1: Warning [LINT002] Misaligned `otherwise` – should be indented to the same level as the matching `if`.
```

These warnings guide the developer to rename the variable and fix the indent. After addressing the linter feedback (or using `--fix` as described later), the code would become:

```wfl
store counter as 5
if counter > 0 then
    display "Positive"
otherwise
    display "Non-positive"
end
```

which is clean of the reported lint issues. By enforcing such rules, the linter ensures all team members write WFL in a consistent style, making it easier to read and reducing the cognitive load of understanding code structure.

## WFL Static Analyzer Design  
The **WFL Static Analyzer** augments the compiler’s existing semantic checks and type checking with deeper static analysis to catch issues that, while not outright syntax or type errors, can lead to bugs or maintenance problems. These include unused variables, code that will never execute, contradictory conditions, and inconsistent behavior across code paths. Many compilers and tools provide such warnings (e.g., GCC/Clang warn about unused variables, and Rust’s compiler issues warnings for unused or unreachable code by default). By building a static analyzer into WFL, developers get immediate feedback on these subtler issues. 

### Static Analysis Checks  
Key analyses and checks performed by the WFL static analyzer will include: 

- **Unused Variable/Function Detection:** Any variable that is declared but never read (or never used after its initial assignment) should be flagged. Unused functions (defined but never called) can also be reported, as they bloat the code and may indicate either dead code or a mistake (perhaps the name is misspelled when calling). The analyzer can leverage the symbol table from the semantic phase: after parsing the whole program, it knows all declared symbols (variables, constants, functions) and can track references to them. If a symbol’s reference count is 1 (the declaration itself) and it’s never used elsewhere, it’s unused. For example, if the code declares a variable `temp` that is never read, the analyzer would produce a warning: *“Variable `temp` is never used.”* In some cases, the tool might suggest a fix – e.g., if this was intentional (perhaps for future use), advise prefixing the name with `_` or another convention to signal it (Rust and other languages use a leading underscore to mark a variable as intentionally unused) ([How to avoid unused Variable warning in Rust? - GeeksforGeeks](https://www.geeksforgeeks.org/how-to-avoid-unused-variable-warning-in-rust/#:~:text=How%20to%20avoid%20unused%20Variable,name%20with%20an%20underscore)).  
- **Unreachable Code:** Code that can never be executed due to the control flow will be detected. Common cases include code after a `return` or `exit` statement in a function, or after a `break`/`continue` that jumps out of the current block. Another case is an `if` branch that is always false (for example, `if 2 > 3 then ...` or an obviously impossible condition) – the analyzer could evaluate constant expressions and determine that certain branches won’t run. Unreachable code often indicates either forgotten removal of old code or logical errors in conditions. The analyzer will build a basic **Control Flow Graph (CFG)** for function bodies and the main script, and perform a reachability analysis. Each basic block or statement is marked if it’s reachable from the entry. If any statement is not reachable, a warning is raised: *“Unreachable code detected at line X – this code will never execute.”* For example: 
  ```wfl
  define action foo
      return 42
      display "This will never run"
  end
  ``` 
  Here the `display` is unreachable; the tool would warn at that line. This is analogous to warnings in many compilers.  
- **Dead Branches or Redundant Conditions:** Similar to unreachable code, if an entire branch of a conditional will never execute, the tool flags it. For instance, `if false then ... otherwise ... end` – the `if` part is never taken. Or if a condition always holds (maybe a prior logic guarantees something), the “else” is dead. The analyzer could detect constant boolean conditions or tautologies/contradictions. Another example: `if X > 5 then ... elseif X > 5 then ... end` – the second condition is redundant (dead) because it’s the same check as the first. These logical issues could be either mistakes or leftovers from refactoring. The analyzer’s data flow or constant propagation pass can catch simple cases.  
- **Inconsistent Return Paths:** In functions or actions that are supposed to return a value, ensure that all code paths actually produce a value of the correct type. If one branch returns a value but another falls through without returning, that’s a bug. The WFL type checker might catch some of these (e.g., missing return leading to an implicit return of “nothing”), but the static analyzer will explicitly warn if not all paths return or if some `return` statements return inconsistent types. For example: 
  ```wfl
  define action called compute needs number x give back number
      if x > 0 then
          return x
      end
      // (implicitly returns nothing here)
  end
  ``` 
  This function claims to return a number but fails to return one in the case `x <= 0`. The static analyzer would emit a warning (or error) like: *“Not all code paths in `compute` return a number.”* Inconsistent return issues can lead to runtime errors or unexpected behavior, so highlighting them is crucial.  
- **Other Semantic Best Practices:** We can include checks for other issues such as:
  - **Variable Shadowing:** If a local variable name shadows an outer variable name, it might be unintentional and cause confusion. The analyzer can warn, “Variable `count` in inner scope shadows a variable in an outer scope.” This is similar to a Clippy lint (Rust warns on shadowing by default as well).  
  - **Always-True/False Conditions:** If the analyzer can deduce that a condition uses values that make it always true or false (perhaps a constant or a variable that doesn’t change), it should warn that the condition is redundant. Example: `let flag = true; if flag == true then ...` – this condition is always true (and could be simplified), so a warning or note could be issued (Clippy has a lint for boolean comparisons like this).  
  - **No-Effect Code:** Detect statements that don’t affect program state or output. For instance, a computation whose result is not used anywhere (e.g., `x + 5;` on its own line, or calling a function that returns a value and not using it). If WFL syntax allows such expressions as statements, they might be useless (unless they have side effects). This is akin to “dead store” or dead code elimination warnings. The static analyzer can flag, *“Result of expression `x + 5` is not used.”*  
  - **Potential Error Patterns:** While deeper bug detection (like array out-of-bounds or null dereferences) might be beyond the initial scope without complex analysis or symbolic execution, simpler patterns can be checked. For example, if a variable is used but was never assigned (uninitialized), the analyzer should catch that (though that might already be a runtime or type error depending on language semantics).  

Behind the scenes, many of these checks use classic data-flow analysis techniques:
  - **Live variable analysis** can find unused variables (variables that are never live after definition) – essentially dead store elimination in compilers, reported as a warning to the developer.
  - **Control-flow analysis** identifies unreachable code and checks all exit paths of functions.
  - **Constant propagation or evaluation** on the AST can catch constant conditions or results (e.g., computing simple constant expressions at compile time to see if a branch is always taken). 

We will design these analyses to minimize false positives. **False positives are a critical concern in static analysis** – if the tool reports too many benign constructs as issues, developers may start ignoring it ([](https://sites.tufts.edu/eeseniordesignhandbook/files/2021/05/Madeghe_StaticCode.pdf#:~:text=It%20is%20crucial%20to%20note,3)). For example, if a variable is unused only in certain configurations, or code looks unreachable but isn’t under some runtime condition, we should be careful. We might allow suppression (like a comment `# allow unused` near a variable) so the developer can silence a particular warning if it’s expected. Overall, the rules should be tuned so that when the analyzer flags something, it is very likely an actual problem or at least something worth reconsidering. 

### Static Analyzer Output and Example  
Static analysis findings will typically be reported as **warnings** (they don’t stop compilation but indicate likely issues). However, some could be escalated to errors if they will likely lead to runtime errors (for instance, “all paths not returning a value” might be considered an error in some languages). For consistency, we will treat them as warnings in output, distinct from lint style warnings by category. We might tag these as **“Analysis Warning”** or give them specific codes (e.g., AN001 for unused variable, AN002 for unreachable code, etc.). The output format will be similar to linter output: `file:line:col: Warning [code] message`. 

For example, given the following WFL code in `sample.wfl`:

```wfl
store x as 10
store y as 20
store z as x plus y
display x
if x > 5 then
    display "Large"
    return x
    display "This won't print"
end
```

Running `wfl --analyze sample.wfl` might produce: 

```
sample.wfl:2:1: **Warning (Analysis/Unused)** – Variable `y` is never used.
sample.wfl:5:5: **Warning (Analysis/Unused)** – Variable `z` is assigned but its value is never used.
sample.wfl:8:5: **Warning (Analysis/Unreachable)** – Code after `return` on line 7 is unreachable.
```

Here, line 2 defines `y` which is never referenced, and line 3 defines `z` which is computed but never used (perhaps the coder meant to display z, but forgot). Line 8 has a `display` that will never execute because line 7 already returns from the action. The analyzer flags all these. The developer, upon seeing this, would likely remove the `y` variable, or use it if it was meant to be used; similarly use or remove `z`; and remove or move the unreachable `display` call.

If using the rich diagnostic output, the tool can pinpoint the exact spots. For instance, the unused variable warning might underline the variable name definition, and possibly point out if it was meant to be used. The unreachable code warning can highlight the `return` and the following statement.

**Inconsistent return example:** If we had:
```wfl
define action compute_sign needs number n give back text
    if n > 0 then
        return "positive"
    elseif n < 0 then
        return "negative"
    end
    // no return for n == 0
end
```
The static analyzer would warn that not all paths return a text. The output could be:
```
compute_sign: Warning (Analysis/Return) – Not all paths in this action return a text value (missing return for some cases).
```
In context, it might highlight the end of the function where a return is expected. This prompts the developer to add a `return "zero"` or some text for the case `n == 0`. 

Through such checks, the static analyzer acts like an extended compile-time tutor, catching logical mistakes or oversights. Many of these issues, if not caught, could result in incorrect behavior or difficult debugging sessions at runtime. By addressing them when the code is written, WFL programmers can write more robust scripts. Importantly, all these analysis warnings can be viewed as quality gates – e.g. the project could enforce that there are no analysis warnings before code is merged, similar to how Clippy warnings are often treated in Rust projects ([GitHub - rust-lang/rust-clippy: A bunch of lints to catch common mistakes and improve your Rust code. Book: https://doc.rust-lang.org/clippy/](https://github.com/rust-lang/rust-clippy#:~:text=There%20are%20over%20750%20lints,included%20in%20this%20crate)).

## WFL Code Fixer (Auto-formatter and Refactoring Tool)  
The **WFL Code Fixer** is a tool to automatically improve WFL code by reformatting it to a canonical style and optionally applying safe refactorings for simplicity and idiomatic usage. This tool serves two main purposes: (1) act as an auto-formatter (like Black for Python or rustfmt for Rust) to ensure all code conforms to a consistent layout, and (2) assist developers by making straightforward code improvements, either automatically or via suggestions. Using the code fixer can save developer time and enforce consistency without manual edits, as well as implement some of the linter’s suggestions in a click. 

### Auto-formatting Engine  
Taking inspiration from **Black**, the code fixer will define a single, strict format for WFL code and apply it uniformly. As Black’s philosophy states, by using an automatic formatter, developers “cede control over minutiae of hand-formatting” and in return get speed and consistency, freeing them from nitpicking formatting issues ([Black 25.1.0 documentation](https://black.readthedocs.io/#:~:text=By%20using%20Black%2C%20you%20agree,energy%20for%20more%20important%20matters)). *“Blackened”* WFL code will look the same regardless of who wrote it, making code reviews focus on logic rather than style ([Black 25.1.0 documentation](https://black.readthedocs.io/#:~:text=Black%20makes%20code%20review%20faster,focus%20on%20the%20content%20instead)). 

Key formatting rules to be enforced: 

- **Indentation and Whitespace:** Use 4 spaces per indent level (no tabs). All blocks inside structures (`if`, `loop`, `define action`, etc.) are indented one level more than the block opener. `otherwise`/`elseif` keywords align with the corresponding `if`. Ensure there’s exactly one space between keywords and identifiers (e.g. `store x as 5`, not `store  x as 5`). Remove any trailing spaces on lines and add a newline at EOF if missing. These rules ensure clean, professional-looking code.  
- **Line Breaks and Wrapping:** Ensure that lines are not excessively long (for instance, wrap lines that exceed 100 characters). The code fixer can break long expressions at logical boundaries – after commas, operators, or around keywords – to improve readability. It will also ensure blank lines are used appropriately (e.g., maybe one blank line between function definitions, no multiple consecutive blank lines).  
- **Consistent Keyword Casing:** If WFL keywords are case-insensitive (for example, `IF` vs `if`), choose one style (likely lowercase everything) and enforce it. The formatter will convert all keywords to lowercase for consistency: e.g., `Then` -> `then`, `End` -> `end`. Similarly, it can enforce consistent casing on boolean literals (`YES`/`NO` to `yes`/`no` if those exist as in the symbol table initialization).  
- **Spacing Around Operators:** Ensure that binary operators and keywords have proper spacing. For example, in an expression `a plus b` or `a + b`, ensure there is exactly one space before and after the operator. In assignments (`store x as 5`), enforce spacing around the `as`. Remove any extraneous spaces (like `Counter  >   0` would be fixed to `Counter > 0`).  
- **Delimiters and Punctuation:** Format commas, parentheses, etc., with standard spacing. E.g., after a comma in a parameter list, put a space. Remove spaces before commas or before semicolons (if those existed). Basically mirror common style guides.  
- **Aligning Multi-line Constructs:** If an expression or parameter list spans multiple lines, the continuation lines should be indented in a way that clearly delineates them. The code fixer can handle indentation for multi-line function calls or data structures (if WFL has multi-line constructs like array literals or similar).  

Implementing these formatting rules will involve pretty-printing the AST. We’ll write a **pretty-printer** that takes the AST (with knowledge of the original source for comments) and produces formatted source code. Special care is needed to preserve comments – the formatter should not drop comments. It can attach comments to AST nodes (e.g., comments on the line before a statement can be associated with that statement node) so that when printing, the comments are output in the right place, perhaps with a standardized spacing.  

The code fixer will essentially do what a developer would do by hand when following a style guide, but instantly and accurately. This is analogous to running Black on Python code which reformats everything to the Black style (PEP8 compliance plus Black’s specific rules). As a result, running `wfl --fix` on any WFL codebase would drastically reduce diffs and style discussions: *“Blackened code looks the same regardless of the project you’re reading. Formatting becomes transparent…”* ([Black 25.1.0 documentation](https://black.readthedocs.io/#:~:text=Black%20makes%20code%20review%20faster,focus%20on%20the%20content%20instead)). 

Because formatting should not change code behavior, we will include a verification step: after formatting, the tool will re-parse the code and possibly compare the AST to the original AST (ignoring differences in whitespace and maybe identifier case if case-insensitive). This ensures the formatter didn’t introduce syntax errors or alter semantics. Black uses a similar safety check, parsing the reformatted code to ensure it’s syntactically valid and equivalent to the original ([Black 25.1.0 documentation](https://black.readthedocs.io/#:~:text=Also%2C%20as%20a%20safety%20measure,fast)). We can incorporate that: if the ASTs differ in a significant way (other than cosmetic differences), the tool can alert that formatting may have introduced an issue, or simply refuse to apply a change that would alter semantics.

### Automated Refactoring and Idiomatic Improvements  
Beyond pure formatting, the WFL code fixer can also act on certain lint findings by automatically rewriting code in a more idiomatic or simplified way. This overlaps with the linter’s suggestions and essentially automates them when safe. Some examples of **idiomatic fixes** and refactorings include: 

- **Renaming for Convention Compliance:** If the linter identified identifiers that don’t follow naming conventions, the fixer (with user permission) could rename them throughout the code. For instance, if a variable `Counter` should be `counter`, the tool can systematically replace all occurrences of `Counter` with `counter`. Since it has the AST and symbol table, it knows all references, making this a safe refactor (much like an IDE rename refactoring). This ensures consistency without the user manually changing every instance. (One would likely run `--lint` to see issues, then `--fix` to apply straightforward ones like this automatically.)  
- **Simplifying Conditionals:** Transform verbose conditional constructs into simpler forms when possible. For example, if WFL supports boolean values `yes/no` (true/false), a function returning “yes” or “no” based on a condition could be simplified. Suppose we have: 
  ```wfl
  if is_valid then 
      return yes 
  otherwise 
      return no 
  end
  ``` 
  This essentially returns the value of `is_valid`. The code fixer could recognize this pattern and simplify it to just `return is_valid` (assuming the type system allows that). Similarly, `if cond then X otherwise X end` (both branches do the same thing) can be simplified to just `X` unconditionally outside the if. These are akin to some of Clippy’s lints that suggest simplifying if-else that return booleans or identical values. The fixer can perform these transformations reliably by pattern matching the AST.  
- **Removing Dead Code:** For unreachable code or unused declarations identified by the analyzer, the fixer can optionally remove them. For instance, if a variable is never used, it could delete that declaration (and any assignment to it). If a piece of code is unreachable, it could be commented out or removed. However, such removal might be better left for the developer to confirm (because maybe the developer is keeping it for future reference). Perhaps the safer approach is to comment it out and add a note, or only remove if an explicit flag is given (like `--fix --prune-unused`). Nonetheless, providing an automated way to eliminate truly dead code can help keep the codebase clean.  
- **Applying Consistent Idioms/API Usage:** If the language or standard library offers certain functions or constructs that should be preferred, the fixer can replace the non-idiomatic usage with the idiomatic one. For example, if WFL has a built-in loop construct for iterating a number of times, but the code uses a while loop manually decrementing a counter, the tool might not reliably convert that (since it involves semantics). But simpler cases, e.g., using `repeat ... until false` for an infinite loop vs a dedicated `forever` keyword, could be standardized. Another scenario: if string concatenation can be done with `&` operator (hypothetically) but the user code does it by calling some method repeatedly, the fixer could simplify to `&`. These require knowing quite a bit about the language’s idioms, so they would be added gradually as we identify common patterns. Clippy’s wide range of lints (over 750 lints in categories ([GitHub - rust-lang/rust-clippy: A bunch of lints to catch common mistakes and improve your Rust code. Book: https://doc.rust-lang.org/clippy/](https://github.com/rust-lang/rust-clippy#:~:text=There%20are%20over%20750%20lints,included%20in%20this%20crate))) provides a rich source of ideas – many of those suggestions could be automated for WFL if analogous patterns exist.  
- **Reordering and Organizing Code:** The fixer might also ensure a standard order of elements, such as sorting `import` statements (if WFL has imports) or grouping constant definitions at top, etc., similar to how some formatters and linters (like isort for Python imports ([Python style guide | GitLab Docs](https://docs.gitlab.com/development/python_guide/styleguide/#:~:text=Formatting%20tools))) operate. This makes code structure predictable.  

All automated refactorings will be designed to **not change the meaning of the code** (except to make it clearer or more idiomatic). They should be conservative; where a transformation’s correctness is uncertain, the tool will not apply it silently. In such cases, it might instead produce a **suggestion note** for the developer. For example, if the linter detects “excessive nesting” but an automatic refactor is complex (involving extracting functions or changing logic), the tool can’t realistically do that safely. Instead, it could output a note in the formatted code or as a comment: `# TODO: Refactor to reduce nesting`. The developer would handle it manually. The code fixer’s focus is on mechanical or straightforward improvements.

### Code Fixer Usage and Output  
Using the code fixer is straightforward via `wfl --fix file.wfl`. By default, it could print the refactored code to standard output. The user can redirect this to a new file or use a `--in-place` flag to overwrite the original file with the formatted code. Alternatively, the tool could output a unified diff of changes if invoked with a `--diff` option, so the developer can preview changes. In any case, the output should clearly communicate what changes were made if not writing directly to file. For example, it might list: *“Renamed 2 variables for naming conventions, reformatted indentation, removed 1 unused variable.”* Each category of change can be summarized.

If we apply the code fixer to the earlier example with naming and indentation issues (`example.wfl`), the tool would automatically produce the corrected code as shown after the linter section. It would effectively perform: rename `Counter` -> `counter`, indent the `otherwise` line, and add the missing 4 spaces indent before the second `display`. The output could simply be the fixed code (since it’s short), or a message like:

```
Applied fixes:
- Formatted code with standard indentation (4 spaces).
- Renamed `Counter` -> `counter` to match naming conventions.
```

Then showing the new code:
```wfl
store counter as 5
if counter > 0 then
    display "Positive"
otherwise
    display "Non-positive"
end
```

For a more complex transformation example, consider the function with redundant returns for booleans:
```wfl
action is_even needs number n give back boolean
    if n mod 2 is 0 then 
        return yes 
    otherwise 
        return no 
    end
end
```
The code fixer could detect this pattern and rewrite it as:
```wfl
action is_even needs number n give back boolean
    return (n mod 2 is 0)
end
```
This assumes WFL treats that boolean expression directly as yes/no. The tool would remove the entire if/otherwise structure, replacing it with a single return of the condition. It would then format it properly. The output might inform: “Simplified conditional return in `is_even` to a direct boolean return.”

We will also ensure idempotence of formatting: running the code fixer on already formatted code should result in no changes. This is important to build trust in the tool; developers should not fear that it will keep changing code back-and-forth. Therefore, our formatting rules will be unambiguous. We take cues from Black here, which is deterministic and stable in its output style ([Black 25.1.0 documentation](https://black.readthedocs.io/#:~:text=By%20using%20Black%2C%20you%20agree,energy%20for%20more%20important%20matters)). 

Finally, as a safety measure, any transformation beyond pure formatting (like renaming or removing code) could be guarded behind an extra flag or confirmation. For example, `--fix` might by default do formatting and very safe fixes, but require `--fix --apply-suggestions` to do things like renaming or removal. Alternatively, the tool might output those changes as suggestions (comments or separate report) and require the developer to re-run with a confirm flag. This prevents any surprise modifications. Over time, as we validate the correctness of certain automated fixes, they can be enabled by default.

## Output Formats and Examples  
All three components will produce output in a structured, friendly format, leveraging WFL’s diagnostic reporting system. Below we summarize the output format for each tool with an example scenario:

- **Linter Output:** Reports each style issue as a warning. Format: `[filename]:[line]:[col]: Warning [LintCode] Description.` For instance, running `--lint` might yield:
  ```
  myscript.wfl:10:5: Warning [LINT-NAME] Function name "ComputeSum" should be lower_snake_case.
  myscript.wfl:22:1: Warning [LINT-INDENT] Line is indented 2 spaces, expected 4.
  myscript.wfl:30:20: Warning [LINT-COMPLEX] Nesting depth is 6, which exceeds max of 5.
  ```
  Each warning may include an inline suggestion. The diagnostic system can also show a snippet of code with a marker. For example, the first warning could be shown with the function definition line and a caret under "ComputeSum", and a note suggesting `compute_sum`. This makes it clear exactly where and what the issue is.

- **Static Analyzer Output:** Reports logical issues as warnings (or errors if severe). Format is similar but with different codes or labels (e.g. `[ANALYZE-UNUSED]`, `[ANALYZE-UNREACHABLE]`). Example:
  ```
  myscript.wfl:15:9: Warning [ANALYZE-UNUSED] Variable "tempValue" is never used after assignment.
  myscript.wfl:47:5: Warning [ANALYZE-UNREACH] Unreachable code detected (code after 'return' at line 45 will not execute).
  myscript.wfl:60:1: Warning [ANALYZE-RETURN] Not all paths in function "processData" return a value.
  ```
  These warnings point out potential bugs. Using the diagnostic reporter, the unreachable code warning could highlight the first unreachable statement and perhaps mark the preceding return that causes it. The return-path warning could highlight the function end or the branch that lacks a return.

- **Code Fixer Output:** If run with output to console, it will either show the transformed code or a summary of changes. We have a few modes:
  - **Direct Rewrite (stdout):** The simplest mode where `--fix` just prints the new code. The developer can see the entire formatted file or redirect it. No specific “warnings” in this case, it’s the final code output.
  - **Diff Mode:** If we implement a diff output, it might show something like:
    ```diff
    -store Counter as  5
    +store counter as 5
     if counter > 0 then
         display "Positive"
    -otherwise
    -display "Non-positive"
    +otherwise
    +    display "Non-positive"
     end
    ```
    This clearly indicates what changed: the variable name case, removed double space, and indented the `otherwise` branch.
  - **Summary Mode:** The tool might also print a human-readable summary of what it did:
    - *Renamed 1 variable for naming convention (`Counter` -> `counter`)*  
    - *Reformatted indentation on 2 lines*  
    - *Removed 1 trailing whitespace*  
    If no changes were needed (the file was already perfect), it could say “No changes needed, code is already formatted.”  

Regardless of mode, the code fixer should also leverage the diagnostic style for any non-trivial suggestions it doesn’t auto-apply. For example, if it detects a complex refactor opportunity it didn’t do, it might output a warning or note about it. But primarily, `--fix` modifies code rather than warning about it. 

**Note:** The examples above demonstrate the kinds of output a user would see on their terminal. These messages are designed to be clear and actionable – they either tell the user how to fix an issue (in case of lint/analyze) or show exactly what was fixed (in case of the fixer). Consistent formatting (like the `[TAG]` codes and the use of severity labels) helps in quickly scanning the results. Also, if integrated into an editor or IDE, these messages can be parsed to pinpoint issues in the source file.

## Testing and Validation Strategy  
Building confidence in the code quality suite is essential. We will develop a comprehensive test plan to validate each component’s correctness, usefulness, and stability:

- **Unit Tests for Rules:** Each lint rule and analysis check will have unit tests with representative code snippets. For example, a snippet with a clearly unused variable should trigger the unused-variable warning – the test will run the analyzer on that snippet and assert that the expected warning (with correct message and location) is produced. Conversely, a similar snippet where the variable is used should produce no such warning (to avoid false positives). We’ll craft minimal code examples to exercise each rule: naming conventions (correct and incorrect names), indentation scenarios, nested ifs for complexity, etc. The WFL codebase already has testing infrastructure (e.g., `diagnostics/tests.rs` for error conversions), which we can extend for lint and analysis diagnostics.  

- **Integration Tests on Sample Scripts:** We will create a set of sample WFL programs (covering various language features) and run the entire suite on them. For each sample, we know what issues it contains. For instance, a sample with a mixture of style issues and logical issues will be run with `--lint`, `--analyze`, and we verify the output contains all the expected warnings. We also ensure that nothing incorrect is flagged. These samples could include:
  - A script with an intentionally unused variable, unreachable code, etc., to test analyzer outputs.
  - A script with messy formatting and naming to test the linter and the code fixer’s ability to clean it up.
  - Some large or real-world inspired WFL script to see how the tools perform on it and ensure they scale.  

- **Round-trip Testing of Code Fixer:** For the formatter, we will test that applying `--fix` on code followed by parsing yields the same AST as before (for semantically equivalent code). We’ll set up tests where a snippet is parsed into AST, then formatted, then re-parsed, and assert that the ASTs (ignoring white-space differences) are identical. This ensures our formatting doesn’t change program logic. Additionally, we’ll test that running the fixer twice doesn’t produce further changes (idempotence). If non-idempotent behavior is found, it indicates an inconsistency in formatting rules that we need to resolve. Black’s own test philosophy of having a “comprehensive test suite” and checking formatting stability is a good model ([Black 25.1.0 documentation](https://black.readthedocs.io/#:~:text=Black%20is%20successfully%20used%20by,support%20for%20new%20Python%20syntax)). We aim for similar rigor.  

- **False Positive and Negative Testing:** We will devise tricky scenarios to ensure the analyzer doesn’t report false issues. For example, a variable that is conditionally used (used only in one branch of an if) – the analyzer should not mark it unused. Or code that appears constant but isn’t (maybe reading from an input, etc.). Each such case will be tested. Likewise, ensure the linter rules don’t mistakenly flag acceptable code. For instance, test that a correctly formatted and named code passes without warnings. If any rule is too aggressive, we’ll adjust it. 

- **Performance Testing:** The suite should be efficient. We will test the tools on increasingly large WFL scripts (or generate a large synthetic script) to measure performance. The linter and analyzer mostly do linear or near-linear scans of the AST, so they should scale linearly with code size. We ensure no significant slowdowns (e.g., a 10,000-line script should still lint in a fraction of a second ideally). If needed, we’ll profile the implementation to optimize hot spots. 

- **Real-world Trial:** If possible, test the suite on actual WFL code (perhaps the WFL standard library or any existing WFL scripts if available). This can reveal unanticipated patterns or the need for additional rules. It also serves as a dogfooding exercise. For example, run `--lint` on the WFL standard library modules (`stdlib/*.wfl`) to see if our own code adheres to the rules, adjusting either the code or the rules accordingly. 

- **CI Integration:** The development of this suite will itself be integrated into WFL’s CI. Every commit that changes the linter/analyzer should run all the above tests. We can also include the linter and analyzer in the CI to lint WFL’s own source (if WFL code was self-hosted, but here WFL is implemented in Rust, so not applicable to its own implementation). However, we can integrate something like running `cargo clippy` on the Rust code, since we are drawing inspiration – ensuring our Rust code quality suite is developed with good practices. 

- **User Feedback Loop:** After initial release, we will encourage WFL users to try `--lint` and `--analyze` on their scripts and report if they find false positives or if they think an important lint is missing. This real-world feedback will help refine rule severity and add new rules. Clippy, for example, has categories and allows users to allow/deny certain lints ([GitHub - rust-lang/rust-clippy: A bunch of lints to catch common mistakes and improve your Rust code. Book: https://doc.rust-lang.org/clippy/](https://github.com/rust-lang/rust-clippy#:~:text=There%20are%20over%20750%20lints,included%20in%20this%20crate)); we will consider adding similar categorization if users desire more control (e.g., a “pedantic” category for very strict rules that can be toggled). 

Through these testing measures, we aim to achieve a robust suite where each tool’s output is trustworthy. The end goal is that developers eventually run these tools routinely (or even automatically via git hooks or IDE integration) and come to rely on them to catch mistakes. By borrowing well-established checks from tools like ESLint and Clippy, we start with a solid foundation of rules that are known to be useful. Each rule addition will be justified and tested. Additionally, we’ll document all the lint rules and analysis checks clearly (perhaps in the WFL manual), so users know why a warning appears and how to address it.

## Lessons from ESLint, Clippy, and Black  
In designing this suite, we have consciously learned from existing successful tools:

- **ESLint (JavaScript):** We adopt ESLint’s idea of a pluggable, AST-based rule engine ([Architecture - ESLint - Pluggable JavaScript Linter](https://eslint.org/docs/latest/contribute/architecture/#:~:text=Individual%20rules%20are%20the%20most,the%20AST%20and%20report%20warnings)), where each rule is an independent unit that inspects the AST for a specific pattern. ESLint’s architecture proved that a linter can be flexible and configurable. We also heed ESLint’s recommendation to separate formatting from linting ([Formatters, linters, and compilers: Oh my! · GitHub](https://github.com/readme/guides/formatters-linters-compilers#:~:text=One%20of%20the%20most%20common,when%20compared%20to%20dedicated%20formatters)). Like ESLint, our linter will focus on finding real issues and consistency problems, rather than enforcing every comma and space (that’s for the formatter). We also plan to allow configuration similar to ESLint’s config files (to enable/disable rules or set custom options like max line length). ESLint’s approach to allow inline disabling of rules for exceptional cases is mirrored in our design with comments/pragmas to suppress warnings when necessary. This ensures the linter is strict but not impractical. 

- **Rust Clippy:** Clippy has taught us the value of having a wide range of lints (it has over 750! ([GitHub - rust-lang/rust-clippy: A bunch of lints to catch common mistakes and improve your Rust code. Book: https://doc.rust-lang.org/clippy/](https://github.com/rust-lang/rust-clippy#:~:text=There%20are%20over%20750%20lints,included%20in%20this%20crate))) but organizing them in categories and levels. We will categorize our lints (style, complexity, correctness, pedantic, etc.) so that users can choose to apply all or a subset. Clippy also integrates with the compiler (running as `cargo clippy` uses rustc internals) – similarly, our linter and analyzer integrate with WFL’s compiler front-end to get rich information (AST, type info). From Clippy’s lints, we borrow specific ideas like warning on shadowing, unnecessary clones or returns, etc., tailored to WFL where applicable. Importantly, Clippy presents its findings with helpful messages and often a **“help:”** with a suggested fix. We emulate this by providing suggestions in diagnostic notes (e.g., “help: rename to ...” or “help: you can simplify this by ...”). This makes the warnings more actionable. Clippy also distinguishes levels (Allow, Warn, Deny) and we can incorporate a similar mechanism for WFL if needed (e.g., treat all warnings as errors in a strict CI mode).  

- **Black (Python Formatter):** Black’s influence is seen in our code fixer’s philosophy. We aim for an “uncompromising” formatter that eliminates debates about style by having one definitive style. By using it, developers save time and reduce diffs, just as Black promises speed and consistency ([Black 25.1.0 documentation](https://black.readthedocs.io/#:~:text=By%20using%20Black%2C%20you%20agree,energy%20for%20more%20important%20matters)) ([Black 25.1.0 documentation](https://black.readthedocs.io/#:~:text=Black%20makes%20code%20review%20faster,focus%20on%20the%20content%20instead)). We also adopt Black’s cautious approach: Black checks that the reformatted code is equivalent to the original and only makes changes that won’t break the code ([Black 25.1.0 documentation](https://black.readthedocs.io/#:~:text=Also%2C%20as%20a%20safety%20measure,fast)). We will implement a similar AST equivalence check and refrain from performing transformations that could alter semantics unless we are highly confident in their safety. Black also runs fast and can be used in editors on-save; we will strive for similar performance and ease of integration (perhaps a mode to format code on the fly). Another lesson from Black is to minimize configuration – Black famously has very few knobs (mostly just line length). This reduces confusion. We likely will bake in a standard style for WFL (indent size, etc.) rather than allowing every team to choose their own. This might be opinionated, but it ensures that any WFL code, anywhere, adheres to the same clean style (as long as they use the tool). Consistency is a big win for readability across projects ([Python style guide | GitLab Docs](https://docs.gitlab.com/development/python_guide/styleguide/#:~:text=Formatting%20tools)).

In summary, our WFL code quality suite stands on the shoulders of these tools: ESLint showed how to design lint rules and focus on real issues, Clippy showed the power of catching deeper mistakes and offering friendly fixes, and Black demonstrated the value of automated consistent formatting. By integrating these lessons, we aim to implement the suite “properly” – meaning it will be effective yet user-friendly. The success measure will be that using `--lint`/`--analyze`/`--fix` becomes a natural part of writing WFL code, much like developers routinely run linters and formatters in other languages. This will lead to WFL programs that are not only correct but also clean, idiomatic, and maintainable.

## Conclusion  
This report proposed a comprehensive plan for a code quality suite in the WFL binary, detailing the architecture, rule engines, outputs, and testing approach for each component (linter, static analyzer, code fixer). By embedding these tools directly into WFL’s workflow, we provide WFL developers with immediate feedback and automated improvements, significantly boosting code quality. Adhering to the strict rules and suggestions will result in uniform coding style across projects and early detection of potential errors or bad practices. 

The implementation will proceed in stages: first the core framework and a base set of lint/analysis rules (perhaps the ones discussed), then iterative expansion of rules and refactors guided by real usage and feedback. Throughout, we’ll remain aligned with the philosophies of proven tools (ESLint’s focus on meaningful linting, Clippy’s breadth and clarity, Black’s consistency and safety). The ultimate vision is for WFL’s tooling to rival that of more established languages, making WFL development a pleasant and low-error experience. With the linter catching style issues, the analyzer catching logical issues, and the fixer resolving formatting issues automatically, WFL programmers can focus on the actual program logic, with the confidence that the quality suite has their back in maintaining high code standards.

**References:** 

- ESLint architecture and rule design ([Architecture - ESLint - Pluggable JavaScript Linter](https://eslint.org/docs/latest/contribute/architecture/#:~:text=Individual%20rules%20are%20the%20most,the%20AST%20and%20report%20warnings)) – inspired the AST-based linting approach.  
- ESLint recommended use (avoid using linter for formatting) ([Formatters, linters, and compilers: Oh my! · GitHub](https://github.com/readme/guides/formatters-linters-compilers#:~:text=One%20of%20the%20most%20common,when%20compared%20to%20dedicated%20formatters)) – justification for having a separate code formatter.  
- Rust Clippy’s scope and categorization of lints ([GitHub - rust-lang/rust-clippy: A bunch of lints to catch common mistakes and improve your Rust code. Book: https://doc.rust-lang.org/clippy/](https://github.com/rust-lang/rust-clippy#:~:text=There%20are%20over%20750%20lints,included%20in%20this%20crate)) – demonstrates the value of many targeted rules and tunable lint levels.  
- Importance of minimizing false positives in static analysis ([](https://sites.tufts.edu/eeseniordesignhandbook/files/2021/05/Madeghe_StaticCode.pdf#:~:text=It%20is%20crucial%20to%20note,3)) – guiding principle for rule calibration.  
- Black’s philosophy of uniform style and minimized diffs ([Black 25.1.0 documentation](https://black.readthedocs.io/#:~:text=Black%20makes%20code%20review%20faster,focus%20on%20the%20content%20instead)) – motivates our one-style formatter and its benefits.  
- Black’s safety check by verifying AST equivalence ([Black 25.1.0 documentation](https://black.readthedocs.io/#:~:text=Also%2C%20as%20a%20safety%20measure,fast)) – a practice we will implement to ensure code fixer reliability.