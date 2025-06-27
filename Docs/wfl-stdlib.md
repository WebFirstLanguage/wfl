Great. I’ll design a modular standard library for WFL that includes core built-in functionality (math, text, collections, etc.) implemented in Rust and exposed globally. I’ll prioritize conventional function calls (e.g., `length(text)`, `random()`) unless natural-syntax alternatives offer significant clarity. I'll also prepare the interpreter hooks needed to register and invoke these functions.

The output will include guidance on which functions should be implemented directly in the Rust interpreter and which could be offloaded to WFL source files later. I’ll also document where async-related functionality will eventually go, but skip implementing it for now.

I’ll get started and let you know when it’s ready.

# WFL Standard Library Design and Architecture Guide

## Overview of the Standard Library

The WFL standard library provides essential built-in functions for basic programming tasks, organized into **modular categories**. Each category (module) groups related functions (e.g. math utilities, text processing) to keep the design clean and understandable. All built-ins are exposed as conventional *function calls* (e.g. `length(x)` or `random()`), not as methods (`x.length`), to align with WFL’s natural-language-friendly style while keeping the syntax simple. This means you call a function by name with arguments in parentheses, without special dot notation.

Key design points:

- **Global Intrinsic Functions:** The standard library functions are implemented as **Rust intrinsics** (native functions) and registered in the WFL interpreter’s global environment ([docs.md](file://file-JkXXku8iwKMdy2TtL7T5Sv#:~:text=standard%20library%20written%20in%20WFL,defined%20ones)). They behave like any user-defined function call in WFL code, but their logic is executed in Rust for efficiency.
- **Static Typing Support:** The WFL typechecker is extended to recognize these built-ins and enforce correct argument and return types. Each function has a defined type signature known to the compiler, preventing misuse (e.g. calling `round("text")` would be a type error). The interpreter and typechecker maintain a registry of built-in function names with their expected types ([docs.md](file://file-JkXXku8iwKMdy2TtL7T5Sv#:~:text=standard%20library%20features%3A%20,unless%20you%20allow%20implicit%20conversion)).
- **Natural Naming:** Function names are chosen to be clear English words (or common abbreviations) that convey their purpose. For example, `to_uppercase(text)` is used instead of a symbolic or terse name. This keeps WFL code readable and beginner-friendly, consistent with WFL’s design ethos ([docs.md](file://file-JkXXku8iwKMdy2TtL7T5Sv#:~:text=,or%20random%20number%20generator%20usage)).
- **Modular Code Structure:** In the Rust implementation, each category of built-ins lives in its own module (e.g. a Rust module `math` for math functions, `text` for string functions, etc.). This modular layout makes the code easier to navigate and maintain. We recommend creating a `stdlib` directory with separate source files for `core.rs`, `math.rs`, `text.rs`, and `list.rs` (and others as needed), each defining the functions for that domain.
- **Integration:** The interpreter on startup will **register** all standard library functions by populating the global environment with their names and native function pointers. Likewise, the typechecker is configured with the signature of each function so it knows what types they expect/return. Together, this ensures that WFL programs can call these functions as if they were built-in language features.
- **Future Expansion:** This initial standard library covers synchronous basics. We include placeholders for upcoming asynchronous and I/O features (like `fetch` or file I/O) as stub functions. These stubs will be defined so that the syntax is recognized, but they currently might do nothing or return a dummy value, anticipating proper implementation in **Milestone 9**. This way, the structure is in place for expansion without breaking compatibility.

With this overview in mind, we now detail each module and its functions, followed by implementation notes on how to integrate them.

## Core Module (Basic Utilities and System Functions)

The **core module** contains fundamental utilities that don’t fit into a specific type category. These are simple, globally available functions for printing output and introspection. They have minimal dependencies and often return a special `Nothing` type (called **“nothing”** in WFL) when they don’t produce a meaningful value. Key functions in the core module include:

- **`print(value)`** – Outputs the given value to the console or standard output. This is primarily for debugging or text output in a development context. In a web-first environment WFL might normally output to a web page, but `print` is still useful during development.  
  • *Parameters:* one argument of any type (Text, Number, Bool, List, etc.). WFL will convert the value to a string for display.  
  • *Return:* Nothing (no value). After printing, it returns `nothing` to the caller (in practice this is the `Null` value in the interpreter).  
  • *Implementation:* In Rust, this calls the standard output (e.g. using `println!`). The interpreter’s `native_print` function will iterate through the arguments and print them. (Multiple arguments can be allowed; the current design prints them separated by spaces, similar to how the existing `display` works.) Finally it returns `Value::Null` ([src.md](file://file-KK8ZWKnRdcranrNnggRhxv#:~:text=fn%20native_display%28args%3A%20Vec%3CValue%3E%29%20,arg%29%3B%20%7D%20println%21%28%29%3B%20Ok%28Value%3A%3ANull)).  
  • *Typechecking:* The typechecker can allow any type for `print`. We can treat `print` as having a polymorphic parameter – effectively `(Any) -> Nothing`. Since WFL’s type system doesn’t have a formal “Any” type, an implementation strategy is to accept any type without error. The typechecker may special-case `print` to skip strict type enforcement on its argument, or treat its parameter type as `Unknown` (which is compatible with any concrete type). The return type is `Nothing` ([docs.md](file://file-JkXXku8iwKMdy2TtL7T5Sv#:~:text=the%20call%20handling.%20,unless%20you%20allow%20implicit%20conversion)), so using the result of `print` in an expression will be a type error (since “nothing” isn’t a usable value).

- **`type_of(value)`** – Returns a text string describing the type of the given value.  
  • *Parameters:* one argument of any type.  
  • *Return:* Text (for example, `"Number"`, `"Text"`, `"List"`, etc.).  
  • *Implementation:* Calls an intrinsic that inspects the runtime type. This is easily done by using the value’s internal `type_name()` method ([src.md](file://file-KK8ZWKnRdcranrNnggRhxv#:~:text=impl%20Value%20,Function)). For instance, a Number value returns `"Number"`, a Boolean returns `"Boolean"`, and so on. The Rust implementation will match on the Value enum or directly use `value.type_name()` which is already provided.  
  • *Typechecking:* Accept any type as input (similar reasoning to `print`). The output is always a Text. The typechecker can infer that `type_of(X)` yields `Text` regardless of X’s type. This function is mainly for introspection or debugging.

- **`is_nothing(value)`** – Checks if the given value is “nothing” (the WFL equivalent of null/None).  
  • *Parameters:* one argument of any type.  
  • *Return:* Boolean (`true` if the value is the `Nothing` type, otherwise `false`).  
  • *Implementation:* In Rust, implement as a simple check: return `Value::Bool(true)` if the argument is `Value::Null`, or `Value::Bool(false)` otherwise. This uses the fact that the interpreter represents “nothing” as `Value::Null`.  
  • *Typechecking:* Accept any type for the parameter. Return type is `Boolean`. This function enables WFL scripts to gracefully handle optional or missing values. For example, after calling a function that might not return a value, a user can do `if is_nothing(result) then ...` to check. The typechecker can consider `is_nothing(x)` always valid, since it’s analogous to a null-check that should be allowed on any value.

In the future, the core module (or a related module) will also include **async and I/O stubs**. For instance, we might predefine functions like `fetch(url)` for HTTP requests, `read_file(path)` for file I/O, or `set_timer(ms)` for timers, as placeholders. These would be registered as built-ins that currently perhaps print a “not implemented” message or return a dummy result. By stubbing them out now, we reserve their names and ensure any basic syntax around them is parseable. When Milestone 9 arrives, these functions will be fully implemented with asynchronous behavior. For example, a stub `fetch(url)` could return immediately with `nothing` or a placeholder object, and later it will perform a real network fetch using the async runtime ([docs.md](file://file-JkXXku8iwKMdy2TtL7T5Sv#:~:text=,program%2C%20enabling%20concurrency%20and%20I%2FO)). All such future I/O functions would also return a result that can indicate success or failure (likely integrating with WFL’s planned `try/when/otherwise` error handling). For now, document their intended use and leave them unimplemented in the code so that the structure is in place.

## Math Module (Numeric Functions)

The **math module** provides common numerical utilities. These are all pure functions that operate on numbers (WFL’s `Number` type, which is typically a floating-point value) and return numbers. They mirror typical math library functions, but exposed with simple names. All math functions expect numeric arguments; the typechecker will reject calls with texts or other types. The functions include:

- **`abs(number)`** – Returns the absolute value of a number.  
  • *Behavior:* If the input is negative, the result is its negation; if non-negative, the result is unchanged. For example, `abs(-5.2)` returns `5.2`, and `abs(3)` returns `3`.  
  • *Implementation:* Uses Rust’s `f64.abs()` under the hood. The intrinsic will extract the f64 from the `Value::Number` and apply `.abs()`, then wrap back in `Value::Number`.  
  • *Types:* Parameter must be Number; return is Number. If a non-number is passed, the typechecker will flag an error: *“abs expects a Number”*. This function could later be written in WFL itself (using an `if` to check sign), but since it’s trivial and performance is not a concern, migrating it is low-priority. Keeping it in Rust ensures consistency (especially for edge cases like `abs(-0.0)`).

- **`round(number)`** – Rounds a number to the nearest whole number (integer).  
  • *Behavior:* .5 and above typically round up, below .5 rounds down (standard round-half-up behavior). E.g., `round(3.2)` → `3.0`, `round(3.7)` → `4.0`, `round(-2.5)` → `-2.0` (depending on chosen rounding mode; standard Rust `f64.round()` rounds halves to nearest even or away from zero? By default, Rust’s `.round()` uses “bankers rounding” – nearest even). We should specify the intended behavior (for simplicity, you may choose away-from-zero rounding for positive .5).  
  • *Implementation:* Use `f64.round()`. This will produce a floating-point with `.0`. The result is returned as Number.  
  • *Types:* Parameter: Number; Return: Number. Non-numbers are not allowed. Rounding in WFL doesn’t produce a separate integer type – it still produces a Number (just without a fractional part).

- **`floor(number)`** – Returns the largest whole number less than or equal to the input (floor value).  
  • *Behavior:* E.g., `floor(3.9)` → `3.0`, `floor(-2.1)` → `-3.0`.  
  • *Implementation:* Use Rust’s `f64.floor()`.  
  • *Types:* Parameter must be Number; return Number.

- **`ceil(number)`** – Returns the smallest whole number greater than or equal to the input (ceiling value).  
  • *Behavior:* E.g., `ceil(3.1)` → `4.0`, `ceil(-2.9)` → `-2.0`.  
  • *Implementation:* Use Rust’s `f64.ceil()`.  
  • *Types:* Parameter: Number; return Number.

- **`random()`** – Returns a pseudorandom number.  
  • *Behavior:* If called with no arguments, it returns a Number between 0 and 1 (typically in the range `[0.0, 1.0)`). For example, `random()` might return `0.4723…`. In future, we might allow overloads like `random(max)` or `random(min, max)` to get integers in a range, but in this version it’s just a 0-1 floating point.  
  • *Implementation:* This will interface with a random generator. We can use Rust’s `rand` crate (e.g., `rand::thread_rng().gen::<f64>()`) to produce a random f64 in [0,1). The function then wraps it as a WFL Number. Because we want determinism in tests, ensure seeding or use of a reproducible generator if needed (maybe for now it’s fine to be truly random).  
  • *Types:* No parameters; return type is Number. The interpreter will call the function with an empty `args` vector. The typechecker knows that `random()` takes 0 arguments and produces a Number – calling it with any argument or assigning its result to a non-number variable is a compile error.

- **`clamp(number, min, max)`** – Constrains a number to lie between a minimum and maximum value.  
  • *Behavior:* Returns `min` if the number is below min, `max` if the number is above max, or the number itself if already in the range. For example, `clamp(5, 0, 10)` → `5`, `clamp(-3, 0, 10)` → `0` (because -3 is below the min of 0), `clamp(15, 0, 10)` → `10`.  
  • *Implementation:* Implemented with basic comparisons in Rust: `if x < min {min} else if x > max {max} else {x}`. Care should be taken that `min <= max`; if not, one could either swap them or return an error. For now, assume callers provide a proper range (we can document that if `min > max`, the behavior is undefined or we treat it as no clamping). This function will call Rust’s comparisons on f64.  
  • *Types:* All three parameters must be Number; return is Number. The typechecker will enforce that (giving an error if someone calls `clamp("5", 0, 10)` or mixes types). If the input number or the bounds are floating, the result will be that float (clamped). `clamp` could be reimplemented in WFL easily (using `if` and comparisons), so it’s a good candidate to migrate to WFL source once the language supports defining such utility functions. For now, we keep it intrinsic for efficiency and simplicity.

All math functions are pure (no side effects) and total (always return a result for valid input). They align with typical math library expectations, but using WFL’s simple syntax. Internally, each of these is a Rust function that takes a `Vec<Value>` of arguments and returns a `Result<Value, RuntimeError>`. The interpreter will map a call like `round(x)` to the native function behind the scenes. For example, when the interpreter evaluates a function call expression, it will find the `Value::NativeFunction(fn_ptr)` in the environment and invoke it ([src.md](file://file-KK8ZWKnRdcranrNnggRhxv#:~:text=Value%3A%3AFunction%28func%29%20%3D%3E%20self.call_function%28%26func%2C%20arg_values%2C%20,native_fn%28arg_values%29.map_err%28%7Ce%7C)). In our implementation, we’ll register these in a `math` module. 

*Example Implementation (Rust):* In `stdlib/math.rs`, define each function, e.g.:

```rust
pub fn native_abs(args: Vec<Value>) -> Result<Value, RuntimeError> {
    let x = expect_number(&args[0])?;              // helper to extract f64 or error
    Ok(Value::Number(x.abs()))
}
```

Similarly define `native_round`, `native_floor`, etc. Then provide a `pub fn register_math(env: &mut Environment)` that does:

```rust
env.define("abs", Value::NativeFunction(native_abs));
env.define("round", Value::NativeFunction(native_round));
… // and so on for each function
``` 

This way, the interpreter can add all math functions by calling `math::register_math` on startup. The type signatures for these functions (Number->Number, etc.) should be recorded in the typechecker’s built-in registry so the compiler knows how to type them.

## Text Module (String Functions)

The **text module** contains functions for manipulating and querying text strings (WFL’s `Text` type). These make common string operations available in an English-friendly way. All text functions treat strings as sequences of characters. Indexing is zero-based (consistent with typical programming practice using `0` for the first character, since WFL internally uses zero-based indexing for lists and text). The functions in this module include:

- **`length(text)`** – Returns the length of a text string.  
  • *Behavior:* Counts the number of characters in the string. For example, `length("hello")` returns `5`. An empty string returns `0`.  
  • *Implementation:* Use Rust’s string length. Caution: Rust’s `len()` on a `str` returns the byte length, which is the same as character count for ASCII but not for Unicode. Ideally, use `.chars().count()` to count actual Unicode code points. For simplicity, and because WFL might not fully handle complex Unicode yet, we can assume `.len()` suffices or document that it returns the number of bytes/characters. The intrinsic will extract the `&str` from `Value::Text` and count it.  
  • *Types:* Parameter must be Text; return is Number (an integer count). If called on a list or number, the typechecker errors out. Note that there is also a `length` function for lists (see List module below). WFL can allow the same function name to be *overloaded* on different types (text vs list) – the typechecker will resolve which one is intended based on the argument type. If the argument’s type is ambiguous or not a Text/List, it’s an error.

- **`to_uppercase(text)`** – Converts a string to all uppercase letters.  
  • *Behavior:* Returns a new text string where alphabetic characters are changed to upper case. Non-alphabet characters remain unchanged. E.g., `to_uppercase("Hello!")` → `"HELLO!"`.  
  • *Implementation:* Use Rust’s `.to_uppercase()` on the string (which handles Unicode casing for letters). This returns a new `String` which we convert to `Rc<str>` for `Value::Text`.  
  • *Types:* Parameter: Text; Return: Text. (If needed in the future, we might provide similar functionality for lists of characters, but in WFL a string is not just a list of chars, it’s its own type.) The typechecker ensures the input is Text. This function is straightforward enough that if WFL had a way to iterate characters, it could be written in WFL, but it’s best kept as a built-in due to Unicode complexity.

- **`to_lowercase(text)`** – Converts a string to all lowercase letters.  
  • *Behavior:* E.g., `to_lowercase("Hello!")` → `"hello!"`.  
  • *Implementation:* Rust’s `.to_lowercase()`.  
  • *Types:* Parameter: Text; Return: Text. Only text input is allowed.

- **`contains(text, substring)`** – Checks if a given substring appears within a text.  
  • *Behavior:* Returns a Boolean true/false. For example, `contains("Hello world", "world")` → `true`, `contains("Hello", "bye")` → `false`. This is case-sensitive (since no mention of ignoring case).  
  • *Implementation:* Use Rust’s `str.contains`. The intrinsic will ensure both arguments are Text, then do `text.contains(substring)`. In Rust, since `substring` is an `Rc<str>`, you can get `&str` via `&*substring`. The result is `Value::Bool(true)` or `Value::Bool(false)`.  
  • *Types:* Both parameters must be Text; return is Boolean. The typechecker will enforce that (it will also allow using the same `contains` name for list membership as described in the List module, selecting the correct overload by context). If a user mistakenly calls `contains(text, 5)`, the compiler will say the second argument should be Text, not Number.  
  • *Note:* This function overlaps with the list `contains` (to check list membership). In our implementation, we actually use one underlying native function that can handle either case by checking the type of the first argument ([src.md](file://file-KK8ZWKnRdcranrNnggRhxv#:~:text=match%20%28left%2C%20right%29%20,Ok%28Value%3A%3ABool%28false)) ([src.md](file://file-KK8ZWKnRdcranrNnggRhxv#:~:text=%28Value%3A%3AText%28text%29%2C%20Value%3A%3AText%28substring%29%29%20%3D%3E%20%7B%20Ok%28Value%3A%3ABool%28text.contains%28%26,a.type_name%28%29%2C%20b.type_name%28%29)). However, in the WFL source, it appears as the same name `contains` working on different types. Document this as a polymorphic behavior: *contains(X, Y)* works on Text (substring search) or on List (membership test). The type system can distinguish them: if X is Text, Y must be Text; if X is List, Y can be any element type.

- **`substring(text, start, end)`** – Extracts a portion of a string from the given start index up to (but not including) the end index.  
  • *Behavior:* Returns the substring of `text` beginning at index `start` and ending just *before* index `end`. For example, if `text` is `"abcdef"`, then `substring(text, 1, 4)` returns `"bcd"` (assuming 0-based indexing: index 1 = 'b', index 4 is 'e' but not included). If `start == end`, the result is an empty string. If `start` is 0 and `end` is the length of the string, the whole string is returned.  
  • *Implementation:* We can leverage Rust string slicing: in Rust, `&text[start..end]` (provided `start` and `end` are in range and align on character boundaries) will give the desired substring. We need to perform bounds checking: if `start < 0` or `end > text.length` or `start > end`, we should handle it. Likely, out-of-range should cause a runtime error (consistent with how list indexing errors on out-of-bounds ([src.md](file://file-KK8ZWKnRdcranrNnggRhxv#:~:text=match%20at%20L1743%20,idx%2C%20list.len))). We could also clamp the indices to valid range, but an error is clearer for a programmer mistake. So the intrinsic will check: 0 ≤ start ≤ end ≤ len, otherwise return a `RuntimeError` (which will likely halt execution or be caught by a try/when block if implemented). If indices are valid, use `.get(start..end)` on the Rust string (which returns an `Option<&str>` – handle None as error if indices cut a Unicode code point, but since we count in code points via `.chars()`, we might need to map char index to byte index first. To keep it simpler, we might assume ASCII or treat indices as byte indices for now). The resulting str slice is then converted to Value::Text.  
  • *Types:* Parameters: one Text, two Numbers (start index and end index); Return: Text. If the indices are not numbers, or the first argument not text, it’s a compile-time type error. The typechecker also can enforce that these functions get exactly 3 arguments (one reason to integrate built-in knowledge into the compiler). In the future, we could offer variations like `substring(text, start)` meaning from start to end of string, but right now we stick to requiring both bounds. This function could technically be implemented in WFL using loops and building a string, but that would be inefficient and complex for multi-char handling, so it’s provided as a native function.

The text module functions make string manipulation tasks straightforward. All of them return new values or information without modifying the original string (strings in WFL are immutable, as typical in many languages). If someday WFL supports **mutable strings or in-place editing**, those would be separate functions (or methods) but currently not in scope.

*Note:* In some languages, strings have methods like `.length` or `.substring`. In WFL, we choose free functions to keep the syntax uniform and simple (no dot notation). This also allows function-style natural phrasing. For instance, one could read `length(myText)` as “get the length of myText,” which is quite clear.

## List Module (List and Collection Functions)

The **list module** offers operations on WFL’s list type (an ordered collection of values). WFL lists can hold elements of any type (they are not typed homogeneous lists in the surface language, though internally the typechecker can track an element type if consistent). These functions allow basic manipulation like getting the size of a list, adding or removing elements, and searching within a list. Functions include:

- **`length(list)`** – Returns the number of elements in a list.  
  • *Behavior:* E.g., if `myList` is `[1, 2, 3]`, `length(myList)` returns `3`. An empty list returns `0`.  
  • *Implementation:* Access the internal vector length. The interpreter represents a list as `Value::List(Rc<RefCell<Vec<Value>>>)` ([src.md](file://file-KK8ZWKnRdcranrNnggRhxv#:~:text=Text%28Rc,FunctionValue%3E%29%2C%20NativeFunction%28NativeFunction%29%2C%20Null%2C)). We can do `list_rc.borrow().len()` to get the length (this is O(1)). Return as a Number (floating-point, but effectively an integer value).  
  • *Types:* Parameter must be a List; return is Number. This function name is the same as the text length function, but the typechecker will disambiguate based on argument type. If the argument type is `List<T>` (any T), it picks this list version. If someone mistakenly passes a Text to this version or a List to the text version, the compiler will still catch the type mismatch. Internally, we might implement a single native `length` function that checks the type of its argument and handles both strings and lists (similar to `contains`), but it may be cleaner to register two separate built-ins under different internal names or a single name with logic. Either way, typechecking ensures correct usage.

- **`push(list, item)`** – Appends an item to the end of a list.  
  • *Behavior:* This modifies the list by adding the new element as the last element. For example, if `myList` is `[1, 2]`, then after `push(myList, 3)`, the list becomes `[1, 2, 3]`.  
  • *Implementation:* The intrinsic will take the list (as a mutable reference internally) and the item. It will do `list_rc.borrow_mut().push(item_value)`. Because the list is an `Rc<RefCell<Vec<Value>>>`, we obtain a `RefMut` to push. This is safe as long as no other references are currently borrowed – the interpreter should only call this when not already iterating the same list, etc. We should be mindful of Rust’s borrow rules here (but the interpreter’s single-threaded, depth-first execution ensures only one active borrow usually). After pushing, we return `Value::Null` (nothing) to indicate completion. We choose to return nothing rather than the list itself to emphasize that this is a procedure with side effect. (If we returned the list, it would just be the same list object; that’s not particularly useful unless chaining, which WFL doesn’t emphasize.)  
  • *Types:* First parameter must be a List (of some element type `T`), second parameter must be of type `T` (or compatible with `T`). The typechecker will enforce that the item’s type matches the list’s element type. If the list’s element type was not known (e.g., an empty list initially has `Unknown` type), the typechecker can unify it to the item’s type upon a push ([src.md](file://file-KK8ZWKnRdcranrNnggRhxv#:~:text=Type%3A%3AList%28item_type%29%20%3D,self.type_error)) ([src.md](file://file-KK8ZWKnRdcranrNnggRhxv#:~:text=Type%3A%3AList%28item_type%29%20%3D,be%20a%20number%2C%20got)). For example, if `myList` was declared but empty (List of Unknown), and we do `push(myList, 5)` (5 is Number), then `myList`’s type becomes List<Number>. If a second push tries to push a Text, the typechecker will error that a Text can’t be added to a List<Number> ([src.md](file://file-KK8ZWKnRdcranrNnggRhxv#:~:text=match%20at%20L5289%20Type%3A%3AList%28item_type%29%20%3D,self.type_error)). Return type is Nothing (the push operation doesn’t produce a value to use in expressions). So `store x as push(list, item)` would be meaningless (the typechecker can flag that storing a Nothing is not useful). Push should be used as a standalone statement or followed by checking the list itself.

- **`pop(list)`** – Removes the last element from a list and returns it.  
  • *Behavior:* If the list has elements, this removes the tail element and returns it. For example, if `myList = [10, 20, 30]`, then `pop(myList)` will remove `30` from the list, leaving `[10, 20]`, and the call evaluates to `30` (which the program can use).  
  • *Edge cases:* If the list is empty, there is nothing to pop. We have two possible approaches: (1) throw a runtime error (like "Cannot pop from an empty list"), or (2) return a special value (like `nothing`) to indicate emptiness. Given WFL’s beginner-friendly angle, returning `nothing` might be safer — it avoids crashing the program and allows the user to handle the condition via `is_nothing`. We can implement it such that if the list is empty, it returns `Null` (nothing) instead of error. We should document this clearly.  
  • *Implementation:* Use `list_rc.borrow_mut().pop()`. In Rust, `Vec::pop()` returns an `Option<Value>` – if it’s `Some(val)`, wrap that as a WFL `Value` result; if it’s `None` (empty list), then return `Value::Null`. If we choose to error on empty list instead, we would check and return a `RuntimeError`. The recommended approach is to return Null on empty to let users check with `is_nothing`.  
  • *Types:* Parameter must be a List of some type `T`. The return type is `T` for non-empty case, but if we’re allowing `nothing` on empty, effectively it’s `T or Nothing`. WFL’s static type system doesn’t currently have union types to express “T or Nothing”. We handle this by saying the static return type is `T` (or Unknown if T is unknown), and we document that at runtime it *might* be nothing if the list was empty. Practically, a user should check `is_nothing` before treating the result as a T. If they don’t check and assume an element, they might get a runtime error later when using a `Nothing` as if it were a T (or the typechecker might force a check by not letting a `Nothing` flow where a non-nothing is required). This is a bit advanced; for now, the simplest model: assume the list has at least one element when using `pop` (or check it). The typechecker can treat `pop(list: List<T>) -> T` as its signature. If someone tries to assign the result to a variable of a different type, that’s a compile error. If they try to pop from a list of Unknown type and use the result, the result type will be Unknown until more context is given or it might cause a type inference failure if not resolved. In implementation, since we return a Value, it’s fine. Just be sure to handle the empty case.

- **`contains(list, item)`** – Checks if an item exists in the list.  
  • *Behavior:* Returns `true` if the list contains an element equal to the given item, otherwise `false`. Equality is defined as WFL’s normal equality comparison (which should handle numbers vs numbers, text vs text, etc., and possibly deep equality for lists/objects). For example, `contains([1,2,3], 2)` → `true`; `contains([1,2,3], 5)` → `false`. If the list is empty, it always returns false (and does not error).  
  • *Implementation:* Iterate through the list’s elements and compare each to the target item. The interpreter already has a helper `is_equal(a, b)` for comparing Values deeply (to handle nested lists, etc.) ([src.md](file://file-KK8ZWKnRdcranrNnggRhxv#:~:text=%28Value%3A%3AList%28list_rc%29%2C%20item%29%20%3D,Ok%28Value%3A%3ABool%28false%29%29)) ([src.md](file://file-KK8ZWKnRdcranrNnggRhxv#:~:text=for%20value%20in%20list.iter%28%29%20,borrow)). The native function will borrow the list (`list_rc.borrow()`) and loop. If any element is equal to the item (using `is_equal` to compare Value enums properly), return `Value::Bool(true)`. If the loop finishes, return `Value::Bool(false)`. This is O(n) linear search.  
  • *Types:* Parameter 1: List<T>; Parameter 2: should be type T (the type of elements in that list). The typechecker will enforce the second argument type matches the list’s element type (or if the list element type is unknown, it can treat the call as a context that *requires* the list to be of the second argument’s type, potentially inferring it). Return type is Boolean. If someone tries `contains(someList, wrongTypeValue)`, a compile error occurs (e.g., “cannot check contains: list of Text with Number”). This function shares its name with the text `contains` – the typechecker uses the first argument’s type to decide if this is the list version. Internally, we can actually implement one `native_contains` that covers both cases by pattern-matching on the types ([src.md](file://file-KK8ZWKnRdcranrNnggRhxv#:~:text=%28Value%3A%3AObject%28obj_rc%29%2C%20Value%3A%3AText%28key%29%29%20%3D,Err%28RuntimeError%3A%3Anew%28%20format)), but from the developer’s perspective it’s the same `contains` keyword.

- **`index_of(list, item)`** – Finds the index of an item in the list.  
  • *Behavior:* Searches the list for the first element equal to the given item. If found, returns its index (0-based) as a Number. If not found, returns `nothing` (since there is no valid index). For example, `index_of([10, 20, 30], 20)` → `1` (because 20 is at index 1), and `index_of([10, 20, 30], 5)` → `nothing` (5 is not in the list).  
  • *Implementation:* Loop through list elements (similar to `contains`). Track the index (starting at 0); for each element, if `is_equal(element, item)` is true, return the current index as a Number value. If the loop ends with no match, return `Value::Null` to indicate not found. This is again O(n).  
  • *Types:* Parameter 1: List<T>; Parameter 2: T (same type requirement as `contains`). The return in the found case is a Number (index). If not found, we return Nothing. So effectively the return is “Number or Nothing”. As with `pop`, the static type system doesn’t natively represent this union. We will specify the return type as Number for the typechecker, but we also tag that it *may* return Nothing. In practice, a user should do something like:  
    ```wfl
    store idx as index_of(myList, "foo")
    if is_nothing(idx) then
        // "foo" not in list
    otherwise
        // idx is a valid number index
    ```  
    This way, the code safely handles the `nothing` case. We can enhance the typechecker later to understand that after an `is_nothing` check, `idx` is a Number, but initially, just ensuring the usage is correct is enough. If the user tries to use the result of `index_of` as a number without a check, it’s technically fine if they assume it’s there – if it wasn’t, it would be a runtime `Null` which if used in arithmetic might cause a runtime error. We document this behavior clearly. (An alternative design is to return -1 as a Number if not found, to avoid using `nothing`. However, using `nothing` is more idiomatic in WFL to indicate absence, and we provided `is_nothing` for this reason. We will stick to returning nothing for “not found”.)

All list functions that modify the list (`push` and `pop`) operate on the actual list passed in (since lists are reference types under the hood, the changes are visible to all references to that list). The others (`length`, `contains`, `index_of`) do not modify the list, they just read it. There is no need to copy lists for these operations; we always operate by reference. WFL’s lists are akin to Python lists or JavaScript arrays in this regard.

Memory and safety: Because WFL lists are internally reference-counted, adding and removing elements is safe. One must be cautious about unusual cases like a list containing itself (which could create a reference cycle). For example, if a user did something like `push(myList, myList)`, the interpreter would insert the list into itself. The implementation should handle it (the interpreter’s data model allows it, but it could lead to a memory leak since that cycle would never drop to zero references) ([docs.md](file://file-JkXXku8iwKMdy2TtL7T5Sv#:~:text=a.push%28a%29%3B%20,into%20itself%2C%20we)). This is an edge case; we might choose to disallow pushing a list into itself at runtime. As an initial pass, it’s enough to note it as a potential pitfall but not explicitly handle it unless we want to be extra safe.

## Integrating Built-ins into the Interpreter and Typechecker

With the modules defined, we need to integrate them so that the interpreter knows about these functions and the compiler enforces their usage. The process involves **registering the functions in the global environment**, and updating the **typechecker’s knowledge base** of function signatures.

**Modular File Layout:** In the Rust project, organize the standard library functions under a `stdlib` module. For example:
```
src/
 └── stdlib/
     ├── core.rs    (print, type_of, is_nothing, and future stubs like fetch)
     ├── math.rs    (abs, round, floor, ceil, random, clamp)
     ├── text.rs    (length (text), to_uppercase, to_lowercase, contains (text), substring)
     └── list.rs    (length (list), push, pop, contains (list), index_of)
```
Each of these files will contain the Rust implementations (as `fn native_func(args: Vec<Value>) -> Result<Value, RuntimeError>`) for each operation, plus a helper function to register them. For example, `core.rs` might have:
```rust
pub fn register_core(env: &mut Environment) {
    env.define("print", Value::NativeFunction(native_print));
    env.define("type_of", Value::NativeFunction(native_type_of));
    env.define("is_nothing", Value::NativeFunction(native_is_nothing));
    // (and stub registrations like env.define("fetch", Value::NativeFunction(native_fetch_stub)) for future I/O)
}
```
And similarly `register_math`, `register_text`, `register_list` in their respective modules.

**Registering with Interpreter:** The interpreter (likely in its `Interpreter::new()` or initialization routine) should call all these register functions to populate the global scope. For instance:
```rust
let global_env = Environment::new_global();
{
    let mut env = global_env.borrow_mut();
    core::register_core(&mut env);
    math::register_math(&mut env);
    text::register_text(&mut env);
    list::register_list(&mut env);
}
Interpreter { global_env }
``` 
This ensures that as soon as the interpreter starts, names like "print", "abs", "length", etc., are already defined in the global environment as native functions. (In the current implementation, we saw an example of registering `display` like this ([src.md](file://file-KK8ZWKnRdcranrNnggRhxv#:~:text=,Value%3A%3ANativeFunction%28Self%3A%3Anative_display%29%29%3B)); we will extend that to all standard functions.) The interpreter’s call execution logic is already set up to detect `Value::NativeFunction` and execute it ([src.md](file://file-KK8ZWKnRdcranrNnggRhxv#:~:text=Value%3A%3AFunction%28func%29%20%3D%3E%20self.call_function%28%26func%2C%20arg_values%2C%20,native_fn%28arg_values%29.map_err%28%7Ce%7C)). When a user calls one of these functions in WFL code, the interpreter will invoke our Rust implementation and get back a `Value` result to continue with.

**Typechecker Updates:** The static typechecker must be aware of these functions so that it knows what types they expect and return. There are a couple of ways to achieve this:

- *Prepopulate Symbol Table:* When the compiler (or typechecker) initializes, we can insert entries for each built-in function into the global symbol table of the typechecker. For example, add a symbol "print" with type `Function([Type::Unknown], Type::Nothing)` meaning it takes any type and returns Nothing. Or "abs" with type `Function([Type::Number], Type::Number)`. These could be hard-coded or generated from a table. This way, when the parser or semantic analyzer encounters `print(x)` in code, it treats "print" as a known function with a signature. If the user defined their own function named "print", the built-in should probably be shadowable (though typically you wouldn’t redefine core functions). We might mark them as reserved to avoid confusion. 
- *Special-case in Type Inference:* Alternatively, during type checking when encountering a FunctionCall expression, if the function name matches one of the known built-ins, directly apply the type rules. For instance, if seeing a call to "round", the typechecker can assert the argument must be Number and set the expression’s type to Number. This approach is a bit more scattered (you’d have `if name == "round"` branches in code). A cleaner approach is the previous one (symbol table of built-ins), which is more data-driven.

Given maintainability, having a centralized registry is good. We can create a mapping, e.g., in the typechecker module:
```rust
let builtins: HashMap<String, Type> = [
   ("print", Type::Function { parameters: vec![Type::Unknown], return_type: Box::new(Type::Nothing) }),
   ("type_of", Type::Function { parameters: vec![Type::Unknown], return_type: Box::new(Type::Text) }),
   ("is_nothing", Type::Function { parameters: vec![Type::Unknown], return_type: Box::new(Type::Boolean) }),
   ("abs", Type::Function { parameters: vec![Type::Number], return_type: Box::new(Type::Number) }),
   // ... and so on for each
].into_iter().map(|(name, sig)| (name.to_string(), sig)).collect();
```
Then, in the typechecker, when resolving an identifier or function call, check this map. For example, if an `Expression::FunctionCall` has a function which is an `Expression::Variable("abs", ...)`, we find "abs" in the map, retrieve its expected parameter type list (`[Number]`) and return type (`Number`). We then verify the provided arguments match the expected types (there should be exactly one argument, and its type should be Number or implicitly convertible). If not, we emit a type error like “abs() expects a Number” ([docs.md](file://file-JkXXku8iwKMdy2TtL7T5Sv#:~:text=indexing%2C%20etc.%20,unless%20you%20allow%20implicit%20conversion)). If it matches, we mark the call’s type as Number. In the case of overloaded names (`length` or `contains` which work on multiple types), we might represent them in the map as taking a type variable or as separate entries distinguished by context. Simpler is to implement logic: if name == "length", check if arg type is Text or List – allow either and return Number. If name == "contains", if first arg type is List<T> then second must be T (return Bool), if first is Text then second must be Text (return Bool). These conditionals can be coded directly, or we can have pseudo-overload entries. Since we already have to branch in code for such overloaded behavior, it’s acceptable to handle those two in code.

**Ensuring Correct Usage:** After integration, we should add tests and documentation to verify everything. For each built-in, create small WFL snippets to ensure they work:
- e.g., `print("Hello")` should output Hello (and return nothing).
- `abs(-4)` yields 4; `abs("foo")` should be a compile-time type error.
- `length([1,2,3])` returns 3; `length("test")` returns 4; `length(42)` is a type error.
- `contains("abc", "b")` true, `contains("abc", "z")` false; `contains([1,2,3], 2)` true.
- Pushing wrong type: 
  ```wfl
  store nums as [1,2,3]
  push(nums, "four")
  ``` 
  should produce a type error at compile time (can't push Text into List of Number). The typechecker’s compatibility check on `push` should catch that ([src.md](file://file-KK8ZWKnRdcranrNnggRhxv#:~:text=match%20at%20L5289%20Type%3A%3AList%28item_type%29%20%3D,self.type_error)).
- Popping from empty:
  ```wfl
  store emptyList as []
  store x as pop(emptyList)
  ``` 
  should set x to nothing (we can verify by `is_nothing(x)` returning true).
- `index_of([10,20,30], 20)` returns 1; `index_of([10,20], 99)` returns nothing (check via is_nothing).

All these should be added to the test suite ([docs.md](file://file-JkXXku8iwKMdy2TtL7T5Sv#:~:text=,checker%20or%20runtime%20errors%20appropriately)) to prevent regressions. Also, documenting them in the WFL standard library reference is important for users, but as an AI agent developer, your focus is implementation and ensuring they align with WFL’s style.

**Migration to WFL Source:** In the long run, some of these built-ins can be re-written in WFL itself once the language is powerful enough to define its own standard library routines. This is a step toward self-hosting parts of the standard library. Likely candidates for migration include:
- *Pure functions that are easily expressed in WFL:* e.g., `clamp(x,min,max)` (can be written with `check if x < min then give back min otherwise if x > max then give back max otherwise give back x end check` in WFL), or `abs(x)` (check if negative then multiply by -1). These don’t require system calls or high performance, so they could be moved out of Rust. 
- *Certain string operations:* One could implement `contains(text, sub)` in WFL by looping over indices and checking substrings, but that’s cumbersome and slow compared to Rust’s optimized version, so this one is better kept in Rust. Simpler ones like `length(text)` could theoretically be a loop counting characters, but again that’s inefficient in WFL. We’d likely keep most string ops in Rust for correctness (especially with Unicode).
- *List operations:* Technically, one could implement `push` and `pop` in WFL if the language allowed direct manipulation of list internals, but since lists are a built-in type, it’s better to keep their fundamental operations in Rust. `contains(list, item)` could be written using a `for each` loop in WFL, and `index_of` similarly, once we have looping constructs – those could be candidates to move to WFL for clarity, but we’d lose performance. Initially, we implement all in Rust for simplicity ([docs.md](file://file-JkXXku8iwKMdy2TtL7T5Sv#:~:text=,Rust%20implementations%20of%20needed%20functionalities)), and we might revisit later.

Essentially, as WFL matures, we might introduce a **prelude** or standard library written in WFL that is loaded at startup (so users can even view or modify it if needed). At that point, some of these functions would just be WFL code. Until then, implementing in Rust is the straightforward approach ([docs.md](file://file-JkXXku8iwKMdy2TtL7T5Sv#:~:text=standard%20library%20written%20in%20WFL,defined%20ones)). The design we’ve put in place (modular separation and clear API boundaries) will make it easier to swap out implementations. For example, we could implement a WFL version of `abs` and have the interpreter call that instead, or simply remove the intrinsic and let the WFL code overshadow it.

Finally, all built-in functions chosen align with WFL’s philosophy of being **clear and beginner-friendly** ([docs.md](file://file-JkXXku8iwKMdy2TtL7T5Sv#:~:text=,or%20random%20number%20generator%20usage)). There are no cryptic names or unexpected behaviors. Each function should include doc-comments (in Rust) explaining its usage, so that if an AI like Devin is generating documentation or help tips, it can surface those. With the core, math, text, and list modules in place, WFL will have a solid foundation of functionality for users to perform common tasks without needing to write everything from scratch. This achieves Milestone 8’s goal of a basic standard library ([docs.md](file://file-JkXXku8iwKMdy2TtL7T5Sv#:~:text=,be%20aware%20of%20their%20signatures)), setting the stage for more advanced features (async I/O in Milestone 9, and beyond).

## Pattern Module (Text Pattern Matching)

The **pattern module** provides a natural language approach to text pattern matching, allowing users to work with patterns in a more intuitive way than traditional regular expressions. This module implements the "Not Your Father's Regex" concept, making pattern matching accessible to beginners while still being powerful enough for complex text processing tasks.

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

## Args Module (Command Line Arguments)

The **args module** provides access to command-line arguments passed to WFL scripts. This enables WFL programs to accept input parameters and behave differently based on user-provided options, making scripts more flexible and reusable.

- **`args()`** – Returns a list of command-line arguments passed to the script.  
  • *Behavior:* Returns all command-line arguments as a list of text strings, excluding the program name itself. For example, if a script is run with `wfl script.wfl --output file.txt --verbose`, then `args()` returns `["--output", "file.txt", "--verbose"]`. If no arguments are provided, returns an empty list.  
  • *Parameters:* None. This function takes no arguments.  
  • *Return:* List of Text values containing the command-line arguments.  
  • *Implementation:* Uses Rust's `std::env::args().skip(1)` to get arguments excluding the program name. Each argument is converted to a WFL Text value and collected into a List.  
  • *Typechecking:* No parameters required; return type is `List<Text>`. Calling with any arguments is a type error.  
  • *Example Usage:*
    ```wfl
    store cli_args as args()
    store arg_count as length(cli_args)
    
    check if arg_count is greater than 0:
        store first_arg as index(cli_args, 0)
        display "First argument: " plus first_arg
    otherwise:
        display "No arguments provided"
    end check
    ```

## Filesystem Module (File and Directory Operations)

The **filesystem module** provides functions for interacting with the file system, including directory listing and file metadata access. These functions enable WFL scripts to discover files, check modification times, and work with file system structures.

- **`dirlist(path, recursive, pattern)`** – Lists files in a directory with optional filtering.  
  • *Behavior:* Returns a list of file paths found in the specified directory. The `recursive` parameter controls whether to search subdirectories. The `pattern` parameter allows filtering files using glob patterns (e.g., "*.md", "**/*.rs"). If `recursive` is true, searches all subdirectories. If `pattern` is provided, only files matching the pattern are included.  
  • *Parameters:* 
    - `path` (Text): Directory path to search
    - `recursive` (Boolean): Whether to search subdirectories recursively
    - `pattern` (Text, optional): Glob pattern to filter files (can be null)
  • *Return:* List of Text values containing file paths.  
  • *Implementation:* Uses Rust's `std::fs` and `glob` functionality to traverse directories and match patterns. Handles both simple patterns like "*.txt" and complex patterns like "**/*.rs" for recursive matching.  
  • *Typechecking:* First parameter must be Text (directory path), second must be Boolean (recursive flag), third must be Text or null (pattern). Return type is `List<Text>`.  
  • *Example Usage:*
    ```wfl
    store md_files as dirlist("./docs", true, "*.md")
    store file_count as length(md_files)
    display "Found " plus file_count plus " markdown files"
    
    for each file in md_files:
        display "File: " plus file
    end for
    ```

- **`filemtime(path)`** – Returns the modification time of a file as a Unix timestamp.  
  • *Behavior:* Gets the last modification time of the specified file and returns it as a number representing seconds since Unix epoch (January 1, 1970). This is useful for sorting files by modification time or checking if files have been updated.  
  • *Parameters:* `path` (Text): File path to check.  
  • *Return:* Number representing Unix timestamp (seconds since epoch).  
  • *Implementation:* Uses Rust's `std::fs::metadata()` to get file metadata, then extracts the modification time and converts it to a Unix timestamp as f64.  
  • *Typechecking:* Parameter must be Text (file path). Return type is Number. If the file doesn't exist or cannot be accessed, results in a runtime error.  
  • *Example Usage:*
    ```wfl
    store file_time as filemtime("document.txt")
    display "File was last modified at timestamp: " plus file_time
    
    store current_time as time()
    store age_seconds as current_time minus file_time
    display "File is " plus age_seconds plus " seconds old"
    ```

## Path Module (Path Manipulation Utilities)

The **path module** provides utilities for working with file paths in a cross-platform manner. These functions help with extracting parts of paths, combining path components, and manipulating file system paths safely across different operating systems.

- **`basename(path)`** – Extracts the filename from a file path.  
  • *Behavior:* Returns the final component of a path, typically the filename. For example, `basename("/home/user/document.txt")` returns `"document.txt"`, and `basename("/home/user/")` returns `"user"`. Works with both Unix-style and Windows-style paths.  
  • *Parameters:* `path` (Text): File or directory path.  
  • *Return:* Text containing the base name (filename or directory name).  
  • *Implementation:* Uses Rust's `std::path::Path::file_name()` to extract the final path component, handling cross-platform path separators automatically.  
  • *Typechecking:* Parameter must be Text. Return type is Text.  
  • *Example Usage:*
    ```wfl
    store full_path as "/home/user/documents/report.pdf"
    store filename as basename(full_path)
    display "Filename: " plus filename  // Outputs: "report.pdf"
    ```

- **`dirname(path)`** – Extracts the directory portion from a file path.  
  • *Behavior:* Returns the directory part of a path, excluding the final component. For example, `dirname("/home/user/document.txt")` returns `"/home/user"`, and `dirname("document.txt")` returns `"."` (current directory). Handles both absolute and relative paths.  
  • *Parameters:* `path` (Text): File or directory path.  
  • *Return:* Text containing the directory path.  
  • *Implementation:* Uses Rust's `std::path::Path::parent()` to get the parent directory, converting the result to a string representation.  
  • *Typechecking:* Parameter must be Text. Return type is Text.  
  • *Example Usage:*
    ```wfl
    store full_path as "/home/user/documents/report.pdf"
    store directory as dirname(full_path)
    display "Directory: " plus directory  // Outputs: "/home/user/documents"
    ```

- **`pathjoin(component1, component2, ...)`** – Combines path components into a single path.  
  • *Behavior:* Takes multiple path components and joins them using the appropriate path separator for the current operating system. Automatically handles path separators (forward slashes on Unix, backslashes on Windows). For example, `pathjoin("/home", "user", "documents")` returns `"/home/user/documents"` on Unix systems.  
  • *Parameters:* Variable number of Text arguments representing path components to join.  
  • *Return:* Text containing the combined path.  
  • *Implementation:* Uses Rust's `std::path::PathBuf::join()` to combine path components, ensuring proper path separator usage for the target platform.  
  • *Typechecking:* All parameters must be Text. Return type is Text. Requires at least one argument.  
  • *Example Usage:*
    ```wfl
    store base_dir as "/home/user"
    store subdir as "documents"
    store filename as "report.pdf"
    store full_path as pathjoin(base_dir, subdir, filename)
    display "Full path: " plus full_path  // Outputs: "/home/user/documents/report.pdf"
    ```

## Enhanced List Module (Extended Sorting)

The existing **list module** has been enhanced with additional sorting capabilities to support more complex data manipulation scenarios.

- **`sort(list, key_function, reverse)`** – Sorts a list with optional key function and reverse order.  
  • *Behavior:* Sorts the elements of a list in ascending order by default. The `key_function` parameter (currently reserved for future implementation) will allow custom sorting criteria. The `reverse` parameter, when true, sorts in descending order. For mixed-type lists, uses string representation for comparison.  
  • *Parameters:* 
    - `list` (List): The list to sort
    - `key_function` (Function, optional): Custom sorting function (currently null/unimplemented)
    - `reverse` (Boolean, optional): Whether to sort in descending order (default: false)
  • *Return:* List containing the sorted elements (creates a new list, original is unchanged).  
  • *Implementation:* Uses Rust's sorting algorithms with custom comparison logic. Numbers are sorted numerically, text alphabetically, and other types by their string representation. The reverse parameter controls sort order.  
  • *Typechecking:* First parameter must be List. Second parameter must be Function or null. Third parameter must be Boolean. Return type is List with the same element type as input.  
  • *Example Usage:*
    ```wfl
    create list as numbers
    push with numbers and 3
    push with numbers and 1
    push with numbers and 4
    push with numbers and 2
    
    store ascending as sort(numbers)
    store descending as sort(numbers, null, true)
    
    display "Ascending: " plus ascending   // [1, 2, 3, 4]
    display "Descending: " plus descending // [4, 3, 2, 1]
    ```

These new modules extend WFL's capabilities for system interaction, file management, and data processing while maintaining the language's commitment to readable, natural syntax. All functions follow WFL's error handling conventions and integrate seamlessly with the existing type system.
