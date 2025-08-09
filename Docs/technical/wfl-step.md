Alright, I’ll engineer a step-by-step execution mode in the `WebFirstLanguage/wfl` interpreter where each script line becomes a single step. This mode will be enabled via a CLI flag and prompt users after each step with a "continue (y/n)" message. I’ll decide on the best granularity for boot sequence steps, and design outputs that show the current instruction, minimal call context, and variable state.

I’ll also ensure the system is tested and submit a clean PR, as per your request. Hang tight while I work this up!


# Step-by-Step Execution Mode Implementation Plan

## Step Definition

**Boot Sequence Steps:** We will break the interpreter's startup into clear steps. For example:

1. **Loading Configuration** – reading the global and local config files.
2. **Initializing Interpreter** – setting up the global environment, loading standard library functions, etc.

   * This occurs when we create the `Interpreter` (which calls `Environment::new_global()` and registers built-ins). We’ll treat this initialization as its own step.
3. *(Optional)* We could also consider additional boot steps such as parsing the script into an AST, semantic analysis, and type checking, but since these are pre-execution compile-phase tasks, we may keep them as a single preparatory step to avoid too many prompts. The key boot steps to highlight are configuration load and interpreter initialization.

**Script Execution Steps:** We define each executed **statement** in the user’s script as a step. In practice, this means each top-level statement (usually one per line in WFL) will count as one step. Additionally, we will extend this stepping into any function calls or loops:

* When stepping through a function (such as the `main` function or any user-defined “action”), each statement inside that function’s body will also be treated as a step.
* Similarly, each iteration of a loop will pause after executing the loop’s body (since the loop body is a block of statements executed stepwise).

This way, whether the interpreter is executing top-level code or inside a called function, **each statement execution is one step**.

For example, if the script has 5 top-level statements and a `main` function with 3 statements, the interpreter will pause after each of those 8 statements (assuming `main` is called). The “boot” steps (config loading, init) will be handled before script statements begin.

## Activation Mechanism (`--step` Flag)

We will add a new CLI flag `--step` to enable step-by-step mode. Changes in `src/main.rs` will include:

* **Argument Parsing:** Introduce a boolean `step_mode` (default `false`). In the argument parsing loop, handle `"--step"` similarly to existing flags. For example:

  ```rust
  "--step" => {
      if lint_mode || analyze_mode || fix_mode || config_check_mode || config_fix_mode {
          eprintln!("Error: --step cannot be combined with other modes like --lint or --analyze");
          process::exit(2);
      }
      step_mode = true;
      i += 1;
      // (No file_path consumed here; the next arg, if any, should be the script path)
  }
  ```

  This ensures `--step` isn’t used alongside incompatible options. The flag will simply set `step_mode=true` for later use.

* **Help Text:** Update the usage message to document `--step`. In `print_help()`, under the FLAGS section, add a line such as:

  ```rust
  println!("    --step             Run the interpreter in step-by-step execution mode");
  ```

* **Passing the Mode:** After parsing arguments, we need to convey `step_mode` to the interpreter. We can do this by:

  * Storing it in the loaded configuration (e.g., add a field in `WflConfig` or use the global `CONFIG`), **or**
  * Simply using it to configure the interpreter directly.

  A straightforward approach is to add a setter or parameter for the interpreter. For instance, we might add a method `Interpreter::enable_step_mode()` or extend `Interpreter::with_timeout()` to accept the flag. The simplest implementation: after creating the interpreter (`Interpreter::with_timeout`), set an internal flag:

  ```rust
  let mut interpreter = Interpreter::with_timeout(config.timeout_seconds);
  if step_mode {
      interpreter.set_step_mode(true);
  }
  ```

  We will add a boolean field `step_mode` to the `Interpreter` struct (defaulting to false) to track this state. This field will gate the step-by-step behavior inside the interpreter.

## Step Control and Execution Flow

With `--step` enabled, the interpreter will **pause after each step** and wait for user confirmation to continue:

* **Pausing After Each Step:** We will modify the execution loops to insert a prompt after each step. Specifically:

  * In the **boot sequence** (in `main.rs`), after completing each major boot task, print a message and prompt the user. For example, after loading config:

    ```rust
    if step_mode {
        println!("➡️  Config loaded successfully. Continue (y/n)?");
        wait_for_continue();
    }
    ```

    and after initializing the interpreter:

    ```rust
    if step_mode {
        println!("➡️  Interpreter initialized (global env and stdlib ready). Continue (y/n)?");
        wait_for_continue();
    }
    ```

    (Here `wait_for_continue()` is a helper to read user input, described below.)

  * In the **script execution** phase, the `Interpreter::interpret` method’s loop will be updated. Currently it iterates through program statements and prints a message for each. We will extend this as:

    ```rust
    for (i, statement) in program.statements.iter().enumerate() {
        // ... execute the statement ...
        if let Ok(val) = self.execute_statement(statement, Rc::clone(&self.global_env)).await {
            last_value = val;
            if self.step_mode {
                // Display step info (see next section)
                println!("➡️  Executed line {} successfully.", statement_line);
                // (Also display state changes here)
                prompt_continue = wait_for_continue();
                if !prompt_continue { 
                    // abort execution if user chose 'n'
                    return Err(vec![]); 
                }
            }
        } else if let Err(err) = ... { ... }
    }
    ```

    The exact placement may differ, but essentially after each statement executes and before moving to the next, we invoke the step prompt. We’ll similarly modify `execute_block` (used for loops and function bodies). In `Interpreter::_execute_block`, instead of simply looping with `?` propagation, we will loop and after each inner statement, if `step_mode` is true, pause for user input.

* **Prompt Implementation:** A new utility (perhaps `wait_for_continue()` function in main or a static in interpreter) will handle the prompt:

  ```rust
  fn wait_for_continue() -> bool {
      use std::io::{stdin, stdout, Write};
      loop {
          print!("continue (y/n)? ");
          let _ = stdout().flush();
          let mut answer = String::new();
          if stdin().read_line(&mut answer).is_ok() {
              let ans = answer.trim().to_lowercase();
              if ans == "y" {
                  return true;
              } else if ans == "n" {
                  return false;
              } else {
                  println!("Please enter 'y' or 'n'.");
                  continue;
              }
          }
      }
  }
  ```

  This will block execution after each step until the user responds. If the user enters 'y', the function returns true (to continue execution). If 'n', it returns false, signaling the interpreter to pause indefinitely or abort. In our design, entering 'n' will **stop the program execution** at that point. (The interpreter will break out of its loop and return early, effectively halting the script.)

* **Abort Handling:** If the user chooses not to continue (`'n'`), we will gracefully terminate the execution:

  * In the interpreter’s loop, detect the false return and break out. We can propagate a special error or simply break. For simplicity, we might treat it as a normal termination (perhaps returning an `Ok(last_value)` or a distinct `Err` that we handle). Another approach is calling `process::exit(0)` to immediately quit, but better is to return from `interpret()` so that main can handle it.
  * We can, for instance, return a `Err(vec![])` with an empty error list or a custom `RuntimeError` like `"Execution aborted by user"`. Then in `main.rs`, detect this case and avoid printing it as a runtime error. For example, if the error list is empty or contains an “aborted” message, simply exit without the usual error logging. This ensures stopping via step mode doesn’t look like a crash.

In summary, the execution flow in step mode will be: perform step -> display info -> prompt -> either continue or stop. This cycle repeats for each step.

## Information Display After Each Step

After each step, the interpreter will output details about what just happened, to help the user understand the program state:

* **Current Script Line or Phase:** We will identify the step either by the script line executed or the boot phase name.

  * For boot steps, we’ll print a short description (as shown above: e.g., "Config loaded", "Interpreter initialized").
  * For script execution steps, ideally we print the actual source line or a summary. We can use the AST node’s stored line number to reference the source. For example, `Statement` nodes often carry a `line` field (for error reporting). We can map this back to the original source line. Since `main.rs` already read the file into `input`, we could pass the source lines to the interpreter (e.g., store `Vec<String>` of code lines in the interpreter when initializing). Then we can print something like:

    ```text
    ➡️  Executed line 42: x = 5
    ```

    showing the actual code that ran. If retrieving the exact code is complex, we can at least indicate the statement type or a simplified form. For instance, using the AST:

    * VariableDeclaration/Assignment: show the variable name and new value.
    * Function call or expression: we could show an expression result if applicable.
    * Control structures: indicate the branch taken (e.g., “if condition was true, executing then-branch”).

    Using the `stmt_type()` helper (available in debug builds) or similar logic, we can get a descriptive label for the statement. This can be part of the output for clarity.

* **Changes to Variables/State:** This is crucial for debugging. After executing a statement, we will display any changes in program state, such as variable values:

  * If the step was a **variable declaration** or **assignment**, show the variable and its new value. For example, if the script line was `x = 5`, after execution we output something like: *“Variable `x` is now 5.”* We can obtain this easily in the interpreter:

    * In the `Statement::VariableDeclaration` branch, after evaluating and defining the variable, we know `name` and the evaluated `value`. We can print `name` and `value`.
    * Likewise for `Statement::Assignment` after `env.assign(name, value)`.
  * If a step calls a **function** or defines an action, we can mention that. E.g., after an `ActionDefinition` (function definition) we might output: *“Defined new function `foo()`.”*
  * For I/O operations like `open file` or HTTP requests, if they assign a result to a variable (they do, e.g., `HttpGetStatement` stores response text into `variable_name`), we will similarly show that variable’s value or at least confirm the action (e.g., *“HTTP GET complete, response stored in `responseText`.”*).
  * In loops (e.g., a count loop or for-each), the loop introduces a loop variable each iteration. We can show its value per iteration. For instance, a count loop uses an internal `count` variable. As we step each iteration, we could print *“(Loop) count = X”* to indicate the loop index in that step.
  * We can generalize by **diffing the environment** before and after each step:
    *Capture the set of variables and their values in the current scope before executing the statement, and again after.* Any differences (new variables, changed values) are then reported. This approach covers all cases uniformly. The `Environment` stores variables in a HashMap which we can snapshot. However, implementing a full diff may be overkill given we know specific statement types that change state. A pragmatic solution is to instrument those specific cases as described, which covers most state changes.
  * Also consider changes like a function return value. If the step is a function return, or an expression statement, we might show the resulting value (if not already printed via `display`). For example, after an expression statement that evaluates to a value (unused), we can print something like *“(Result of expression: 42)”* for transparency.

* **Call Stack / Scope Context:** When stepping inside functions, it’s useful to know the context. We will optionally display the current call stack depth or function name:

  * The interpreter maintains a `call_stack` of `CallFrame` objects for function calls. We can retrieve this via `interpreter.get_call_stack():contentReference[oaicite:13]{index=13}`. For instance, if we're currently inside a function `foo` called from `main`, the stack might have `["main", "foo"]`. We can output this hierarchy. A simple format would be:

    ```
    Call stack: main() → foo()
    ```

    or just *“\[In function foo, called from main]”* to give context. We will include this when relevant (i.e. when depth > 0).
  * Additionally, if we want to be very detailed, we could show local variables in the current function scope. The `CallFrame.locals` (which is filled on error) might not be populated during normal execution. Instead, we can directly inspect the current environment (for the function) since we have it in the interpreter when stepping. For example, we could print the function’s parameters or important local vars at that point. This is an enhancement for deeper debugging if needed.

* **Formatting:** We will ensure the step output is clear and easy to read. For example:

  ```
  ➡️  Executed line 10: x = 5
     Updated variable x -> 5
  ```

  ```
  ➡️  Executed line 11: call foo(5)
     Call stack: main() → foo()
     (in foo) Updated variable y -> 25
  ```

  and so on. Each step’s info will be grouped together, and the `continue (y/n)?` prompt follows immediately after these details.

By providing the line executed, the effect on state, and the context, the user can understand each step’s impact before moving on.

## Implementation Notes (Integration and Edge Cases)

* **Non-Disruptive Integration:** All new step-by-step functionality will be conditional on the `step_mode` flag. We will add `if self.step_mode { ... }` or `if step_mode { ... }` checks around all step-specific logic. This ensures that normal runs (without `--step`) behave exactly as before (no extra prompts or prints, just the existing log messages). The interpreter’s performance in normal mode remains unaffected aside from trivial boolean checks.
* **Interpreter Struct Changes:** Add a field `step_mode: bool` to `Interpreter` (and initialize it to false in `Interpreter::new()` and `with_timeout`). Provide a method or make it public for `main` to set after creation (or pass via constructor). Since `Interpreter` is created in `main.rs`, we can do:

  ```rust
  let mut interpreter = Interpreter::with_timeout(config.timeout_seconds);
  interpreter.step_mode = step_mode;
  ```

  (If we prefer encapsulation, use a setter like `interpreter.enable_step_mode()` internally doing the same.)
* **User Input Handling:** Reading from stdin in the middle of our normally async interpreter might block the async runtime. However, given that our interpreter mostly runs single-threaded and we’re waiting on user input deliberately, this is acceptable. We should flush stdout before reading to ensure the prompt is visible. Since `main` is `async` (Tokio), an alternative is to use an async read, but a simple blocking read is sufficient here because we *want* to pause everything.
* **Exiting vs Pausing:** We interpret 'n' as a decision to abort the execution entirely. In this implementation, once the user enters 'n', the script will stop (and the process will exit shortly after). We will document this behavior to the user. If a true “pause and possibly resume later” functionality is desired, it would require the interpreter to wait without terminating. Our simpler model is binary: continue or quit.
* **Boot Sequence Granularity:** We decided to pause after major boot steps (config load, interpreter init). We are **not** pausing after every compile-phase action (parsing, type-checking) by default to avoid overly verbose interaction before execution. However, we do output some info about them (e.g., number of statements, confirmation that analysis passed) as already present in `main.rs`. Those messages will still appear, but without a prompt. If needed, we could easily add a step prompt after “Semantic analysis passed” as well, by wrapping it in `if step_mode` and asking the user before moving on to execution.
* **Main Function Call:** If the script defines a `main` function, recall that after executing all top-level statements, the interpreter explicitly calls `main`. In step mode, by the time we reach this, we have likely been pausing after each top-level statement already. We may insert one more prompt like *“Top-level script execution complete. Ready to call `main()`.”* before entering the main function. This can be done just before `self.call_function(&main_func)` in `interpret()`. Once inside `main`, the step mode will handle its statements via the same mechanism in `execute_block`. This extra pause gives the user a clear delineation between global code and entering the main function.
* **Ensure Correct Output Order:** Since we are mixing prints and input reading, we should be careful to flush outputs. Using `println!` (which flushes on newline) for messages is fine. For the prompt, using `print!` then `flush()` ensures the question appears before reading input.
* **Testing Considerations:** The step mode should be tested interactively, but we should also ensure it doesn’t break non-interactive use. For example, running the interpreter with `--step` in an environment without a TTY (or in an automated test) will block waiting for input. We might want to detect if stdin is not a TTY and disable step mode in that case or warn the user. This is a corner-case consideration for CI testing; for now, we assume step mode is used in interactive debugging scenarios.

## Testing and PR Submission

**Testing the Feature:** We will perform both manual and (if possible) automated tests:

* **Manual Testing:** Run a simple WFL script with the `--step` flag. For example, a script:

  ```
  x = 5
  display x
  if x > 0
      x = x + 1
  display x
  ```

  Launch with `wfl --step script.wfl`. Verify the interpreter pauses after each line:

  1. After `x = 5`, it should print something like `Executed line 1: x = 5` and `x is now 5`, then prompt. Enter 'y' to continue.
  2. After `display x`, it should show the output of display (which is `5`) and then pause with `Executed line 2: display x` and perhaps note no state change.
  3. On the `if` statement, after evaluating it, pause either at the beginning of the then-block or after executing the then-block’s first line. We expect a message indicating the if condition was true and we entered the branch. After executing `x = x + 1` inside, it should show `x is now 6`. Step through and ensure the logic is correct.
  4. Test saying 'n' at various points to ensure the execution stops. If 'n' is entered, the program should exit without further output or only a graceful message.
  5. Also test with a function call (e.g., a script that defines and calls a function) to see that stepping continues inside the function and that the call stack info is shown.
  6. Try a loop to ensure it pauses each iteration.
* **Automated Testing:** Write a small integration test (if possible) that spawns the interpreter in step mode with known input and feeds a sequence of 'y' and 'n'. This is tricky due to interactive input, but we could simulate stdin using pipes. Alternatively, factor out the step prompt logic so it can be overridden or simulated in tests (e.g., have `Interpreter` accept a callback for continue decision in tests). For initial manual testing might suffice given this is a debugging feature.
* **Regression Testing:** Ensure that running without `--step` is unchanged. All existing unit tests (which likely run the interpreter normally) should pass with no behavior differences. Pay attention to any tests that capture stdout; our new prints (guarded by `step_mode`) should not appear in normal mode. We should run the test suite to confirm nothing broke. If needed, adjust tests or add a flag in tests to suppress step prints.

**PR Submission:** Once the feature is implemented and tested, we will prepare a PR.

* Branch name could be **`feature/step-mode-execution`** (following any project naming conventions).
* In the PR description, explain the feature and reference the issue or request (if an issue ID was given, mention it, e.g., "Closes #XYZ if one exists). Summarize the changes: new `--step` flag, interactive stepping functionality, etc.
* Include examples of usage in the PR description (perhaps the same as tested manually) to demonstrate the new mode.
* Ensure all new code is formatted and linted according to the project guidelines, and that documentation (README or help text) is updated for the new flag.

After submitting the PR, the code will be reviewed. We should be prepared to make iterative improvements, for example, fine-tuning what information is displayed or how the prompt behaves based on feedback. Once approved, the feature will be merged, giving users a powerful new way to debug and walk through their WFL programs step by step.
