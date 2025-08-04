Great! I’ll create a detailed project plan for building WFL from scratch, written in Rust with the help of AI tools, and tailored specifically for solo development. The plan will focus entirely on technical implementation tasks and will be broken into clear milestones to guide your progress.

I’ll include stages like language design, parser and interpreter development, standard library creation, IDE integration, and error/debug tooling, all in a format that works well with iterative solo development using AI copilots and assistants. I’ll let you know when the plan is ready for review.

# Project Plan for Building the WebFirst Language (WFL) in Rust

This project plan lays out a **milestone-driven roadmap** for a solo developer to implement the **WebFirst Language (WFL)** from scratch in Rust. Each milestone focuses on technical implementation tasks (excluding community or marketing work) and suggests how AI tools (ChatGPT, GitHub Copilot, etc.) can accelerate development. The plan covers all major components of a programming language implementation, from initial design through runtime and tooling. Throughout, we assume continuous testing and iteration at each step, and we leverage Rust libraries (e.g. Pest, Nom, Tokio) where appropriate for efficiency.

## Milestone 1: Language Design Foundations

**Description:** Establish a solid foundation by defining WFL’s syntax, semantics, and overall philosophy **before writing any code**. Given WFL’s unique goals (natural-language syntax, minimal symbols, strong types, web-first capabilities), the design phase is critical.

- **Define Syntax and Grammar:** Outline WFL’s syntax in a human-readable specification. Embrace an *English-like, natural-language syntax* (inspired by languages like Inform 7 or Elm) with minimal special characters, per WFL’s guiding principles. For example, decide how typical constructs will look (e.g. using words like “**let** age **be** 25” instead of `age = 25`, or “**if** X **is** true **then** ...” instead of symbolic syntax). Create an EBNF or PEG grammar capturing these rules. *AI Assistance:* Use ChatGPT to brainstorm grammar rules and catch ambiguities – for instance, ask it to generate example code snippets in an English-like style and refine your grammar based on those. This helps ensure the grammar covers intended constructs and is unambiguous. Copilot can assist by suggesting grammar definitions or repetitive rule structures once you provide examples.

- **Define Semantics and Type System:** Decide how WFL’s semantics will work. Determine if it’s statically typed (per guiding principles, WFL emphasizes *strict type checking with type inference*). Describe the primitive types (number, text, boolean, etc.), compound types (collections, custom structures), and whether the language supports concepts like classes or only functions. Define how functions, variables, and scoping work (e.g. block scope, global scope rules), and how **type inference** will apply (e.g. “The button is clickable” might imply a custom `Button` type automatically). Also design the **memory model**: will WFL be garbage-collected, reference-counted, or rely on Rust’s memory safety for most structures? Given WFL is high-level and web-oriented, a garbage-collected or managed-memory model might be appropriate (to simplify for users). *AI Assistance:* ChatGPT can help compare approaches (e.g. “What are the pros/cons of using a GC vs. Rust’s ownership for a high-level language runtime?”) and even suggest semantic rules (like how to handle asynchronous semantics with keywords like "wait for ... then ..."). Use these insights to finalize decisions.

- **Establish Guiding Principles in Implementation Terms:** Translate WFL’s guiding principles into concrete requirements for implementation. For example, **natural-language syntax** implies the parser must handle multi-word tokens or keywords (“**open file**” as a phrase), and **minimal symbols** means the lexer must treat most text as identifiers/keywords rather than punctuation. “**Clear and actionable error reporting**” means the compiler architecture needs to carry rich context (source spans, hints) to produce user-friendly messages (similar to Elm or Rust’s error messages). **Interoperability with web standards** suggests that the language should ultimately compile to or interface with JavaScript/WebAssembly, which will influence the compiler back-end design. Enumerate these needs now to guide the architecture in later milestones. *AI Assistance:* Use ChatGPT to review the list of principles and brainstorm how each could impact the design. For instance, ask *“Given a goal of Elm-like error messages, what data structures should a compiler store to generate helpful errors?”* This can yield ideas like storing an AST node’s source text for later use in errors.

- **Draft Example Programs:** To validate your design, write a few sample WFL programs in the *proposed syntax*. For instance, a simple “Hello World” page update, an if/else example, a loop or asynchronous call (e.g. “Wait for the server response, then show it”). These examples will test whether the syntax feels natural and whether the semantics cover common scenarios. They will later serve as test cases. *AI Assistance:* Prompt ChatGPT to **simulate being a user of WFL**, writing code for certain tasks (like “connect to a database and query users” in natural language style). This helps identify if any language feature is missing or if syntax could be confusing. Refine the design from this feedback.

**Deliverable:** A language specification document describing WFL’s syntax (with a tentative grammar), core semantics, and design decisions. This will guide all implementation steps. Save this as a reference (e.g., `docs/WFL_Spec.md`) to consult throughout development. 

## Milestone 2: Project Setup and Tooling

**Description:** Set up the Rust project structure and development tools. Establishing the right scaffolding early will make the implementation smoother. This milestone also includes selecting libraries and setting up testing frameworks.

- **Initialize Rust Workspace:** Create a new Rust project (e.g., with `cargo init wfl`). Organize it into logical modules: e.g. `lexer`, `parser`, `ast`, `typechecker`, `interpreter`, `runtime`, `repl`, etc. This modular structure will allow working on components in isolation and easier testing. *Tool:* Leverage Cargo’s workspace if splitting into sub-crates (for example, a core library crate vs. a binary for the CLI/REPL). Use `rustfmt` and `clippy` from the start for consistent style and catching issues.

- **Library Selection:** Decide on parser/lexer libraries or confirm a plan for hand-written parsing. The Rust ecosystem offers powerful libraries like **Pest**, **Nom**, **LALRPOP**, or **rust-peg** for parsing ([Building a Rust parser using Pest and PEG - LogRocket Blog](https://blog.logrocket.com/building-rust-parser-pest-peg/#:~:text=Currently%2C%20there%20are%20several%20parser,are%20LalrPop%2C%20Nom%2C%20and%20Pest)). Given WFL’s natural-language syntax, a PEG-based parser like Pest or rust-peg might be very suitable, as it allows writing grammar rules that can handle “wordy” syntax easily. Pest, for example, lets you define grammar in a separate file and will generate a parser for you ([Building a Rust parser using Pest and PEG - LogRocket Blog](https://blog.logrocket.com/building-rust-parser-pest-peg/#:~:text=but%20can%20also%20be%20used,to%20parse%20strings)). Alternatively, Nom (parser combinators) could be used if you prefer code-driven parsing; however Nom often shines for binary or highly performant needs, whereas readability and flexibility might be more important here. *Task:* Evaluate these options by prototyping a small grammar snippet (e.g., just for an `if` statement or a declaration) in Pest vs. Nom to see which is more readable for WFL’s style. If using Pest or rust-peg, add it to `Cargo.toml` and set up the basic file structure (e.g., a `grammar.pest` file). If using Nom, prepare utility functions for parsing (Nom will be pure Rust code). *AI Assistance:* Ask ChatGPT about the experiences of using Pest vs Nom for natural language-like grammars. It may highlight ease of grammar changes with Pest, which could be valuable as WFL’s syntax evolves. Also, Copilot can suggest Nom parser combinators or Pest grammar rules as you start writing them.

- **Testing Framework:** Set up a testing strategy. Create a `tests/` directory or use Rust’s built-in test module conventions. Plan to write unit tests for each component (lexer tests, parser tests for specific snippets, typechecker tests for valid vs. invalid programs, etc.). You might integrate a snapshot testing library for comparing multi-line outputs (e.g., for error message formats). Also consider using property-based testing (e.g. the `proptest` crate) for the parser (to feed random input and ensure it either parses or fails gracefully). *AI Assistance:* Use ChatGPT to generate some initial test cases from your example programs. For instance, it can help produce a variety of small code samples to test each grammar rule or each type rule. Copilot will also auto-complete test functions when you describe the scenario.

- **Continuous Integration Setup (optional):** If desired, set up a simple CI (GitHub Actions or similar) to run tests on each commit. This isn’t a product feature, but ensures technical quality. This can be done later as well, but having it early will catch regressions in each milestone. AI tools are less needed here, but Copilot can help write a CI YAML file if asked.

**Deliverable:** A working Rust project scaffold. At the end of this milestone, running `cargo build` should succeed (though it does nothing significant yet), and running `cargo test` should run the (initially empty or basic) test suite. You will also have decided on the parsing approach and included the necessary crates (e.g., Pest, Nom, etc.) in the project.

## Milestone 3: Lexer (Tokenizer) Implementation

**Description:** Build the lexical analysis component that converts raw source code (WFL text) into a stream of tokens. The lexer (tokenizer) handles character-by-character reading, grouping characters into meaningful tokens (keywords, identifiers, numbers, strings, etc.), which is the first stage of any compiler. Even if using a parser generator (like Pest which can handle lexing internally via grammar rules), it’s important to define the lexical structure clearly.

- **Define Token Types:** Enumerate all the token categories in WFL. Typical tokens include identifiers (which in WFL may be multi-word identifiers or keywords like “open file”), string literals (e.g., text in quotes), numbers, punctuation (if any, e.g. maybe period `.` or comma if WFL uses them in syntax), and keywords. WFL’s principle of minimal special characters means there will be few symbolic tokens. For instance, arithmetic might allow both word forms (“plus”) and `+` symbol ([wfl-foundation.md](file://file-2R4kWv6kRZFxzrEzf6aiTM#:~:text=Description%3A%20Eliminate%20special%20characters%20%28e,and%20more%20approachable%20for%20newcomers)); decide if both are tokens or if one is translated to the other. Define an enum in Rust for `Token` with variants for each type (e.g. `Identifier(String)`, `Number(f64)` for numeric literals, `StringLiteral(String)`, `KeywordIf`, `KeywordOpen`, etc.). This provides a structure for the parser to consume.

- **Implement the Lexer Logic:** Write functions to read the source input and produce the sequence of tokens. If using **Pest or rust-peg**, a lot of lexing is handled by the grammar (you define regex-like rules for tokens). In that case, focus on writing the grammar’s *lexical rules* (Pest allows defining custom *patterns* for things like ASCII letters, etc.). If writing the lexer manually or with a library like **Logos** (a Rust lexing library), implement it accordingly. Key things to handle:
  - Skipping whitespace and comments (if WFL supports comments, e.g. lines starting with `#` or `//` – decide comment syntax in design).
  - Recognizing multi-word constructs: This is tricky. For example, “open file” might be two tokens (`OpenKeyword`, `FileKeyword`) or a single composite token. Likely, it’s easier to tokenize them separately and let the parser interpret sequences. So the lexer might just output `Identifier("open")`, `Identifier("file")` etc., unless you choose to treat certain phrases as one token. The design decision from Milestone 1 influences this.
  - Handling string literals (e.g. `"Hello world"` should be one token with value `Hello world`). Ensure escape sequences (if any) are handled (e.g. `\"` for quotes inside strings, newline characters, etc.).
  - Numbers: handle integer vs float, maybe allow underscores for readability? (depending on design).
  - Special symbols: If symbols like `+ - * =` are allowed, tokenize them as separate variants (but many are discouraged in WFL by design, aside from maybe `=` in “equals” or using word “equals” instead).
  
  *AI Assistance:* Copilot can be very handy here for boilerplate. As you start writing a loop to iterate over chars and match patterns, Copilot can suggest code for recognizing identifiers or numbers. You can also use ChatGPT to validate your token definitions – e.g., provide it a snippet of WFL code and your token list, and ask “Does this tokenization make sense? Any ambiguities?” It might catch edge cases (like a word that could be both a variable name or a keyword).

- **Use of Libraries:** If using Pest, write lexical rules (Pest rules like `WHITESPACE = _{ " " | "\t" | "\n" }` to skip whitespace, etc.). If using Logos, derive a Logos lexer on the Token enum with regex patterns for each token. For example, you might define `#[regex("[0-9]+", callback)]` for numbers, `#[regex("\"[^\"]*\"")]` for strings, and a catch-all for words. *Potential Pitfall:* Natural-language syntax means many keywords are actually English words (“open”, “read”, “write”, etc. could all be tokens). You’ll need to decide on a set of reserved words vs. words that can be identifiers. For instance, “open” as a verb likely is a reserved keyword in context of I/O operations. Maintain a list of reserved keywords in the lexer (e.g., map specific words to distinct token variants like `KwOpen`), while anything not reserved but matching word pattern becomes a generic `Identifier`. This approach is similar to many language lexers.

- **Testing the Lexer:** Create unit tests feeding small strings to the lexer and verifying the token output. For example, test single-word tokens, sequences like `let age be 25` should produce [`KwLet`, `Identifier("age")`, `KwBe`, `Number(25)`] or similar. Also test boundary cases: string with escaped quotes, an unclosed quote (should error), a number followed by text, etc. *AI Assistance:* Use ChatGPT to generate some tricky lexical scenarios (like “a string with a quote inside, an identifier with digits, etc.”) and ensure your lexer handles or rejects them properly. If using Pest, it will report errors for invalid tokens; verify those errors are understandable (later you’ll integrate a nicer error mechanism).

**Deliverable:** A fully functional tokenizer. By the end of this milestone, you should be able to feed a WFL source string into a lexer function (or the parser, if using integrated parsing) and get a sequence of tokens out. This will set the stage for parsing. You should also have a suite of lexer tests passing. If you run the WFL compiler in a debug mode to just print tokens, you can see the source broken down correctly.

## Milestone 4: Parser and AST Construction

**Description:** Develop the parser that takes the stream of tokens from the lexer and produces a structured **Abstract Syntax Tree (AST)** representing the program’s structure. The AST will be the basis for semantic analysis (type checking) and execution. WFL’s parser will likely be more involved due to its natural-language flavor, but using the grammar from design and possibly parser generators will help. This milestone includes building the AST data structures and basic error handling for syntax errors.

- **Define AST Data Structures:** Design Rust structs/enums for the AST nodes of WFL. Each language construct (as designed in Milestone 1) should have a representation. For example:
  - `AstNode` (enum) with variants like `LetStatement { var: String, value: Expr }`, `ExprStatement(Expr)` (for standalone expressions), `IfStatement { condition: Expr, then_block: Block, else_block: Block }`, `FunctionDef { name: String, params: [(String, Type)], body: Block, return_type: Type }`, etc., depending on what WFL supports.
  - An `Expr` enum for expressions: e.g. `BinaryOp(Box<Expr>, OpKind, Box<Expr>)`, `Call(Box<Expr> /* func */, Vec<Expr> /* args */)`, `Literal(LiteralValue)`, `Variable(String)`, etc.
  - A `Block` could be a Vec<AstNode> or a specialized type.
  - For I/O and special constructs in WFL’s design (like “open file and read”), decide how to encode them. Perhaps as function calls or as distinct AST nodes. You could treat `open file "path"` as a library call under the hood (parsed as something like `Call(Identifier("open_file"), [Literal("path")])`) or as a special AST node `OpenFileStmt(path)` initially. Define nodes for asynchronous operations if syntax like “wait for X then Y” exists (maybe an AST node `Await(expr)` or so).
  
  *AI Assistance:* ChatGPT can help by reviewing your planned AST structure – e.g., provide it the list of language constructs and ask if the AST covers all or if it might need adjustment. It can also suggest modeling certain things in simpler ways. Copilot will autocomplete a lot of the Rust struct definitions once you start writing them.

- **Implement the Parser:** If you chose a parser generator (Pest, LALRPOP, rust-peg), write the grammar rules to parse the tokens into the AST. If doing it manually or with Nom, implement recursive descent functions. Key tasks:
  - **Grammar Rules:** Write rules for top-level definitions (variables, functions), and for statements and expressions. For WFL, an example grammar rule might be:  
    ```pest
    IFStmt = { "if" ~ Condition ~ "then" ~ Block ~ ("otherwise" ~ Block)? }
    ``` 
    (assuming “otherwise” as an else). In code, this would construct an `IfStatement` AST node. Ensure the grammar handles optional parts (e.g., else may be optional) and multiple ways to express something if WFL allows synonyms (like “otherwise” vs “else” if both were allowed – probably choose one for consistency).
  - **Operator Precedence:** If the language has expressions with operators (like arithmetic or comparisons), ensure the grammar or parser code handles precedence (e.g., multiplication vs addition order, etc.). In PEG (Pest), you might break expressions into precedence levels or use the **infix notation** features if available. In a manual parser, you’d implement precedence by recursive descent (e.g., `parse_expression` calls `parse_term` etc., or use the Pratt parsing technique). Given WFL’s preference for words, you might have both word-operators and symbol-operators (e.g., “plus” and “+” should behave the same). You can decide to normalize tokens (the lexer could tag “plus” as a Plus operator token just like `+`).
  - **Error Handling:** Plan how to report syntax errors. Parser generators often produce an error with a position when the input doesn’t match grammar. If writing manually, you can detect unexpected tokens. Make sure to capture the line/column (perhaps carry a reference to the source text or have the lexer include positions in token structs). At this stage, the error messages can be basic (“Syntax error at line X: unexpected token Y”), as a more user-friendly error system will come in a later milestone. But structure your parser to propagate errors (use Rust `Result<AstNode, ParseError>` types). Define a `ParseError` type carrying location info. This will feed into the error reporting system later.

  *AI Assistance:* Use ChatGPT to help craft tricky grammar rules. For instance, ask it to express some English-like constructs in PEG. It might output something close that you can refine. If using LALRPOP, it might help with writing the `.lalrpop` file structure. Also, if you encounter shift/reduce conflicts or PEG ambiguities, you can describe them to ChatGPT for advice. Copilot can often fill in grammar productions or match arms of a parsing function once it learns your AST types (e.g., it might suggest how to parse an `IfStatement` after you define the AST node).

- **Build the Parse Tree to AST:** Ensure that the parser actually constructs the AST nodes using the data structures. For example, in Pest you will get a parse tree (pairs of rules) and you’ll need to traverse it to build your AST. Write functions to convert Pest’s output into your AST types. If using Nom or manual parsing, you’ll be directly creating AST nodes in code as you recognize grammar patterns. Keep this well-structured – e.g., a separate module `ast_builder` if needed – so it's easy to modify if the grammar changes.

- **Incremental Testing:** After implementing parts of the parser, test them. Parse the example programs from Milestone 1 to see if they produce the correct AST. Write unit tests for specific grammar pieces: e.g., ensure an `if ... then ... otherwise ...` string yields an AST with IfStatement node and correct children. Also test error cases (e.g., “if true then” without an end or else – the parser should error). It’s useful to have pretty-print or debug print for AST nodes to inspect them. Derive `Debug` on AST types or implement a simple pretty-printer.

**Deliverable:** A working parser that can take WFL source code and output an AST (or indicate syntax errors). At this stage, you have a basic **compiler front-end**: source code -> tokens -> AST. This is a big milestone; once done, you can parse WFL programs (but they won’t run yet). All subsequent stages will operate on the AST. The milestone is complete when you can successfully parse non-trivial WFL source examples into an in-memory AST, verified by tests. The parser error messages might be rudimentary, but they should pinpoint location of issues.

## Milestone 5: Semantic Analysis and Symbol Resolution

**Description:** Now that we can parse programs, the next step is to analyze the AST for correctness beyond syntax. This includes building the symbol table (tracking variables, functions, their definitions) and resolving identifiers to their declarations, as well as checking for semantic errors like using an undefined variable. This stage sets the groundwork for type checking.

- **Symbol Table & Scopes:** Implement a mechanism to handle scopes (e.g., global scope, function-local scopes, block scopes if any). Likely, create a structure like `SymbolTable` or utilize nested Rust `HashMap`/`BTreeMap` to map identifier names to information (like what kind of entity and possibly type). Traverse the AST and at each new scope (e.g., entering a function definition or a block), push a new scope context, and pop when leaving. Mark declarations:
  - When a `Let` statement (variable definition) is encountered, add it to the current scope’s symbols (and error if it’s redefining an existing name in the same scope accidentally).
  - When a function is defined, add it to the global scope symbol table.
  - Also handle built-in names (if WFL has any built-in identifiers for standard library; maybe not yet at this stage).
  - For each identifier usage (e.g., a variable in an expression), resolve it: find which scope has it. If not found, that’s a semantic error (undefined variable). Gather these errors in a list or structure.

- **Constancy and Mutability (if applicable):** Decide if WFL distinguishes mutable vs immutable variables (the design might not mention it explicitly; perhaps all variables can be reassigned or maybe they wanted a more declarative approach). If there is a concept of constants or final variables, incorporate that in symbol info, and check that rules are followed (e.g., if reassigning a constant, produce error). If WFL uses natural phrases like “set X to Y” for assignment after declaration, ensure the parser recognized that and you reflect it here.

- **Name Resolution for Function Calls:** If the AST has nodes for function calls or method calls, ensure the function name is resolved to a symbol (and perhaps annotate the AST node with a reference or pointer to the function’s symbol table entry). This will be useful in type checking and later during code generation.

- **Prepare for Types:** You can start annotating the AST with type placeholders if that helps (e.g., add an optional `Type` field in expression nodes to fill in later, or maintain a separate map from AST node IDs to types). Sometimes compilers use an annotated AST or a parallel structure for after type checking. A simple way is to have a struct like `TypedExpr` that extends `Expr` with a type, but since we’re in Rust, carrying types during checking might be done via an environment mapping node IDs to types. Consider how you will propagate type info. At minimum, ensure the symbol table entries for variables and functions carry their declared type (if given) or a placeholder if to be inferred.

- **Semantic Error Reporting:** For now, accumulate errors such as “variable X not defined” or “function Y called with wrong number of arguments” (argument count can be checked here if function signature is known). Structure these errors with location info (you have the AST node, which should carry span info from parsing – if not, modify AST to include a Span or reference to source for each node). Don’t worry about making the error message extremely friendly yet, just ensure you have enough info (e.g., variable name and line number) to later format a nice message.

  *AI Assistance:* ChatGPT can provide guidance on how to implement a symbol table in a compiler. It might suggest using an environment stack. It can also simulate test scenarios: “what if a variable is used outside its scope” – ensure your logic catches that. If you’re unsure about lifetimes or ownership in Rust for the symbol table (because AST might hold references), ChatGPT can help design around that (perhaps store just names and look them up by name each time, which is simpler but less efficient, fine for now). Copilot can speed up writing the traversal code: as you write a recursive `fn resolve_names(node: &mut AstNode, env: &mut Env)`, Copilot might fill in arms for different node types based on AST definitions.

- **Test Semantic Analysis:** Write tests to validate this stage. For example, a small program that uses a variable before declaration should trigger an error. A program with a proper let binding and usage should pass without error. If possible, add an option to run the compiler up through semantic analysis and return errors for a given input. Verify that correct programs produce no semantic errors list.

**Deliverable:** The AST is now annotated or accompanied by symbol resolution results. You have a symbol table implementation and a name resolution pass that can report undefined names or similar issues. At this point, running the compiler on input code will either produce a decorated AST (with scope info, etc.) or a list of semantic errors. This is the stepping stone to the type checking. No user code is executed yet, but the code is semantically validated.

## Milestone 6: Static Type Checker

**Description:** Implement the static type checker for WFL. This component will ensure that operations in the AST are type-safe: e.g., you can’t add a number to a text string unless such an operation is defined, function calls match their signatures, etc. It will enforce WFL’s **strict type checking with type inference** principle ([wfl-foundation.md](file://file-2R4kWv6kRZFxzrEzf6aiTM#:~:text=5)), meaning the language should catch type errors at compile time while allowing developers to omit explicit types where obvious.

- **Type Definitions:** First, represent types in the compiler. Define a `Type` enum/struct to model WFL types (primitive types like Int, Text, Bool, maybe more complex like List of T, or user-defined types if any). If WFL allows user-defined structured types or classes, include a way to represent those (e.g., a symbol entry for a type name and its fields, or simpler, maybe WFL initially only has basic types and the “web” types like elements, etc.). Include a special `Type::Unknown` or `Type::Infer` to use during inference before a type is determined, and perhaps a `Type::Error` to mark an expression that already failed (to avoid cascading errors). If the language has generics or polymorphism, that would add complexity (likely not in initial version, unless needed for certain features).

- **Type Inference Algorithm:** Decide on the strategy for type checking/inference. A simple approach is a single-pass type checking where types are propagated down or up as needed (like how Rust does it to some extent, but Rust’s is complex; for WFL it might be simpler). For example, in an assignment `let age be 25`, if no type is declared for `age`, you infer it as Number from the literal. Or in `let x be none`, maybe infer a special None type or a generic? Identify if WFL has `null/nil/undefined` concept – possibly not explicitly mentioned, but maybe “none”. Ensure the type system can handle that if needed (e.g., Option types or a bottom type). If there are function calls, and the function has a known return type, that gives the call expression a type. If a function’s parameter types are known, check that the argument expressions match those types (recursively type-check arguments first).

  A common approach is *unification* for type inference (like Hindley-Milner algorithm) if the language has type inference beyond simple cases. If WFL is like Elm (which has full type inference), you may implement a constraint solver: traverse AST collecting type constraints (e.g., `lhs.type must equal rhs.type for an assignment`, or for binary `+`, both sides must be Number, etc.), then solve those constraints. However, given this is a solo project, you might simplify: for each construct, determine types in place:
  - For each variable declaration, if an explicit type annotation is present (maybe WFL syntax allows “Let X be 5 as Number”? or maybe not; if not, then always infer from value).
  - For expressions, have functions like `type_of_expr(expr) -> Type` that recursively compute types. Use the symbol table to get declared types of variables or function return types.
  - If a type can’t be determined immediately (e.g. function with no return annotation and not used in a way to infer), you might need a two-phase approach or require annotations for those.
  
  *AI Assistance:* ChatGPT can guide on implementing type inference. It might suggest focusing on a subset (maybe require type annotations for function parameters and let inference fill the rest, similar to TypeScript’s approach). If the type system is too complex, ChatGPT can help narrow it to something feasible (e.g., no generics initially, etc.). Copilot can assist writing lengthy `match` statements for type checking each AST node variant.

- **Type Checking Rules:** Implement checks for each AST node:
  - **Binary Operations:** Ensure operands have appropriate types. For instance, if `+` is used, both sides should be Number (or one or both could be Text if you allow string concatenation with `+`? Define that in design). If types mismatch, record a type error (“Cannot add Number and Text”, for example).
  - **Assignments:** The value’s type must be assignable to the variable’s declared/inferred type. If a variable is declared without type and we are inferring, set the variable’s type now based on the value (and possibly allow it to change if a later assignment contradicts? Ideally no, WFL should probably not allow changing type of a variable after init).
  - **Function Calls:** Check that the number of arguments and their types match the function’s parameter types. If the function returns a type, mark the call expression as that type. If the function is defined after usage (forward references), ensure you handle that (maybe one pass collects all function signatures first).
  - **Control Flow:** For an `if` expression (if WFL’s if returns a value, like an expression, e.g., “result = if cond then 1 otherwise 0”), ensure both branches have compatible types and set that as the type of the if-expression. If WFL’s if is statement-only, then ensure the branches individually type-check. Similarly, for loops or other constructs.
  - **Asynchronous constructs:** If WFL has syntax like “wait for X then Y”, treat it similarly to an `await`: the `X` should be of an async task type (like a Future or Promise type), and the result of the whole expression inherits the result type of X once awaited. You might model this by having a special Type constructor like `Type::Async(Box<Type>)` meaning “an async task yielding Type”. The `wait for X then Y` would require X’s type is `Async(T)` for some T, and then inside the `then` block Y can use a value of type T (like the resolved result). The implementation of this might not be fully realized until runtime, but statically you can ensure only awaitable things are awaited, etc.

- **Type Environment and Mutability:** As you check, update the environment. For instance, once you determine `age` is a Number, store that in the symbol table entry for `age` so further uses of `age` know its type. If there’s type inference that needs whole-program solving (which can happen if, say, a function’s return type isn’t annotated, you might not know it until you see how it’s used), you may simplify by requiring explicit return types for functions initially or doing a second pass. A practical approach: 
  1. Infer as much as possible in one walk.
  2. If any types remain unknown, either default them (like unknown numeric types become Number, etc.) or error that inference failed.
  
- **Accumulate Type Errors:** Create a list of type errors similar to semantic errors. Examples: “Type mismatch: expected Number but found Text in addition”, “Function foo expects Text argument but got Bool”, etc. Keep the error objects with span info and possibly the expected vs found types for later messaging. Aim to not crash on first error – continue checking to report all errors in one run, if possible.

  *AI Assistance:* After implementing, test via ChatGPT by describing a scenario: e.g., “if I have `let x be 5; set x to \"hello\"`, what errors does my checker produce?” It might help verify that you indeed catch that type change. Or, ask how to handle a particular pattern (like inferring types in a conditional expression). ChatGPT can also suggest test cases: “function with missing return, variable used in arithmetic with wrong type,” etc. Copilot will be helpful writing the code that patterns through AST nodes.

- **Testing Type Checking:** Use the example programs and new examples to test. For instance, a snippet `let x be 5; let y be "hi"; x plus y;` should yield a type error on the `x plus y` expression. A correct program `let a be 1; let b be 2; let c be a plus b;` should infer c as Number and have no errors. Also test the asynchronous usage: if your design has `wait for fetchData from server then let result be it`, check that if `fetchData` is considered Async<Text>, then `result` is Text, etc. Gradually build confidence that the type checker enforces rules and infers types where obvious. Every let-bound variable should end up with a type.

**Deliverable:** A robust static type checker. By completion, the compiler will reject ill-typed programs with appropriate error messages and accept correct programs, annotating the AST with type info (or a separate structure carrying types of each expression). You likely have a function like `check_types(ast: &mut Ast) -> Vec<TypeError>` that you can call after parsing. Achieving this means WFL now has a proper front-end: it can parse and type-check code. The next step is to execute or compile that code.

## Milestone 7: Interpreter Implementation (Execution Engine)

**Description:** Build an **interpreter** for WFL so that programs can actually run. An interpreter will walk the AST and execute it directly. This is the fastest way to get a runnable language as opposed to writing a full compiler to machine code. (We will consider a compiler to WebAssembly/JS in a later milestone for web integration, but an interpreter helps validate the language semantics quickly.) The interpreter will also handle runtime behaviors like I/O by calling Rust functions or libraries under the hood.

- **Runtime Value Representation:** Define a Rust enum for runtime values of WFL, e.g.:
  ```rust
  enum Value {
      Number(f64),
      Text(String),
      Bool(bool),
      // ... any other primitive type
      List(Vec<Value>),
      Object(HashMap<String, Value>), // if needed for structures
      Null, // maybe for "none"
      Function(FuncHandle), // if supporting first-class functions or closures
  }
  ```
  The `Function` variant could store a reference or pointer to a user-defined function’s AST/bytecode or a native function implementation. You might also have a variant for external things like file handles, but those could be handled as opaque objects in Value (e.g., `Resource(…)`). The goal is to have a type that can hold any WFL value at runtime (since Rust is statically typed, using an enum like this is typical for interpreters).

- **Evaluate Expressions:** Implement an evaluator that takes an AST `Expr` and returns a `Value`. Write it as a recursive function, e.g. `fn eval_expr(expr: &Expr, env: &mut RuntimeEnv) -> Result<Value, RuntimeError>`. The `RuntimeEnv` here is an environment mapping variable names to `Value`s at runtime (basically the analog of symbol table but now with actual values). This environment will be nested for scopes: e.g., have a structure to represent call stack frames (each function call gets a new env frame). Key things to handle:
  - **Literals:** Straightforward (Number literal -> `Value::Number`, etc.).
  - **Variables:** Look up the variable’s value from the environment (which was set when the variable was defined).
  - **Binary Ops:** Perform the operation on the two operand values. E.g., for `+`, expect both to be Number (the type checker guaranteed type-safety, so you can assume types match, maybe assert or handle if not as a panic or runtime error if something slipped through). Produce a `Value::Number` sum. If you implement string concatenation, if `Value::Text` plus `Value::Text`, concatenate. Similarly for other ops.
  - **Comparisons and Boolean logic:** Implement `<, >, ==` etc., returning `Value::Bool`. Possibly allow comparing Text for equality, etc.
  - **Function Calls:** This is substantial. When encountering a call, you need to:
    1. Determine if it’s a **built-in function** (like print, or an I/O operation) or a **user-defined** function.
    2. If built-in, execute the corresponding Rust code (for example, if calling a built-in `print(text)`, then print to console; if calling a built-in `open_url(...)`, handle via Rust HTTP library, more on I/O in next milestone).
    3. If user-defined, retrieve the function’s AST (you need to have stored AST or pointer for functions in a table when they were defined). Then create a new environment for the function’s scope, map the passed arguments (as Values) to the function’s parameter names, and execute the function body by evaluating each statement in it. If a return value is expected, capture it when a return statement is encountered (you might implement `ReturnValue` as a special control-flow exception or have `eval_block` return an Option<Value> to represent a return).
    4. Handle recursion and make sure to avoid infinite recursion (though that’s user’s problem mostly) – but ensure your interpreter doesn’t leak memory in recursion (Rust will manage stack for Rust frames, but WFL function frames are within your Value env structures).
  - **Control Structures:** For an `if` statement node, evaluate the condition expression, check if truthy (if `Value::Bool` is true). Based on that, choose then-block or else-block to execute (by calling eval on that block’s statements). For loops (if WFL has loops like “repeat X times” or “while”), implement accordingly (likely as AST nodes with a condition or range, etc.). For each loop iteration, you may need to manage a break condition – possibly implement `RuntimeError::Break` or a special control flow signal if WFL has a “break out of loop” concept.
  - **Asynchronous constructs:** This is a crucial part given WFL’s web-first nature. However, implementing true concurrency in a straightforward interpreter is tricky. We will integrate with Tokio in the next milestone for actual async, but here, consider how to represent an async function call or a “wait”. One approach for the interpreter: when encountering `wait for X then Y`, you could:
    - Evaluate X (which should produce a `Value::AsyncTask` perhaps).
    - Actually kick off the async task in the background using Rust async (Tokio). This likely means the interpreter function itself might need to be `async` to `.await` Rust Futures. Alternatively, you yield control to the runtime and resume later. To simplify, you might initially *synchronously block* on the async operation just to get something working (e.g., if X is an HTTP request, perform it synchronously). But that’s against the spirit of async. Better: design the interpreter such that certain operations return a **suspended state** or register a callback.
    
    A simpler strategy: Use Rust’s async in the interpreter: make `eval_expr` an `async fn` that returns a `Future`. Then for an `await` (WFL "wait for"), you `.await` the inner future. This will integrate naturally with Tokio runtime when running the interpreter. Essentially, treat WFL async like Rust async under the hood. This requires restructuring evaluation as async, which might cascade changes (like many eval functions need to be async). We will focus on this integration next milestone. For now, mark where you need to await or spawn tasks.
  - **Error Handling at Runtime:** If something goes wrong (division by zero, calling something not callable, etc.), return a `RuntimeError`. Define `RuntimeError` with information and possibly span to pinpoint where it happened. These will later be turned into nicely formatted errors. The static type checker should prevent most type-related runtime errors (so runtime errors are mostly things like division by zero or user-triggered errors).

- **Integrate with Symbol Table:** At runtime, you will use a separate environment for values, but you can utilize the symbol table from compile-time to know function definitions, etc. Potentially, when you resolved symbols in the AST, you could annotate AST nodes with indexes or IDs to refer to symbol table entries – then at runtime, you can have a parallel structure mapping symbol IDs to values. This avoids string lookups at runtime (for performance). For instance, assign each variable declaration an index in a frame array and use that. However, given simplicity and solo dev, a straightforward approach is fine: use a `HashMap<String, Value>` for each scope (the cost is not big unless programs are large).

- **Testing the Interpreter:** Try running some basic programs end-to-end: a hello world (if output supported), some arithmetic, an if/else, a short function. Add tests where you feed code to the interpreter and assert on the resulting value or output. E.g., evaluate an expression `5 plus 3` and check you get `Value::Number(8)`. Evaluate a snippet that defines a variable and uses it, ensuring the final environment has the correct value.

  *AI Assistance:* If stuck with how to implement certain constructs, describe the scenario to ChatGPT (e.g., “How can I implement function calls in an interpreter where functions can be recursive?”). It might suggest using the call stack approach you’re doing. For tricky async logic, you could ask about how to simulate async in an interpreter; though likely we rely on Rust’s async to handle that as mentioned. Copilot can assist writing the boilerplate of evaluation for each AST variant once you start (it may recognize patterns like how to evaluate a binary operation or a loop from your code context).

**Deliverable:** A working interpreter that can execute WFL programs that don’t involve asynchronous operations yet (those will fully come online next milestone). By the end, running the WFL compiler in "interpret" mode will parse, check, and then execute the input program, producing the intended effects (computations, printed output, etc.). This is a major milestone as it turns WFL into a runnable language for the first time. Keep in mind that performance isn’t a priority for the interpreter (it’s fine for now if it’s not super optimized).

## Milestone 8: Standard Library and Core Features

**Description:** Develop the **standard library** of WFL, i.e., the built-in functionalities that users expect out-of-the-box. Even a minimal language will have some basic library features (math functions, string utilities, etc.), and WFL being web-first will require additional capabilities (though those fall under I/O and async, which we handle next). This milestone focuses on non-async core libraries and language features that may not have been fully handled yet, such as collections, string operations, and any remaining built-in functionality (except I/O which is next). Essentially, make sure the language is usable for basic tasks by providing a layer of library support.

- **Builtin Functions and Methods:** Identify which built-in functions WFL should have. From the design, maybe things like:
  - Basic I/O: `print` (to console), though web-first might not emphasize console print, but for dev it’s useful.
  - String operations: perhaps a way to get length of text, concatenate (if not using `+`), substring, etc., unless all done via operators or methods.
  - Math: utilities like rounding, random numbers, etc., if needed.
  - Collections: if WFL has a list or array type, provide functions to push, pop, iterate maybe. If not explicitly in design, you might defer collections or keep minimal (since not mentioned in principles, maybe lists are just used but not heavily featured).
  - Date/time or other utility classes (could be deferred).
  
  Implement these either as *intrinsics* in the interpreter (like when the interpreter sees a call to `print`, it calls a Rust function) or as part of a standard library written in WFL itself. For efficiency, many core functions (especially ones interfacing with system like random, or performance-critical like math) are better implemented in Rust and exposed. You can map them by reserving certain names. For instance, have a map of built-in function names to Rust closures in your interpreter. When the interpreter is about to call a function, it checks if the function name exists in this built-in map before looking at user-defined ones.

- **Implementing in Rust vs WFL:** Decide if some of the standard library could actually be written in WFL and shipped as source that is preloaded. This is advanced (basically having a prelude that the compiler inserts), and not necessary early on. It might be simpler to implement everything in Rust first, then later you could port some to WFL for flexibility. For now, proceed with Rust implementations of needed functionalities.

- **Standard Data Types:** If not already, implement any complex data types needed. For example, WFL’s design might include a notion of an HTML element or a “Page” since it’s web-first (the foundation doc mentioned adding a paragraph to the page). Possibly the standard library should include a representation for DOM elements or similar. If that’s in scope, define a Value variant for DOM nodes or a separate structure that the interpreter can manipulate (maybe as opaque handles which in a browser environment would correspond to real DOM elements – but if running outside browser, you might simulate or just stub it out). This can get deep, so perhaps limit scope: for now, focus on file and network I/O types in next milestone and leave actual browser DOM integration to the compilation to JS/WASM milestone. Instead, ensure WFL has an **Array/List** type and maybe a **Dictionary/Map** type if needed for basic data manipulation.
  
- **Error and Option Types:** Many languages have standard types for errors or nullability. WFL might not expose these directly to beginners (since it tries to be friendly), but internally you might want an `Option` type or `Result` type structure. If WFL has exceptions or a `try/when/otherwise` error handling (the I/O spec hinted at `try/when/otherwise` which sounds like `try/except`), plan that:
  - Possibly treat errors with exceptions or result values. For now, implementing an exception mechanism might be complex, so you might treat it like Elm does: errors are values to handle. But since the spec explicitly says *natural error handling using try/when/otherwise*, that implies a language-level construct. You may plan to implement that as sugar that the parser turns into something like a match on a Result. However, implementing it fully might come after you have I/O operations that can fail.
  - At least prepare the ground by deciding how a function can signal an error. Perhaps any I/O function returns a special `Error` value that triggers the `when/otherwise` in the language. Consider adding an `Error` variant to the `Value` enum or use a separate flow in interpreter (like throwing an exception). We will refine this during I/O, but keep it in mind.
  
- **Integrate with Interpreter:** Add cases in the interpreter to handle new standard library features:
  - If lists are introduced, implement evaluation for list literals and indexing, etc.
  - If new built-in functions are added (like `len(text)`), implement those in the call handling.
  - Make sure the type checker knows about these too (e.g., declare `print` as a function type `Text -> None` so that calling print with a number would be a type error unless you allow implicit conversion).

- **Document and Test Standard Library:** It’s helpful to write a short guide (for yourself) of what standard functions exist and how to call them, then create test scripts using them. For example, test that `print("Hello")` actually outputs Hello, test that list append or similar works if added. Ensure that misuse (like calling a built-in with wrong types) triggers your type checker or runtime errors appropriately.

  *AI Assistance:* ChatGPT can help suggest what basic functions a new language might need for it to be minimally useful. It can also help design the API consistent with natural language style (e.g., maybe instead of `len(text)` WFL might allow “tell length of X” or some phrasing – but that might be too complex to parse; a simple `length(X)` could be fine). Use it to sanity-check that the chosen standard library functions align with WFL’s ethos (clear naming, etc.). Copilot will assist with implementing these functions, especially if any are complex (like a string search or random number generator usage).

**Deliverable:** An initial standard library integrated into the language. Developers can now perform basic tasks (math, string formatting, use simple data structures) using WFL built-ins. With this, WFL is not just a barebones language but starts to have practicality. All standard library features should be covered by tests. The interpreter can execute them, and the type checker should be aware of their signatures.

## Milestone 9: Asynchronous I/O and Task Runtime Integration

**Description:** Incorporate **I/O operations and asynchronous runtime** capabilities into WFL. This is where WFL’s *web-first* nature really comes in: support for networking (HTTP requests), file I/O, and possibly database access, with a unified async approach as outlined in the I/O spec. Technically, this involves integrating with Rust’s **Tokio** async runtime (or a similar library) to handle non-blocking operations and promises (futures) in WFL. The outcome will be that WFL code can “wait for” asynchronous results without blocking the whole program, enabling concurrency and I/O.

- **Design the I/O API in WFL:** Using the provided I/O specification as a guide ([wfl-IO.md](file://file-3JdYNPvd6AvCALmfJSPAbT#:~:text=WebFirst%20Language%20,like%20way.%20Key%20goals%20include)) ([wfl-IO.md](file://file-3JdYNPvd6AvCALmfJSPAbT#:~:text=,across%20files%2C%20network%2C%20and%20databases)), finalize how WFL code will express I/O. For example:
  - *File I/O:* “open file at *path* and read content” might be a single statement that results in the file’s content (or an object). 
  - *HTTP:* “open url *link* and read response” similarly.
  - *Database:* maybe “open database *conn* and query *sql*”.
  - The unified syntax suggests using common verbs (`open`, `read`, `write`, `close`, etc.) across these domains.
  
  Likely, the parser and AST need to represent these. One approach: treat them as function calls or method calls in the AST. For instance, `open file at "foo.txt" and read content` could be parsed into something like:
  ``` 
  AST: Call(Identifier("read_file"), [Literal("foo.txt")]) 
  ```
  where `read_file` is a runtime function that opens and reads. Or you create a special AST node `IoOperation(IoKind::ReadFile, args...)`. Another example, `wait for (open url "...") then ...` could be parsed as an `Await` AST node around a `Call(Identifier("http_get"), [...])`.
  
  You might lean towards mapping the natural syntax to known function calls, as that keeps the interpreter simpler (just handle a set of known functions for I/O). Update your grammar to support these phrases. Possibly introduce new rules: e.g., a rule for `IOExpr` that matches `"open" ~ "file" ~ "at" ~ StringLiteral ~ "and" ~ "read" ~ "content"` and builds an AST call to a builtin. Alternatively, parse it as a generic structure and interpret accordingly.
  
  *Grammar implementation:* If using Pest, you might add something like:
  ```pest
  IOReadFile = { "open" ~ "file" ~ "at" ~ STRING ~ "and" ~ "read" ~ "content" }
  ```
  and then in post-processing, convert that parse node to an AST function call or a special AST variant.

- **Integrate Tokio Runtime:** Add **Tokio** as a dependency (if not already via other crates). Tokio will provide the event loop for async tasks. Modify the main entry point of the WFL interpreter (or compiler binary) to start a Tokio runtime. For example, use `#[tokio::main]` on the `fn main` of the REPL or compiler CLI. This ensures that when WFL code performs async operations, they actually run concurrently. Inside the interpreter, when you hit an operation that is asynchronous (like an HTTP request or a file read if done async), you will use Tokio:
  - For file I/O, Tokio provides async file APIs (`tokio::fs`).
  - For HTTP, you might use an HTTP client like **Reqwest** (which itself uses Tokio under the hood) or lower-level **hyper**. To keep it simple, consider using `reqwest` for HTTP calls in async fashion.
  - For database, if needed, a crate like `sqlx` or others could be used which are async as well. But maybe stub or skip DB in initial version if too much; focus on file and network which are common.
  
  *Implementing Async in Interpreter:* Refactor the interpreter’s evaluation functions to be async (i.e., return `Future`s). For example, `eval_expr` becomes `async fn eval_expr(...) -> Result<Value, RuntimeError>`. This way, if during evaluation you call an async Rust function (like `reqwest::get().await`), you can `.await` it within the interpreter logic. For instance:
  - If AST node is `IoOperation::HttpGet(url)`, your interpreter code would do something like:
    ```rust
    let response = reqwest::get(url).await.map_err(|e| RuntimeError::IoError(e.to_string()))?;
    let text = response.text().await?;
    Ok(Value::Text(text))
    ```
    wrapped appropriately.
  - If AST node is `Await(innerExpr)`, you evaluate `innerExpr` which should produce a `Value::AsyncTask` or similar, then you `.await` on that task to get its result. However, if you design it such that `innerExpr` itself already performs the async operation (like `http_get` above returns a `Text` by awaiting internally), then `Await` might be implicit. Another design: represent ongoing futures as a `Value::Future(FutureHandle)` and only on `wait for` do you `.await`. This is closer to how JavaScript Promise works. But implementing your own Future poll logic is complex – better to leverage Rust’s async directly.
  
  Perhaps a pragmatic approach: whenever a WFL async operation is invoked, just perform it to completion (since you are in an async function anyway). The `wait for ... then ...` could simply be syntax; you might not actually implement lazy futures at the WFL level, instead any `open ... and read` returns the final result (because you awaited it internally). But then why have "wait for ... then ..." syntax? Possibly for sequencing multiple async calls without nested callbacks, which you can reflect by just writing the code sequentially in interpreter as well. 
  In summary, you can implement WFL async as *direct coroutines* using Rust async underneath: the WFL code order will naturally await each async call in sequence.

- **Batch Operations & Parallelism:** The I/O spec mentions batch operations (like reading multiple files in parallel). Supporting true parallel awaits would mean letting WFL spawn tasks. You could introduce a syntax like “wait for both (op1) and (op2)” or simply encourage launching tasks. This might be advanced; for now, ensure at least that multiple independent async ops can be fired if written imperatively (they’ll just execute sequentially unless you allow something like explicit `spawn`). A future enhancement could be an API like `start task X` which returns a task handle (future) that you can await later. If you want to include something like that: implement a built-in function `spawn(async_func, args…)` that returns a `TaskHandle` (which is a wrapper around a `JoinHandle` from Tokio). Then a `wait for` could accept that handle to join it. This might be beyond initial scope, but design is flexible for later.

- **Unified Resource Management:** Ensure that after performing an I/O operation, resources are closed if needed. For example, file handles should be closed (Tokio file read to end will close automatically when file object is dropped). For HTTP, connections are managed by the client. If providing an explicit `close` operation in WFL (like `close file X`), you might not need it for simple reads (since reading fully can auto-close), but if WFL allows opening a resource and keeping it open for streaming, you need a way to close. Implement a `close(handle)` built-in if necessary, which would map to dropping or closing in Rust. Keep track of open resources in the interpreter environment (e.g., an `ResourceHandle` in Value variant which holds the actual Rust object like a file or network connection).

- **Error Handling in I/O:** Many I/O operations can fail (file not found, network error). WFL’s `try/when/otherwise` construct is intended for this. Decide how to implement it:
  - One way is to use Rust’s `Result` mechanism: e.g., `open file` returns either the content or an `Error` value. Then `when/otherwise` in WFL is like pattern matching on that. But that might complicate WFL code (not very “natural”).
  - Alternatively, treat I/O errors as exceptions: if an I/O builtin fails, you could raise a `RuntimeError` up the interpreter, which would unwind to the nearest `try ... when ... otherwise` in WFL AST. This means implementing `try` as a construct that catches exceptions. For simplicity, you might implement `try { block } when { err -> ... } otherwise { ... }` as something that in AST is like a `Try(block, errVar, catchBlock, finallyBlock?)`. At runtime, you use Rust `Result` propagation: run the block, if it returns Err(RuntimeError) and that error is of type I/O, then execute the catchBlock with the error bound to errVar. Implementing this fully could be heavy. 
  Perhaps delay full `try/when` until after basic I/O works, unless it’s straightforward. You can always have I/O functions return an `Error` Value and not throw, then implement `when/otherwise` as syntactic sugar for checking that and branching. For example, parse `try X when E otherwise Y` into something like an if that checks an error flag in result of X. This might require boxing the result and error together (like always return an object or sum type).
  
  Given complexity, you might provide minimal error handling: operations either succeed or you stop execution on error (printing an error). Full user-facing error handling can be a later improvement.
  
  *AI Assistance:* Discuss with ChatGPT how to best integrate async/await semantics. For example, ask “In a simple interpreter, how can I implement the equivalent of async/await using Rust’s Tokio?” It might confirm the approach of making evaluation functions async and using Tokio’s runtime. Also ask about error handling strategies (exception vs result) in interpreted languages. This can yield insight from Python/JS analogies.

- **Testing Async and I/O:** This is critical. Write WFL programs that perform file reads and HTTP requests (maybe use a public API or a local test server). For file I/O, create a temp file and have WFL read it, verify content. For network, maybe point to `http://example.com` or better, run a local server to return a known response. Also test non-happy paths: file not found (should produce an error in WFL), bad URL or network down. If implementing `try/when`, test that as well by causing an error and seeing if the catch clause runs.

**Deliverable:** WFL now supports asynchronous operations and I/O in a unified way. The interpreter, running on Tokio, can execute WFL code that does things like fetch URLs, read/write files, etc., using `wait for` syntax to pause until completion without blocking other tasks. This fulfills a core promise of WebFirst Language. By the end of this milestone, WFL can be used to write simple web client code or server interactions in a high-level fashion, and those programs will actually do the I/O and return results. It’s important that the system is stable (no deadlocks, resources freed, errors handled to some extent) because I/O is often where things go wrong.

## Milestone 10: Error Reporting and Diagnostics System

**Description:** Improve the compiler/interpreter’s error reporting to be **clear, actionable, and user-friendly**, aligning with WFL’s emphasis on helpful errors (inspired by Elm) ([wfl-foundation.md](file://file-2R4kWv6kRZFxzrEzf6aiTM#:~:text=4,Reporting)). Up to now, we have accumulated error information (syntax errors, semantic errors, type errors, runtime errors) mostly as raw messages or simple structs. In this milestone, we build a proper diagnostics system to present errors (and warnings, if any) with contextual messages, possibly code snippets, and suggestions.

- **Structured Error Data:** For each kind of error (parse, semantic, type, runtime), ensure you have a structured representation:
  - e.g., `ParseError { location: Span, message: String }`
  - `TypeError { location: Span, expected: Type, found: Type, message: String }`
  - `RuntimeError { location: Option<Span>, message: String }` (runtime might not always have a span if it’s an internal issue, but usually if it relates to a specific AST node, you have that span).
  - If you want richer structure: include the source snippet or line in the error, or a code frame. You can always get that later via the span and stored source text.
  
  Ensure that during parsing and analysis, you stored the source code text or have access to it when printing errors. Typically, one stores the input source in a `String` and keeps it around, and spans are (file_id, start_offset, end_offset) or (line, column info).

- **Diagnostic Formatting:** Use a library or implement your own formatting:
  - The Rust community has crates like **`codespan-reporting`** and **`ariadne`** that help produce colorful, nicely formatted error messages with source code excerpts ([GitHub - brendanzab/codespan: Beautiful diagnostic reporting for text-based programming languages.](https://github.com/brendanzab/codespan#:~:text=Beautiful%20diagnostic%20reporting%20for%20text,programming%20languages)). For example, `codespan-reporting` can take your error with a span and generate an output like:
    ```text
    error: Expected a number but found text
       --> example.wfl:10:15
        |
     10 | let age be "twelve"
        |            ^^^^^^^^ expected Number here
    ```
    These libraries let you annotate spans with labels and suggestions. Consider integrating one: it will save time and yield professional-looking results. For instance, you can create a `SimpleFile<String, &str>` for the source and then for each error, create a `Diagnostic` with labels for spans ([GitHub - brendanzab/codespan: Beautiful diagnostic reporting for text-based programming languages.](https://github.com/brendanzab/codespan#:~:text=Example)) ([GitHub - brendanzab/codespan: Beautiful diagnostic reporting for text-based programming languages.](https://github.com/brendanzab/codespan#:~:text=let%20mut%20files%20%3D%20SimpleFiles%3A%3Anew)).
  - If not using a library, you can still format errors decently: e.g., print the file name and line, then the line of code with a caret under the error portion, and a message. This is more work and likely less pretty. Given we want a high-quality language, using a crate is advisable.

  *Integration:* Add `codespan-reporting` to Cargo dependencies. Write a helper function `report_error(error: Error, source: &str)` that uses the library to print to stderr. You will need to map your internal error representation to the diagnostic. For multiple errors, print all or up to some limit.

- **Actionable Messages:** Revisit each error message text and improve clarity and actionability:
  - For syntax errors: instead of "Unexpected token", say something like "I was expecting X here, but found Y. Perhaps you forgot ...". This may be tricky to do generally, but for common mistakes you can special-case (like if a block isn't closed, you detect EOF, suggest "It looks like something wasn't closed").
  - For type errors: mention both expected and found types clearly (“Expected a Number but found a Text. You might need to convert the text to a number first.”). This directly hints the solution, which matches Elm’s style of being helpful.
  - For undefined names: “`foo` is not defined in this scope. Did you misspell it or forget to declare it?”.
  - These improvements can be done gradually. Write a small mapping from error code or type to a polished message string. Alternatively, use ChatGPT itself: for a given raw error, ask it to phrase it like a helpful compiler error. It might produce a good phrasing you can adapt.
  - Also consider warnings for potential issues (maybe not much in WFL yet, but e.g., unused variable could be a warning). Set up an infrastructure for warnings similar to errors.

- **Interactive Error Experience:** If integrating with an editor or REPL, ensure errors show nicely there as well:
  - In the REPL, catching an error should not crash the REPL but print the error diagnostic and allow continuing.
  - For editor integration (next milestone), having errors with spans is essential so the editor can underline them. If using an LSP, you will send structured errors.

- **Testing Error Messages:** Create scenarios in code that trigger each kind of error and observe the output. It’s often helpful to have a dedicated test that runs the compiler on an erroneous file and compares the error output to an expected output (snapshot testing). This ensures your messages don’t unintentionally change and meet quality. Because messages are user-facing, this is worth doing.

  *AI Assistance:* Use ChatGPT to critique your error message wording. You can show it a draft message and ask if it's clear or how to make it more beginner-friendly. Also, ask it to generate some common mistakes in WFL and see if your error handling catches them gracefully. Copilot might not directly help with message text, but it can help using the codespan API by providing code from examples.

- **Integration with Previous Stages:** Go back to your parser/typechecker and make sure they propagate file/line info correctly so that now you can pinpoint errors. You might need to modify your AST nodes or symbol table to carry `Span` data if you haven’t already. It’s worth the effort: an error without location is frustrating.

**Deliverable:** A polished error-reporting subsystem. When a user writes incorrect WFL code, the compiler/interpreter will output clear errors with source context and hints, rather than cryptic messages or Rust panic traces. This greatly improves the developer experience and is in line with WFL’s goals. You should be proud to see errors from your compiler that look as nice as Rust’s or Elm’s. For example, by the end of this milestone, a type error might be displayed with a snippet and suggestion, fulfilling the principle of *clear and actionable error reporting*. 

## Milestone 11: REPL (Read-Eval-Print Loop) Implementation

**Description:** Develop an interactive REPL for WFL, allowing developers to type WFL commands or code snippets and see results immediately. This is useful for experimentation and learning. The REPL will reuse the compiler front-end and interpreter you’ve built, but needs to manage state between inputs (so you can define a variable and use it in the next line, etc.).

- **Basic REPL Loop:** Use a crate like **`rustyline`** to handle line editing, history, and prompt display ([Read-Eval-Print Loop (REPL) - Create Your Own Programming Language with Rust](https://createlang.rs/01_calculator/repl.html#:~:text=REPL%20,we%20can%20optionally%20choose%20to)). `rustyline` provides an easy way to read a line of input with editing support. Set up the REPL to:
  - Print a prompt (e.g., `wfl> `).
  - Read a line of input from the user.
  - If the input is just whitespace or empty, continue (maybe maintain previous multi-line if needed).
  - If the input is an exit command (like `.exit` or pressing Ctrl-D), break the loop.
  - Otherwise, feed the input to the WFL compiler pipeline (parse -> check -> interpret).

- **Maintaining State:** By default, your compiler treats each run as a fresh program. In a REPL, after executing a line, the definitions (variables, functions) should remain for subsequent lines. Approach:
  - Maintain a persistent AST or environment that accumulates. For simplicity, you can maintain the interpreter’s runtime environment (variable map) across iterations. For example, if a user does `let x be 5`, you execute it and store `x=5` in the REPL’s environment. Next input, before execution, preload that environment so `x` is still defined.
  - Also, for functions or other definitions, you might need to store their AST or compiled form. Possibly easier: each time, instead of discarding the AST, *append* it to a global AST representing the session. But continuously growing AST might be tricky. Alternatively, after each input, update the symbol table: e.g., add any new function definitions to a global list of functions, so they can be called later.
  - One design: have a structure `ReplState` with fields: `variables: HashMap<String, Value>`, `functions: HashMap<String, FuncDefAST>` (or a compiled closure), etc. After each input, update this state. On a new input, inject these definitions into the compilation process. Possibly by prepending some generated code (for parser context) or by checking this state in the symbol resolution (like treat those symbols as pre-defined).
  - A simpler hack: for each REPL input, manually combine it with previous inputs into one pseudo-program and compile that. But that gets complicated as history grows (and error re-checking old lines etc.). Better to manage state explicitly.
  
- **Executing Expressions vs Statements:** In a REPL, when the user enters an expression (not a full `let` or something), it’s nice to print the result of that expression. Decide how to differentiate:
  - One way: try parsing the input as an Expression (maybe wrap it in a dummy statement if needed). If it parses as an expression and not as a formal statement, then after interpreting it, print the returned Value. If it’s a `let` or a function def (which in WFL might not produce a value), you might not print anything or a confirmation.
  - Many REPLs (like Python) print the result of any expression statement automatically. You can adopt that: track if the last executed AST node was an expression and not a declaration, and if so, print its `Value` (in a user-friendly format). If the Value is an object or list, format accordingly. You might want to implement a `Display` trait for your `Value` enum for nice output. Ensure no huge data dumps on large structures (maybe cap output length).
  
- **Multiline Input:** People might want to write multi-line constructs (like a function or an if with an else). `rustyline` can handle multiline input if you detect that the input is incomplete (for example, if the user types `if condition then` and presses Enter, that’s not a complete statement because no `otherwise` or no end of block detected). You can detect incomplete input if the parser returns an error indicating more is expected (or if input ends with a continuation token like `then`). 
  - A straightforward approach: if parse fails and the error is of type "unexpected EOF" or similar, then don’t error; instead, prompt for another line continuing (maybe change prompt to `...>`). Append the new line and try parse again. Continue until parse succeeds or a different error occurs. This requires re-entrant parsing.
  - If too complex, an alternative is to require that each REPL entry is a single statement or expression. But that hampers writing functions or multi-line blocks. It's a nice feature to support multi-line, but not mandatory for initial REPL version.
  
- **Error Handling in REPL:** Make sure that if an input has an error, it prints the error (using your nice error reporter) but does not exit the REPL. It should simply allow the next input. If the error is that a definition failed, best to not add that partial definition to state. e.g., if user tries to define a function with a type error, that function should not exist in state after the error.
  
- **Using AI in REPL context:** Optionally, one can imagine using an AI assistant within the REPL to explain errors or suggest next steps, but that’s beyond scope. However, you could advertise that ChatGPT was used to build WFL here jokingly or keep as an internal note.

  *AI Assistance:* ChatGPT can provide guidance on maintaining state in a REPL. It might propose using an AST storage or environment carry-over. Copilot will help writing the loop and integrating the reading logic, especially since many projects have similar REPL patterns (it might even suggest using `rustyline` because it's common).

- **Testing the REPL:** Manually test it by running the compiled binary and typing in commands:
  - Define variables and use them.
  - Define a function and call it later.
  - Do an asynchronous call (the REPL’s Tokio runtime should handle it since main is async).
  - Trigger an error and see that it doesn’t quit.
  - Test multiline if possible.
  - Check history (arrow keys to retrieve old commands, ensured by rustyline).
  
  Also, perhaps write an automated test that simulates input/output: you can spawn the REPL process and send input to stdin, but that’s complex to automate. Manual testing and maybe a small integration test with predetermined input is fine.

**Deliverable:** A user-friendly interactive REPL for WFL. The developer can launch `wfl` in interactive mode and get a prompt to try out the language. This greatly helps exploration and debugging. It also proves that the language implementation can handle incremental input and maintain state, which is a good validation of the architecture. With the REPL, WFL is now accessible for quick experiments just like Python/Node REPLs, fulfilling part of the tooling goals.

## Milestone 12: Editor/IDE Integration (Language Server Protocol)

**Description:** Provide editor integration for WFL to improve developer experience in code editors (like VS Code, etc.). The standard way is to implement a **Language Server Protocol (LSP)** server for WFL, which can offer features like real-time error checking, auto-completion, and go-to-definition. As a solo developer, you can start by implementing at least diagnostics (error squiggles) and maybe symbol completion. This milestone might be considered advanced, but even a basic LSP that reports the compiler’s errors is extremely useful.

- **Choose an LSP Framework:** Instead of implementing LSP protocol from scratch, use an existing Rust library such as **`tower-lsp`** or **`lsp-server`** (used by rust-analyzer). These provide a skeleton where you implement trait methods for initialize, text change, etc., and they handle the JSON RPC communications. For example, `tower_lsp::LspService` can be used to create a server easily.

- **Basic Capabilities – Diagnostics:** Implement the language server to parse and type-check documents on the fly and send back diagnostics (errors/warnings). Steps:
  - When the editor opens a WFL file or as the user types, the LSP will receive the text (didOpen, didChange events).
  - On each change (you might debounce or handle at interval), run your compiler front-end (parser + checker) on the document text. Because this is just analysis, you don’t actually run the program. You’ll use the same parsing and type checking functions but in “no execution” mode.
  - Collect the errors, convert them to LSP Diagnostic objects (they require range in the document, message, severity, etc.). You have spans (with line/col) from your error system; map those to LSP ranges.
  - Send a “publishDiagnostics” notification to the client with these diagnostics. The editor will then underline errors and show messages on hover.
  - This ensures that as the developer writes code, they see if they made a mistake (essentially like a compiler check in the IDE).
  - Test this using a VSCode extension or `vim` LSP client by connecting it to your server.

- **Auto-Completion (optional):** A nice-to-have is completion suggestions. For WFL, this might include:
  - Keywords or natural language snippets (like suggesting “open file at … and read content” template when they start typing "open f...").
  - Variables in scope or functions in scope.
  - Since WFL is wordy, completion can actually be quite helpful (to avoid typing a long phrase).
  - Implement by handling the `textDocument/completion` LSP request: get the position, see what the user has typed. If they typed, say, `open f`, you detect this and can suggest `file at "<path>" and read content`. For variables, maintain a list of symbols from last compile (the symbol table can give you all current variables/functions in scope at that position).
  - This can be complex to do perfectly (need scope resolution by position). A simpler heuristic: after each successful compile, store all defined identifiers (vars, funcs) and just always suggest those when typing an identifier. Not scope-aware but easier. Or integrate with the symbol table builder to query symbols at a given AST node corresponding to cursor.
  - If time permits, also implement function signature help (on `(` trigger an info with parameters) and hover documentation (when hovering a symbol, show its type or value if constant).

- **Testing LSP:** This is tricky without an editor, but you can simulate by sending JSON requests to your language server process. There are integration tests possible using `tower-lsp` testing utilities, or just run it and use VSCode. For now, manual testing is likely:
  - Launch VSCode with a simple extension configuration pointing to your `wfl-lsp` executable.
  - Open a WFL file, type code with errors, see if they appear.
  - Try auto-complete after typing part of a known keyword or variable.
  - Adjust as needed.

  *AI Assistance:* ChatGPT might provide a blueprint for an LSP implementation as it’s a common pattern. It can also help with ideas for completions, like what could be useful to WFL developers. Copilot, when editing the LSP code, may suggest correct sequences as it has likely been trained on similar code (especially if using tower-lsp, since it’s common, Copilot might fill in method stubs or example logic).

- **Documentation Tools (optional):** Editor tooling could also mean syntax highlighting definitions or generating docs. If time, you can write a TextMate grammar or Tree-sitter grammar for WFL for syntax highlighting. That’s outside the main code, but adding it to the project (as a VSCode extension package) would be nice. However, given focus, skip deep dive here.

**Deliverable:** A working language server for WFL that at least provides real-time diagnostics in an editor. This makes development in WFL much more pleasant, catching errors as you type rather than only at run compile time. It demonstrates that WFL is maturing as a tool. Combined with the REPL, this means WFL now has both interactive and integrated development support.

## Milestone 13: Web Integration and Deployment

**Description:** (Optional final milestone) Enable WFL to be used in real web environments by compiling it to web-friendly targets (JavaScript or WebAssembly) and ensuring it can interface with web standards like DOM, CSS. This goes beyond the interpreter, moving WFL from “running on Rust as a host” to being able to run in a browser or be included in web projects.

- **Compile to JavaScript or WASM:** Decide on a compilation strategy:
  - **Transpile to JavaScript**: Since WFL syntax is high-level, you can translate the AST to equivalent JavaScript code. For example, a WFL function becomes a JS function, WFL async operations become async/await in JS, etc. This could be done by writing a code generator that walks the AST and prints JS code. This is a straightforward but potentially large task. Alternatively, 
  - **Compile to WebAssembly**: This could be done by translating AST to an intermediate representation then using a crate like `wasm_encoder` or `Cranelift` to produce WASM bytecode. WASM is more low-level (no direct DOM access unless through JS bridging), and implementing GC or complex data structures in WASM might be hard. Given WFL’s high-level nature, compiling to JavaScript might actually be more practical, because it can directly manipulate the DOM or call web APIs which WASM cannot do without JS glue.
  - As an initial step, implementing a **JavaScript backend** is the most feasible: you already have a runtime in Rust; you can mimic that logic in JS output. For instance, for each AST node type, produce corresponding JS code:
    - Expressions map to JS expressions (with perhaps variable name mapping).
    - Async functions in WFL become `async function` in JS.
    - Built-in operations like file I/O or HTTP: in a browser context, file I/O is not allowed, but network is (via fetch). So WFL’s `open url and read content` could compile to a `fetch()` call in JS.
    - DOM operations (if any in WFL standard lib) could compile to actual DOM calls (like `document.createElement` etc.).
  - This is a huge area, so possibly narrow: ensure that pure computation and network calls compile. The developer can then run the JS in the browser or Node. 

- **Compiler Implementation:** Create a new component for code generation. Possibly as part of the main binary, add a flag or mode to output JS instead of interpreting. For each function or top-level statement in AST, generate equivalent JS source text. Manage indentation, scopes, etc., properly.
  - You might find it easier to piggyback on an existing JS AST crate or just build strings. Simplicity: build strings.
  - Use the symbol table from before to know variable names (they’re the same but ensure no conflicts with JS reserved words).
  - Types can be largely ignored in codegen since JS is dynamic, but you might want to insert runtime checks or conversions if needed (like ensure a number vs string if aligning with WFL types).
  
- **Testing the JS Output:** Take some WFL code, compile to JS, then manually run the JS (in Node or browser) to see if it behaves the same as running in the interpreter. Write tests comparing interpreter result vs compiled JS result for some inputs.

- **Web APIs and Interop:** This is where WFL can shine but also is challenging:
  - Provide a way for WFL to interact with the DOM. Possibly by exposing some global objects or functions. If compiling to JS, the simplest approach is to allow calling JS from WFL: e.g., you might let WFL code call a built-in `js(...)` function that injects raw JS or call a JS library. Or design WFL standard library functions that correspond to common web actions (like `add_element("p")` which under the hood does `document.createElement("p")` in JS).
  - Considering time, you might not implement full DOM integration, but set the stage for it. For instance, ensure that if someone writes WFL that calls an “alert” function, your JS backend can call `alert()` or if they call some interop function, it passes through.
  - Possibly define a special WFL type or value that is just a wrapper around a JS object (like DOM node) and your interpreter and JS backend both can handle that (the interpreter could simulate a minimal DOM or just skip, while JS will have actual DOM).
  
- **Security features:** From guiding principles, auto-escaping output to prevent XSS is mentioned ([wfl-foundation.md](file://file-2R4kWv6kRZFxzrEzf6aiTM#:~:text=8.%20Built)). If WFL has templates or HTML generation, ensure that if compiling to web, it escapes strings placed in HTML by default. This could be done by making any library function that outputs to page do an escape (like replacing `&` `<` etc.). At compile-time, you might not enforce, but at runtime (interpreter or compiled) ensure those library functions do it. If not implemented already in standard library, note it as a to-do or implement a simple one for any relevant function (like a `innerHTML` setter equivalent should escape, etc.).

- **Packaging:** Provide a way to use WFL in projects:
  - If compiled to JS, you might create an npm package or CLI command to generate a `.js` file from a `.wfl` file. That way, web developers can integrate it. Document how to include the runtime (maybe the output JS is standalone if you include all needed functions).
  - If WASM, you’d compile to a `.wasm` and have a JS glue. That’s more complex due to lack of GC.
  - For now, perhaps just ensure the CLI can output JS and leave it to user to include that in HTML.

- **Testing on Web:** If possible, take a small WFL snippet that manipulates DOM (or just does a fetch and logs result) and include the generated JS in an HTML file. Open in a browser and confirm it works. This shows end-to-end that WFL can be used “web-first” as intended.

**Deliverable:** A prototype compiler back-end for web integration. This is marked optional or advanced because it is a substantial effort, but achieving it means WFL can run in a browser environment, fulfilling its vision. The deliverable could be a new command like `wflc --target=js input.wfl -o output.js` that produces JavaScript. Along with this, update documentation on how to use WFL for web. Even if not fully feature-complete, having a path to compile to JS/WASM demonstrates forward-thinking in the project and provides a clear route to real-world usage.

---

With these milestones completed, the WebFirst Language would have been designed, implemented, and equipped with a runtime, standard library, robust tooling (REPL, LSP), and even a path to web deployment. Throughout the project, leveraging AI tools like ChatGPT and Copilot accelerates brainstorming, coding, and problem-solving at each step – from shaping the natural-language syntax to debugging tricky runtime issues. The milestones are structured to build complexity gradually, with early focus on design and core compiler, and later focus on runtime, UX, and integration. This ensures a solo developer can make consistent progress and have a working language early (by Milestone 7 or 8, an MVP of WFL is running), then polish and extend it in subsequent milestones. 

Every milestone's outputs (specs, code, tests) serve as a foundation for the next, reflecting a logical dependency chain. By following this plan, the development of WFL in Rust can be managed in attainable stages, all while using modern tools and libraries to lighten the load where possible (e.g., using Pest for parsing ([Building a Rust parser using Pest and PEG - LogRocket Blog](https://blog.logrocket.com/building-rust-parser-pest-peg/#:~:text=Currently%2C%20there%20are%20several%20parser,are%20LalrPop%2C%20Nom%2C%20and%20Pest)), Tokio for async runtime ([Roll your own JavaScript runtime](https://deno.com/blog/roll-your-own-javascript-runtime#:~:text=,s)), rustyline for REPL ([Read-Eval-Print Loop (REPL) - Create Your Own Programming Language with Rust](https://createlang.rs/01_calculator/repl.html#:~:text=REPL%20,we%20can%20optionally%20choose%20to)), and codespan for error reporting). The result will be a new programming language aligned with the vision of being *web-first* and *human-friendly*, built efficiently by a solo developer with AI-assisted productivity.

---

## Known Issues and Outstanding Tasks

### High Priority Issues

#### Parser and Runtime Issues
- [ ] **Runtime Type Conversion Error with "of" Syntax** - **Added 2025-01-08**
  - **Description**: When using natural language function calls with "of" syntax (e.g., `path_join of "home" and "user"`), a runtime type conversion error occurs: "Expected text, got Boolean"
  - **Status**: Parser correctly handles the syntax after recent fixes, but there's an issue in argument processing/type conversion during runtime
  - **Impact**: Prevents full functionality of natural language function calls despite parser working correctly
  - **Investigation needed**: Check how function call arguments are processed and converted in the interpreter
  - **Workaround**: Use intermediate variables to store function results before using them in expressions
  - **Related**: Parser "of" syntax issue was resolved in commit ba93c9c, but runtime processing still has issues

### Recently Completed
- [x] **Parser "of" Syntax Issue** - **COMPLETED 2025-01-08**
  - Fixed KeywordOf and KeywordAnd token handling in parser
  - Added comprehensive filesystem standard library module
  - All unit tests passing (161 tests)
  - Branch: `devin/1751974012-filesystem-stdlib`

---

*Last updated: 2025-01-08*y.

