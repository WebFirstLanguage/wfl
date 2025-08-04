# WebFirst Language (WFL) Specification

## Introduction  
The **WebFirst Language (WFL)** is a high-level programming language for web development that uses natural English-like syntax to make coding intuitive and accessible. WFL’s programs read like plain English, bridging the gap between how people describe tasks and how code is written. By leveraging full words and phrases instead of cryptic symbols, WFL enables developers of all experience levels to create clear, readable, and maintainable web applications. This document defines WFL’s syntax and core semantics – from variables and control flow to functions, asynchronous operations, I/O, and error handling – and explains the guiding philosophy behind the language’s design.

## Design Philosophy  
WFL is guided by principles that emphasize clarity, simplicity, and safety in coding. Key design tenets include:

- **Natural-Language Syntax:** Code is written in descriptive phrases that mirror natural English. Most operations use keywords that form readable sentences (e.g. `if user is active then display "Online"`), lowering the learning curve for beginners. The language favors words over symbols so that programs can be read almost like prose.

- **Minimal Special Characters:** WFL avoids unnecessary punctuation and operator symbols in favor of words. Common operators have word forms (for example, use **`plus`** instead of `+` for addition). Only a few symbols (like quotes for text, or `+` as an *optional* shorthand) are used, and only when they significantly improve readability. This makes code less intimidating and more approachable.

- **Clarity and Accessibility:** The language prioritizes readability and self-documenting code. WFL syntax is designed to clearly convey intent without terse or cryptic constructs. Features are described in plain language (e.g. **yes/no** instead of boolean literals) to be welcoming for newcomers. The goal is for someone with no programming background to follow what the code does, while still giving experts the tools to be precise and expressive.

- **Strong Type Safety with Inference:** WFL is statically typed for reliability – it enforces strict type checks to prevent errors – but it infers types automatically whenever possible. This means you often don’t need to explicitly annotate types; the compiler deduces them from context (e.g. writing **`store count as 42`** tells WFL `count` is a number). This combination ensures safety (catching type mismatches at compile time) without burdening the programmer with verbose type declarations.

- **Modern and Secure by Design:** WFL incorporates modern programming features in an intuitive way. Asynchronous operations, concurrency, and even pattern matching are expressed in simple, declarative phrases (for example, *“wait for the server response, then show it”*). The language also has built-in security practices (like auto-escaping of HTML output) so that beginners write safe code by default. WFL is designed to integrate seamlessly with web standards (HTML/CSS/JS), making it practical for real-world web projects.

Following these principles, the WFL specification below outlines the formal syntax and semantics of the language in a way that remains understandable to non-experts. Each section provides an EBNF grammar for core constructs alongside plain-language explanations and examples.

## Syntax Overview  
WFL’s syntax is designed to read like written instructions. A WFL program is composed of **statements** written one per line (no semicolons needed). Blocks of code (for example, the body of a loop or an `if` condition) are introduced by a colon (`:`) at the end of a header line and are terminated by a matching `end` clause (e.g. `end loop`, `end if`). Indentation is used in examples for readability, but the `end ...` keywords are the official block terminators. Keywords and identifiers are **case-insensitive** (by convention, code is written in lowercase). Comments can be written with `//`, and everything after `//` on a line is ignored by the compiler.

**Identifiers (Names):** WFL lets you use descriptive names for variables and functions – even multi-word names with spaces. For instance, **`user name`** and **`is active`** are valid variable names. (Internally, you can think of the spaces as part of the name.) Identifiers may include letters, digits, and spaces, but cannot conflict with reserved keywords like `if`, `for`, etc. To avoid ambiguity in parsing, certain keywords (like `as`, `to`, `from`) are used to separate parts of a statement. For example, in **`store is active as yes`**, the word **`as`** signals the end of the variable name and the start of the value. This approach allows names to be very readable. 

**Literals:** WFL supports number literals (e.g. **`10`, `3.14`** or even phrases like **`1 million`** which the compiler interprets as 1000000), text literals written in quotes (e.g. **`"Hello"`**), and the boolean values **yes** and **no** (synonyms for true/false). Special words like **nothing**, **missing**, or **undefined** act as a “null” value for when a variable has no value. Strings can be concatenated by writing them adjacent to the keyword **`with`** or by using the word **`and`** in certain contexts (more on this later), instead of using a `+` operator ([wfl-IO.md](file://file-XU2WRnQ9nsyxEU1hEuxVJX#:~:text=,you%20might%20have%20options%20like)) ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=,allowed%2C%20but%20WFL%20prefers%20words)).

Below, we describe the major syntactic constructs of WFL with formal grammar and examples.

### Variables and Assignment  
**Variable Declaration:** To declare a new variable and give it a value, WFL uses a simple English phrase. The syntax is:

```ebnf
VariableDecl ::= ("store" | "create") <VariableName> "as" <Expression>
```

For example: 

```wfl
store name as "Alice"
store count as 42
create is active as yes
``` 

Each of the above lines introduces a variable and assigns it an initial value. The first line creates a variable **`name`** and stores the text `"Alice"` in it. The second line creates **`count`** with the numeric value 42. The third creates **`is active`** with the boolean value **yes**. In WFL, **`yes`** and **`no`** are natural-language booleans (you can also use **`true`/`false`**, but “yes/no” is encouraged for readability). As shown, variable names can be multiple words long, which helps make their purpose clear ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=Here%20we%20made%20a%20,%E2%80%9Cis%20active%20is%20yes%E2%80%9D)). The keyword **`as`** connects the name and the value in a readable way. (You can read `store is active as yes` as “store ‘is active’ as yes.”) Under the hood, WFL infers each variable’s type from the value: here `count` becomes a number, `name` a text string, etc. – you don’t have to declare types explicitly.

**Type Inference Example:** If you write **`store age as 25`**, WFL knows from the literal **25** that `age` is a number (integer). If you later try to assign a non-number to `age`, the compiler will flag it as a type error. This inference applies to all literals (e.g. quotes make a text, **yes/no** make a boolean).

**Assignment and Updating:** Once a variable exists, you can change its value with an assignment statement. WFL offers intuitive keywords for common updates. The general assignment form is:

```ebnf
AssignmentStmt ::= "change" <VariableName> "to" <Expression>
```

You can simply say **`change X to Y`** to set variable **X**’s value to the result of expression **Y**. For example, **`change count to 100`** would update `count` to 100 (assuming `count` was already defined).

For numeric variables, WFL provides convenient verbs to modify them in-place without needing arithmetic symbols: 

- **`add <amount> to <var>`** increases a numeric variable by the given amount.  
- **`subtract <amount> from <var>`** decreases it by that amount.  
- **`multiply <var> by <factor>`** multiplies it by a factor.  
- **`divide <var> by <divisor>`** divides it by that divisor.

These act as compound assignment operators. For example, if `count` is 42, **`add 8 to count`** will make it 50, and **`subtract 2 from count`** would then make it 48. All these statements read clearly (e.g. “add 5 to count”) and avoid the need for symbols like `+=` or `-=`. Of course, you can also just do a full re-assignment (e.g. `change count to count plus 5`), but the provided phrases are more concise.

WFL allows arithmetic expressions in natural terms. The word **`plus`** can be used in expressions to add, **`minus`** to subtract, etc., if needed inside a larger expression. For instance, you could write **`change count to count plus 5`**. Similarly, comparisons and other operators are written out in English (discussed below in the context of conditions). This continues the philosophy of minimizing symbols in favor of clarity.

### Conditional Statements (If/Else)  
WFL uses an `if`-style construct to express conditional logic, but the syntax is designed to be read as a logical English sentence. The primary form is a **`check if ...`** block, which may include an **`otherwise`** clause for the “else” part. The grammar is:

```ebnf
IfStmtBlock ::= "check if" <Condition> ":" <Block> 
                [ "otherwise if" <Condition> ":" <Block> ]* 
                [ "otherwise:" <Block> ] 
                "end check"
```

In practice, you might write:

```wfl
check if user is active:
    display user name with " is online."
otherwise:
    display user name with " is offline."
end check
```

This example checks a condition and executes one of two blocks. It reads: “check if the user is active – if so, display ‘[name] is online.’ otherwise, display ‘[name] is offline.’” The keyword **`display`** is used here to output text (similar to a print statement; more on I/O later), and **`with`** is used to concatenate strings (`user name` is the value of the variable or field `name` of `user`, joined with the literal string). The condition **`user is active`** illustrates a boolean check written in plain English – it could be a variable that is already a yes/no value. Conditions can use comparators like **`is equal to`**, **`is greater than`**, **`is less than or equal to`**, or even phrases like **`is at least`** for inclusive comparisons, and **`is not ...`** for negation, etc. (e.g. **`if age is not over 18`**). All logical operators are words: use **`and`** and **`or`** to combine multiple conditions (e.g. *“check if X is true **and** Y is false:”*). These operators short-circuit in the typical way (the second condition is evaluated only if needed).

You can chain multiple conditions by using **`otherwise if`** for else-if cases. For example:

```wfl
check if temperature is below 0:
    display "Freezing cold"
otherwise if temperature is below 20:
    display "Cold"
otherwise if temperature is below 30:
    display "Warm"
otherwise:
    display "Hot"
end check
```

Each `otherwise if` introduces an additional condition to check if the previous ones were false. Finally, a plain **`otherwise`** handles the “none of the above” case. This structure avoids deeply nested ifs and reads top-down. It corresponds directly to the traditional if/else-if/else logic ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=,explanatory)).

For simple one-off conditions, WFL also allows a single-line **`if ... then ... otherwise ...`** syntax. For example: 

```wfl
if list is empty then display "No items" otherwise display length of list
``` 

This would output `"No items"` if `list` is empty, or otherwise output the number of items. The single-line form is handy for brief conditions, but for anything more than a trivial action, the block form with `check if / end check` is clearer. Both forms are semantically equivalent.

**Conditions and Comparisons:** As noted, WFL conditions use natural language comparisons. Some examples of valid conditions: 

- **`score is at least passing score`** – true if `score >= passing score`.  
- **`name is "Alice"`** – true if the text in `name` equals `"Alice"`.  
- **`user age is greater than 18`** – true if the `age` field of `user` > 18.  
- **`email contains "@"`** – true if the text in `email` includes an "@" substring.  
- **`file exists`** – true if a file resource is present (in context of I/O).  

WFL tries to make these reads like English. There is no need for parentheses around conditions in WFL; precedence of `and` vs `or` follows logical convention (and binds tighter than or), but if you need explicit grouping you can break an if into nested `check if` or use clarifying variables.

### Loops  
Loops in WFL allow you to repeat actions with language that sounds like giving instructions. The language provides a few types of loops to cover common patterns: **counting loops** for numeric ranges, **for-each loops** for collections, and **conditional loops** that repeat until a condition changes. All loops use a clear English structure with an explicit end.

#### Counting Loops (Numeric Ranges)  
A **count loop** iterates over a range of numbers. The syntax is:

```ebnf
CountLoop ::= "count from" <Expr> ( "to" <Expr> [ "by" <Expr> ] 
              | "down to" <Expr> ) ":" <Block> "end count"
```

This means you can **count from** a start number **to** an end number, optionally specifying a step. For example:

```wfl
count from 1 to 5:
    display "Hello!"
end count
```

This loop will execute 5 times, printing “Hello!” each time ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=,end%20count)). It effectively counts 1, 2, 3, 4, 5. By default, the loop increments by 1. WFL automatically provides a loop variable during a count loop – by default named **`count`** – which holds the current number. In the above loop, `count` will take the values 1, 2, 3, 4, 5 on each iteration. You can use it inside the loop if needed:

```wfl
count from 1 to 5:
    display "This is iteration number " with count
end count
```

This would display lines “This is iteration number 1”, “This is iteration number 2”, and so on ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=You%20can%20use%20the%20current,For%20example)).

If you want a different step increment, you can add **`by <step>`**. For example, **`count from 0 to 10 by 2`** would iterate with `count` = 0, 2, 4, …, 10 (step by 2) ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=Specifying%20Step%20Increments)). If you need to count downward, use **`down to`** instead of `to`. For example, **`count from 5 down to 1`** will loop with `count` = 5,4,3,2,1 ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=,display%20count%20end%20count)). These loops make it easy to express “do X for each number in this range” or “do X N times” without manual index management.

#### For-Each Loops (Collections)  
A **for-each loop** iterates over each element of a collection (like each item in a list or set). The syntax is:

```ebnf
ForEachLoop ::= "for each" <ElementName> "in" <CollectionExpr> [ "reversed" ] ":" <Block> "end for"
```

For example, given a list called `shopping`:

```wfl
for each item in shopping:
    display item
end for
```

This will loop through the `shopping` list (which might contain items like `"milk"`, `"bread"`, `"eggs"`) and execute the block for each element ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=WFL%20provides%20a%20%2A%2Afor,style%20that%20reads%20nicely)). On each iteration, the variable **`item`** will hold the current element’s value. The above code would display each item name in turn. The loop automatically stops after the last element. The loop variable (`item` in this case) is scoped to the loop. You can choose any descriptive name (if you were looping through users, you might say **`for each user in users:`**). If the collection is empty, the loop body runs zero times.

By default, a for-each loop goes in the collection’s natural order (first to last). If you want to iterate in reverse order (for an ordered collection like a list), you can add the keyword **`reversed`** after the collection expression: **`for each item in shopping reversed:`** ... `end for` would go through the list from last element to first ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=for%20each%20item%20in%20shopping,display%20item%20end%20for)).

#### Conditional Loops (While/Until)  
A **conditional loop** repeats as long as or until a condition holds, similar to “while” loops in other languages, but using plain language. WFL offers two forms:

```ebnf
WhileLoop  ::= "repeat while" <Condition> ":" <Block> "end repeat"
UntilLoop  ::= "repeat until" <Condition> ":" <Block> "end repeat"
```

Use **`repeat while`** when you want to keep looping *as long as* a condition is true, and **`repeat until`** to loop *until* a condition becomes true (i.e. loop while the condition is false). For example:

```wfl
repeat while user is not logged in:
    prompt login
end repeat
```

This will execute the `prompt login` action repeatedly **while** the condition `user is not logged in` remains true (meaning the loop stops once the user is logged in) ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=,and%20stop%20once%20it%E2%80%99s%20true)). Likewise:

```wfl
repeat until temperature is above 100:
    increase heater
end repeat
``` 

will keep increasing the heater **until** the temperature exceeds 100 ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=repeat%20until%20temperature%20is%20above,increase%20heater%20end%20repeat)). Both of these mean the same thing, just phrased differently; WFL lets you choose the phrasing that reads best for your scenario.

For an endless loop, you can literally write **`repeat forever:`** ... **`end repeat`**, which will loop indefinitely until you break out of it ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=repeat%20forever%3A%20check%20system%20status,end%20repeat)). (Use this with caution, and typically with a break condition inside.)

**Loop Control:** Inside any loop, you can control the flow with natural commands. WFL provides a way to **break out** of a loop early or **skip** to the next iteration. For example, you might use **`break`** to exit a loop if a certain condition is met in the middle, or **`continue`** (or **`skip`**) to jump to the next iteration without finishing the rest of the loop body. These work like in other languages but are expressed as words. A concrete example:

```wfl
for each number in numbers:
    check if number is negative:
        skip   // skip negative numbers, go to next iteration
    end check
    display "Processing " with number
    check if number is 0:
        break  // stop the loop entirely if number is 0
    end check
end for
```

In this pseudo-code, the loop skips any negative number and stops completely if it encounters 0. (The keywords `skip` and `break` would be used as shown – WFL might allow synonyms like `continue` or `exit loop` as well, but the idea is the same.)

### Functions (Actions)  
In WFL, reusable blocks of code are defined as **actions** (analogous to functions in other languages). Defining an action allows you to encapsulate a sequence of steps under a name and call it whenever needed, promoting code re-use and clarity. The term *“action”* emphasizes that it performs some operation, and the syntax for actions is designed to look like defining a procedure in plain language.

**Action Definition:** The syntax to define a function/action is:

```ebnf
ActionDef ::= "define" ["async"] "action" <ActionName> ":" 
              [ "needs:" ( <ParamName> "as" <Type> )+ ] 
              [ "gives back:" ( <ReturnName> "as" <Type> ) ] 
              [ "do:" ] 
              <Block> 
              "end action"
```

- **`define action <name>:`** starts a function definition. The action’s name can be multiple words (e.g., “send email” or “add user record”) – choose a name that describes what it does. If the action will be asynchronous (more on async in the next section), include the keyword **`async`** right after `define` (i.e., **`define async action ...`**).

- **`needs:`** (optional) lists the parameters (inputs) the action expects, each with a name and type. If the action requires arguments, list each on a new line under a indented **needs:** section, using the format `<paramName> as <TypeName>`. For example, `needs: user id as text` means this action takes a parameter called **`user id`** of type **text** (string). If no parameters are needed, you can omit the `needs:` section entirely.

- **`gives back:`** (optional) specifies the action’s return value, if any. You provide a name and type for the result. For instance, `gives back: profile data as text` indicates the function will return a text value named **`profile data`**. If the action doesn’t return anything (i.e. it’s like a void function), you omit this section.

- **`do:`** (optional) indicates the beginning of the action’s body. In cases where you have a `needs:` or `gives back:` section, it’s common to put a `do:` before the actual code to clearly separate the header from the body. (If there are no parameters or return specified, you can actually start the body immediately after the colon on the definition line, and a `do:` is not required.) In practice, using `do:` is a stylistic choice to improve readability when an action has a long header.

- **Action body:** The indented block of code following the definition (or the `do:`) is the sequence of steps the action will perform when called. Within this body, you write statements just like in the global scope (you can declare local variables, use loops, conditionals, etc.). To return a value, use the **`give back`** statement inside the body.

- **`end action`** terminates the function definition.

**Example – Simple Action:** Here’s a basic example of defining and using an action with no parameters:

```wfl
define action say hello:
    display "Hello, world!"
end action

// ... later in the code ...
perform say hello
perform say hello
```

This defines an action named **`say hello`**. Whenever we **`perform say hello`**, the program will execute the action’s body and display "Hello, world!" ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=,end%20action)) ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=)). We called it twice in the example, so it will print the greeting twice. Notice that calling (invoking) a function is done with the keyword **`perform`** followed by the action name. (WFL may allow omitting `perform` and just using the action’s name as a command in some cases, but using **`perform`** is unambiguous and recommended ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=We%E2%80%99ve%20already%20seen%20usage%20of,perform)).)

**Example – Action with Parameters and Return:** Here’s a more involved example:

```wfl
define action add numbers:
    needs:
        x as number
        y as number
    gives back:
        sum as number
    do:
        give back x plus y
end action
```

This defines an action **`add numbers`** that takes two numeric inputs, `x` and `y`, and returns their sum ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=define%20action%20add%20numbers%3A%20needs%3A,x%20plus%20y%20end%20action)). Inside the body we use **`give back`** to return the result of the expression `x plus y`. (The word **plus** here adds the two numbers; we could also use `x + y`, but writing it out keeps with WFL’s style ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=,allowed%2C%20but%20WFL%20prefers%20words)).) The return value is labeled as **`sum`** in the definition (type number).

To call this action and use its returned value, we can do:

```wfl
store result as perform add numbers with x as 5 and y as 7
display result   // displays 12
```

Here we **perform** the action `add numbers`, specifying the arguments: **`with x as 5 and y as 7`** passes 5 for `x` and 7 for `y` ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=Using%20this%20action%3A%20,would%20display%2012)). The call `perform add numbers ...` produces a number (the sum, 12), which we capture into a variable `result` using **`store result as ...`**. We then display the result. Note that the name `sum` given in the action definition is just to describe the output in that context; when we capture the returned value, we can store it in any variable (here we chose `result`). 

The syntax for calling an action generally is: **`perform <ActionName> [with <param1 name> as <value1> [and <param2 name> as <value2> ...] ]`**. You list the parameters by name after `with`, each followed by **`as <value>`**, joining multiple parameters with **`and`**. This explicit naming makes calls easy to read (you can see which argument is which). If an action has no parameters, you just write `perform ActionName` with nothing after it.

If an action returns a value, you can use that in an expression or store it as shown. If it doesn’t return anything (procedures), you just perform it on its own line.

**Example – Action with Text Return:** For a slightly different illustration, consider:

```wfl
define action make greeting:
    needs:
        name as text
    gives back:
        message as text
    do:
        give back join "Hello, " and name and "!"
end action

store greet_msg as perform make greeting with name as "Alice"
display greet_msg   // displays "Hello, Alice!"
```

This action takes a text `name` and returns a greeting message that incorporates that name ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=Another%20example%3A%20An%20action%20could,and%20name%20and)) ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=This%20returns%20a%20text,)). We demonstrate building the string using **`join ... and ...`** to concatenate pieces (equivalent to `"Hello, " + name + "!"` but written in words). After defining it, we call **`perform make greeting with name as "Alice"`**, store the returned text in `greet_msg`, and then display it. The output is *Hello, Alice!*.

Under the hood, each action has a specific type signature (e.g., `add numbers` is a function taking two numbers and returning a number), but WFL handles matching up calls with definitions for you. There is no need to manually write type signatures in most cases since the syntax already includes the type of each parameter and return.

### Asynchronous Operations  
One of WFL’s strengths is expressing asynchronous operations (like waiting for web requests or performing tasks in parallel) in a straightforward way. In many languages, async code introduces complexity with callbacks or special syntax, but WFL treats it as just another scenario to describe in English.

**Async Actions:** To define a function that can be awaited (i.e. perform work asynchronously), declare it as **`async`**. For example:

```wfl
define async action read files:
    needs:
        file paths as list of text
    gives back:
        contents as list of text
    do:
        ...
end action
```

By marking this action with **`async`**, you signal that it may perform operations concurrently or wait for external events. Inside an async action, you can use the **`await`** keyword to pause until a result is ready (similar to `await` in other languages). For instance, if `read files` action opens several files concurrently, it might `await` each file’s read completion.

In general, any action can be made async if it needs to perform asynchronous tasks (such as network I/O). Calling an async action from within another async context allows concurrency.

**Awaiting Results:** WFL introduces the keyword **`await`** to wait for an asynchronous operation to finish and get its result. You can **await** an async action or certain I/O operations (like an HTTP request). For example, suppose `fetch data` is an async action that starts downloading from a URL. You could use:

```wfl
store response as await perform fetch data with url as "https://api.example.com/info"
display response
``` 

This will start the fetch and pause execution of the current async action until the `fetch data` action completes, then store the result in `response` and proceed to display it. The syntax looks almost identical to a synchronous call; the only difference is adding **`await`** to indicate we’re waiting asynchronously ([wfl-IO.md](file://file-XU2WRnQ9nsyxEU1hEuxVJX#:~:text=vars.md%5D%28file%3A%2F%2Ffile,or%20using)) ([wfl-IO.md](file://file-XU2WRnQ9nsyxEU1hEuxVJX#:~:text=,read%20content%20from%20reportFile)).

If you omit `await` when calling an async operation inside an async context, WFL will start the operation and immediately continue with the next statement, allowing you to do other work in parallel and `await` the result later. For instance:

```wfl
// In an async action:
store req1 as perform fetch data with url as "https://api.example.com/1"
store req2 as perform fetch data with url as "https://api.example.com/2"
// Both requests started concurrently without waiting.

... do other work here ...

store data1 as await req1
store data2 as await req2
display "Got results: " with length of data1 with " and " with length of data2
```

In this snippet, `req1` and `req2` act like promises or tasks for the data fetches. We launched them without waiting, did other work (if any), then later used **`await req1`** and **`await req2`** to get their results ([wfl-IO.md](file://file-XU2WRnQ9nsyxEU1hEuxVJX#:~:text=,https%3A%2F%2Fapi.example.com%2Fdata2)) ([wfl-IO.md](file://file-XU2WRnQ9nsyxEU1hEuxVJX#:~:text=%2F%2F%20Now%20await%20both%20results,store%20result2%20as%20await%20request2)). This approach makes it easy to express parallel operations: it feels like stating “do this, do that, then when both are done, continue.” There is no explicit thread or callback management exposed; the language runtime handles the details.

**Async I/O:** Many I/O operations (discussed below) can be awaited. For example, reading from a file or making a network request can be asynchronous. WFL’s grammar doesn’t change for async versus sync; you simply add `await` when you need to pause for the result ([wfl-IO.md](file://file-XU2WRnQ9nsyxEU1hEuxVJX#:~:text=vars.md%5D%28file%3A%2F%2Ffile,or%20using)). If an operation is used in an async context without `await`, it means it’s initiated asynchronously. If used in a normal (synchronous) context, it will block until completion (or you must handle it via callbacks or events, which WFL tries to avoid). In practice, you will typically mark entire actions as `async` if they involve awaiting multiple steps.

To summarize, WFL’s async support allows you to write asynchronous code that looks very similar to synchronous logic. The explicit `async` in definitions and `await` in usage make the timing clear without introducing complex syntax. This aligns with WFL’s goal of making advanced features approachable.

### Containers and Object-Oriented Programming
WFL supports object-oriented programming through **containers**, which are similar to classes in other languages. Containers allow you to define reusable templates for objects that encapsulate data (properties) and behavior (methods). The syntax follows WFL's natural language philosophy, making object-oriented concepts accessible through English-like constructs.

**Container Definition:** The syntax to define a container is:

```ebnf
ContainerDefinition ::= "create" "container" Identifier [ "extends" Identifier ] 
                       [ "implements" IdentifierList ] ":" 
                       PropertyDefinition* MethodDefinition* EventDefinition*
                       "end"
PropertyDefinition ::= "property" Identifier ":" Type [ "default" Expression ]
MethodDefinition ::= "action" Identifier [ "needs" ParameterList ] [ ":" Type ] ":" 
                    StatementList "end"
EventDefinition ::= "event" Identifier [ "needs" ParameterList ]
```

For example:
```wfl
create container Person:
    property name: Text
    property age: Number default 0
    
    action greet:
        display "Hello, I am " + this.name + " and I am " + this.age + "."
    end
    
    action set age needs new age: Number:
        change this.age to new age
    end
end
```

This defines a `Person` container with two properties (`name` and `age`) and two methods (`greet` and `set age`). The `default` keyword allows specifying initial values for properties. Inside methods, `this` refers to the current instance of the container.

**Container Inheritance:** Containers can extend other containers to inherit their properties and methods:

```wfl
create container Student extends Person:
    property school: Text
    
    action greet:
        display "Hello, I am " + this.name + ", a student at " + this.school + "."
    end
end
```

The `Student` container inherits `name` and `age` from `Person` but overrides the `greet` method with its own implementation.

**Interface Implementation:** Containers can implement interfaces to ensure they provide specific methods:

```wfl
create interface Drawable:
    action draw:
end

create container Circle implements Drawable:
    property radius: Number
    
    action draw:
        display "Drawing a circle with radius " + this.radius
    end
end
```

**Container Instantiation:** To create instances of containers, use the `create new` syntax:

```ebnf
ContainerInstantiation ::= "create" "new" Identifier "as" Identifier ":" 
                          PropertyInitializer* "end"
PropertyInitializer ::= Identifier "=" Expression
```

For example:
```wfl
create new Person as alice:
    name = "Alice"
    age = 28
end

alice.greet()
display alice.name
```

This creates a new `Person` instance named `alice`, initializes its properties, then calls its `greet` method and accesses its `name` property.

**Static Members:** Containers can have static properties and methods that belong to the container itself rather than instances:

```wfl
create container Math:
    static property PI: Number = 3.14159
    
    static action square needs value: Number: Number
        return value * value
    end
end

display Math.PI
store result as Math.square(5)
```

Static members are accessed using the container name directly, without creating an instance.


### Input/Output (File, Network, Database)  
WFL treats input/output operations (file reading/writing, HTTP requests, database queries, etc.) as high-level actions described in English. All I/O shares a unified style: you **open** a resource, perform reads/writes, and **close** it, with similar syntax for files, web URLs, and databases ([wfl-IO.md](file://file-XU2WRnQ9nsyxEU1hEuxVJX#:~:text=WebFirst%20Language%20,like%20way.%20Key%20goals%20include)) ([wfl-IO.md](file://file-XU2WRnQ9nsyxEU1hEuxVJX#:~:text=,across%20files%2C%20network%2C%20and%20databases)). This consistency means once you learn how to do one kind of I/O, the others feel familiar.

**Opening Resources:** To work with an external resource, you first **open** it, which creates a connection or handle (represented as a resource object). The syntax is:

```ebnf
OpenStmt ::= "open file at" <TextLiteral> [ "for" <Mode> ] "as" <ResourceName>
          | "open url at" <TextLiteral> [ "with" <RequestOptions> ] "as" <ResourceName>
          | "open database at" <TextLiteral> "as" <ResourceName>
```

Examples:

- **`open file at "data/input.txt" as inputFile`** – Opens a file from the given path. By default, if not specified, this might open for reading (and maybe writing if allowed). You can explicitly add a mode: e.g. **`open file at "data/output.txt" for writing as outputFile`** ([wfl-IO.md](file://file-XU2WRnQ9nsyxEU1hEuxVJX#:~:text=This%20opens%20the%20file%20at,mode%20explicitly%20in%20natural%20terms)). Modes are phrased in words like *for reading, for writing, for appending*. If the file doesn’t exist and you open for reading, WFL will throw a *file not found* error that you can catch. If you open for writing, it may create the file if missing.

- **`open url at "https://api.example.com/data" as apiResponse`** – Initiates an HTTP GET request to that URL and creates a handle `apiResponse` representing the connection/response ([wfl-IO.md](file://file-XU2WRnQ9nsyxEU1hEuxVJX#:~:text=%2AExamples%3A%2A%20%60open%20file%20at%20,%E2%80%93%20Closes%20any%20open%20resource)). By default this is a GET; if you need a POST or other method, you can specify options. For instance: 
  ```wfl
  open url at "https://api.example.com/data" as apiResponse with:
      method as POST
      body as "{\"query\": \"status\"}"
      header "Content-Type" as "application/json"
  end open
  ``` 
  This shows an extended form where after the **with:**, you can provide request details like HTTP method, request body, or headers in a natural way ([wfl-IO.md](file://file-XU2WRnQ9nsyxEU1hEuxVJX#:~:text=open%20url%20at%20,Type%22%20as%20%22application%2Fjson)). (Here we used a block with `end open` to list multiple options; alternately, a simpler request might be given in one line.) In most cases, a simple GET needs no options – just `open url at "..." as response`.

- **`open database at "sqlite3://mydb.sqlite" as myDB`** – Connects to a database (the connection string or URL specifies the DB location/protocol) and gives you a handle `myDB`. WFL abstracts database connections similarly to files and URLs.

When you open a resource successfully, you get a resource variable (like `inputFile`, `apiResponse`, `myDB`) that you will use for subsequent operations. If an open fails (file not found, network error, etc.), it triggers an error that can be handled with WFL’s error handling (try/when) described later.

**Reading from Resources:** Once a resource is open, you can **read** from it. The general form is:

```ebnf
ReadStmt ::= ["store" <VarName> "as"] "read" <ContentType> "from" <ResourceName> [ "as" <VarName> ]
```

This looks a bit complex, but it allows flexibility in phrasing. Essentially, you either do `store X as read ... from Y` or `read ... from Y as X` – both achieve “read from Y into variable X.” The `<ContentType>` part is a word that clarifies what you’re reading, depending on the resource:

- For files, you typically use **`read content from <fileHandle>`** to read the entire content (or the next chunk of content) from a file ([wfl-IO.md](file://file-XU2WRnQ9nsyxEU1hEuxVJX#:~:text=,For%20example)) ([wfl-IO.md](file://file-XU2WRnQ9nsyxEU1hEuxVJX#:~:text=,The%20keyword)).
- For network responses (HTTP), use **`read response from <urlHandle>`** ([wfl-IO.md](file://file-XU2WRnQ9nsyxEU1hEuxVJX#:~:text=match%20at%20L91%20,as%20read%20results%20from%20myDB)).
- For databases, after you send a query, you might use **`read results from <dbHandle>`** to get the result set ([wfl-IO.md](file://file-XU2WRnQ9nsyxEU1hEuxVJX#:~:text=,form%20to%20send%20a%20query)).

You can also specify reading in chunks or lines if needed. For example, in a loop you might do `read line from file as lineData` to get line-by-line reading. In streaming scenarios, phrases like **`read next 100 bytes from file`** or **`read next record from db`** could be used – the exact grammar for these is an extension of the basic read.

**Example – File Read:** 
```wfl
open file at "config.txt" as configFile
store configData as read content from configFile
close configFile
display "Config contents: " with configData
``` 

This opens a text file, reads its whole content into the variable `configData`, closes the file, and then displays the content. Each step is clearly delineated. 

**Writing to Resources:** To write or send data, WFL uses the **`write`** keyword:

```ebnf
WriteStmt ::= "write" <Expression> "to" <ResourceName>
```

For example:
```wfl
open file at "log.txt" for append as logFile
write "Application started\n" to logFile
close logFile
``` 

This opens a log file (creating it if needed, in append mode), writes a line of text to it, then closes it. Similarly, if `apiResponse` is an HTTP connection for a POST, you might **`write requestBody to apiResponse`** to send data in the request ([wfl-IO.md](file://file-XU2WRnQ9nsyxEU1hEuxVJX#:~:text=match%20at%20L99%20,or%20HTTP)). For databases, `write` can send a command, but typically you’ll use the higher-level **`perform query`** syntax (see below).

**Closing Resources:** After finishing with a resource, use **`close <ResourceName>`** to close it and free any associated system resources. E.g., `close inputFile` or `close myDB`. WFL will attempt to warn you if you forget to close something (and will clean up at program end or via a `finally` block), but it’s good practice to close when done, just like saying “we’re done with this file/connection.”

**Performing Queries and Fetches:** WFL provides some composite operations for convenience:

- **`perform fetch from url "<...>"`** is a shorthand to open a URL, get its content, and close it in one go (essentially an HTTP GET). You would typically use it like: `store data as await perform fetch from url "http://example.com"` – which returns the fetched data (text or JSON, etc.) in `data`. This is equivalent to doing an `open url ... as X; store data as read response from X; close X` sequence, but shorter.

- **`perform query "<SQL>" on <dbHandle>`** sends a database query and retrieves results in one step ([wfl-IO.md](file://file-XU2WRnQ9nsyxEU1hEuxVJX#:~:text=,form%20to%20send%20a%20query)). For example: 
  ```wfl
  perform query "INSERT INTO Users VALUES ('Alice')" on myDB
  store resultRows as perform query "SELECT * FROM Users" on myDB
  ``` 
  The first line writes to the database; the second reads data (storing the returned rows in `resultRows`). Under the hood this likely does a `write` (send query) followed by a `read results` on the DB handle.

These high-level commands use natural phrasing (“perform query … on …”) to keep code concise and clear.

**Batch and Parallel I/O:** As mentioned in the async section, you can initiate multiple I/O operations in parallel. For example, to read multiple files at once in an async action, you could:

```wfl
// inside an async action
for each path in filePaths:
    store task{index} as perform read content from file path
end for

// ... later, await all tasks
for each task in tasks:
    store content{index} as await task
end for
```

(This is a conceptual sketch: WFL might allow a more direct syntax for parallel operations on collections, but the idea is you launch all the reads with `perform` and then `await` them.)

**Example – Full I/O scenario (File):**  
To illustrate file I/O, here’s a small example that reads a configuration file or creates a default one if it doesn’t exist (combining I/O with error handling):

```wfl
try:
    open file at "config.txt" as configFile
    store configData as read content from configFile
    close configFile
    display "Loaded config: " with configData
when file not found:
    display "Config file not found. Creating a default config."
    create file at "config.txt" with "defaultSettings=true\n"
    retry
otherwise:
    display "Could not read config: " with error message
end try
```

Let’s unpack this: We attempt to open "config.txt". If that fails because the file doesn’t exist, a `file not found` error is thrown, and we catch it with the **`when file not found:`** clause. In that handler, we display a message and then **`create file at "config.txt" with "defaultSettings=true\n"`**, which is a convenient one-liner to create a new file containing the given text (it opens the file, writes the content, and closes it). Then we use **`retry`**, which jumps back to the start of the `try` block and tries again (now that the file exists, the second attempt to open/read should succeed) ([wfl-IO.md](file://file-XU2WRnQ9nsyxEU1hEuxVJX#:~:text=match%20at%20L244%20%2F%2F%20If,txt%20again)). The `otherwise` clause catches any other error (like we didn’t have permission to open the file, etc.), and prints a generic error message along with the provided **`error message`** (a built-in variable in error handlers that contains the system’s description of what went wrong). After a successful read, we print out the config data.

This example shows how file operations and error handling work together in WFL. Similar patterns can be used for network requests (e.g., retry on a timeout) or database queries (retry on a deadlock, etc.), using appropriate error conditions (like `when network timeout`, `when db locked`, etc.).

### Error Handling  
WFL’s error handling model is built around the idea of writing what should happen, rather than dealing with low-level exceptions. It uses a **try/when/otherwise** construct that mirrors how one might describe contingencies in English. The syntax is:

```ebnf
TryStmt ::= "try:" <Block> 
            { "when" <ErrorCondition> ":" <Block> }
            [ "otherwise:" <Block> ] 
            [ "finally:" <Block> ]
            "end try"
```

- **`try:`** introduces a block of code that may throw errors. You put the normal sequence of operations inside this block.

- **`when <ErrorCondition>:`** introduces a handler for a specific type of error. You can have multiple `when` clauses to handle different errors differently. The `<ErrorCondition>` is written in words, describing the error scenario. For example: **`when file not found:`**, **`when network timeout:`**, **`when http error:`**, **`when invalid data:`**, etc. These conditions are matched against the error that occurred. They read like “when [this happens] do the following.” Inside each when-block, you can write recovery or cleanup code to respond to that error. You can also use a special variable **`error message`** (and possibly others like `error code` depending on context) to get details about the error.

- **`otherwise:`** introduces a catch-all handler that will run if none of the specific `when` conditions matched. This is analogous to a general `catch` or `else` for errors. Use it to handle any unexpected issues in a generic way (like logging or displaying a message).

- **`finally:`** (optional) introduces a block of code that will run after the try (whether an error happened or not). This is useful for cleanup actions that should happen regardless of success or failure – for example, ensuring a file is closed. In WFL, you might not often need `finally` since the language encourages handling resource management in the normal flow (and will auto-close at program end), but it’s there for cases where you must guarantee some cleanup.

- **`end try`** closes the try construct.

**Using try/when:** We saw an example above in the I/O section. Here’s a more generic template:

```wfl
try:
    <normal operations that might fail>
when <ErrorType1>:
    <recover from ErrorType1>
when <ErrorType2>:
    <recover from ErrorType2>
otherwise:
    <handle any other error>
finally:
    <cleanup actions>
end try
```

When the code in the try block runs, if an error occurs, it immediately jumps out to the first matching `when` clause and executes that. Within a `when` or `otherwise` block, you can even fix the issue and use **`retry`** to attempt the try block again from the beginning (as we did above) ([wfl-IO.md](file://file-XU2WRnQ9nsyxEU1hEuxVJX#:~:text=match%20at%20L244%20%2F%2F%20If,txt%20again)). This is a powerful feature for errors that are transient or can be corrected (like creating a missing file, or reattempting a network call after a timeout) ([wfl-IO.md](file://file-XU2WRnQ9nsyxEU1hEuxVJX#:~:text=when%20network%20timeout%3A%20%2F%2F%20If,retry)) ([wfl-IO.md](file://file-XU2WRnQ9nsyxEU1hEuxVJX#:~:text=display%20,retry)). If a `retry` is executed, none of the remaining handlers or finally block run yet – control goes back to the top of the try.

If no error happens in the try, all the `when` and `otherwise` blocks are skipped. The `finally` (if present) will still run at the end.

If an error occurs that doesn’t match any `when` condition, then the `otherwise` block (if provided) will run. If no otherwise is given either, the error will propagate up (to an outer try, or if none, it will cause the program to terminate with an error message).

**Error Conditions:** WFL defines error conditions with descriptive phrases. Some common ones have been anticipated by the language: 
- **`file not found`**, **`file unreadable`**, **`network timeout`**, **`network unreachable`**, **`http error`** (for non-200 HTTP status), **`parse error`**, **`invalid input`**, **`permission denied`**, etc. 

These cover typical failure modes. For example, if an HTTP request returns a 500 status code, WFL might throw an “http error” that you can catch with `when http error:` and maybe check `error message` or an `apiResponse.status` for details ([wfl-IO.md](file://file-XU2WRnQ9nsyxEU1hEuxVJX#:~:text=when%20http%20error%3A%20%2F%2F%20Handle,return%20empty%20JSON%20as%20fallback)) ([wfl-IO.md](file://file-XU2WRnQ9nsyxEU1hEuxVJX#:~:text=%2F%2F%20Handle%20HTTP%20errors%20%28non,return%20empty%20JSON%20as%20fallback)). During database operations, an integrity constraint violation might throw an “invalid data” error, etc.

WFL’s philosophy is to make error messages and conditions human-friendly. The runtime and compiler generate messages like “Expected a number but found text — try converting it first” for type errors, for instance. These messages are meant to guide the developer to a solution, not just report a failure. When you catch errors, you might choose to display `error message` to the user or log it. That message will be in plain language explaining the issue.

**Example – Network call with error handling:**  
```wfl
define action get user profile:
    needs:
        user id as text
    gives back:
        profile data as text
    do:
        try:
            open url at "https://api.example.com/users/" with user id as apiResponse
            store profileJson as read response from apiResponse
            close apiResponse
            give back profileJson
        when network timeout:
            display "Request timed out, retrying..."
            retry
        when http error:
            display "Server returned an error (" with apiResponse.status with ")."
            give back "{}"   // return an empty JSON object on error
        otherwise:
            display "Unexpected network error: " with error message
            give back "{}"
        end try
end action
```

In this (hypothetical) action, we try to fetch a user profile from a web API ([wfl-IO.md](file://file-XU2WRnQ9nsyxEU1hEuxVJX#:~:text=try%3A%20%2F%2F%20Attempt%20to%20fetch,close%20apiResponse%20give%20back%20profileJson)) ([wfl-IO.md](file://file-XU2WRnQ9nsyxEU1hEuxVJX#:~:text=when%20http%20error%3A%20%2F%2F%20Handle,return%20empty%20JSON%20as%20fallback)). We handle two specific errors: a network timeout (maybe the server didn’t respond) by logging and retrying once, and a general HTTP error (like 404 or 500 status) by reporting it and returning a default empty JSON (`"{}"`). The `otherwise` catches anything else (like no internet connection) and also returns a safe default. The normal path returns the `profileJson` on success. This demonstrates how **readable** and declarative error handling is in WFL: we describe conditions (“when timeout, do this; when HTTP error, do that; otherwise do this”) without low-level exception objects. The code that handles errors sits next to the code that might produce them, making it easy to follow the logical outcomes.

WFL’s built-in errors cover many scenarios, but if needed, the language might allow users to throw their own errors (perhaps with a statement like `fail with "<message>"` or by calling an error utility). In such cases, you would still catch them with try/when. The focus is always on the *meaning* of the error rather than an exception class name.

**Error Messages and Logging:** When an error isn’t caught, WFL will output a message in plain language explaining what went wrong. The language’s guiding principle is to make these messages as helpful as possible, often suggesting a fix. For example, a type mismatch might say, *“The value needs to be a number — check your input.”* If running in a browser environment, WFL could show these errors in a console or overlay, formatted for readability, possibly even in multiple languages (the error system is designed to support localization of messages ([wfl-error.md](file://file-M39LPJvbA7VACQ4xFuZ7u3#:~:text=match%20at%20L415%20found%20text,or%20specific%20phrasing%20are%20adapted)) ([wfl-error.md](file://file-M39LPJvbA7VACQ4xFuZ7u3#:~:text=match%20at%20L429%20template%20%28%60,way%2C%20developers%20can%20switch%20the))).

For debugging and logging, WFL likely has a logging facility with various levels (info, warning, error) that also uses natural language. While not a core part of the syntax, this fits the philosophy – e.g., `log info "Starting process"` might be how you write a log entry.

In summary, WFL’s error handling semantics ensure that error cases are handled in a structured yet readable way. Developers can describe how to recover from issues without getting bogged down in exception class hierarchies or obscure codes. This approach turns error handling into a part of the program’s narrative, aligning with WFL’s emphasis on clarity and approachability.

## Type System and Semantics  

WFL is a **statically-typed** language with a strong type system to catch errors early and enforce consistency, but it uses **type inference** to keep the syntax clean. This section describes WFL’s primitive types, how compound data structures are typed, how type inference works, and other semantic aspects like variable scope and memory management.

### Primitive Types  
WFL provides a set of built-in primitive types that cover basic kinds of data:

- **Number:** A numeric type for integers and real numbers. WFL does not distinguish between int and float in syntax; `number` covers any numerical value (the compiler/runtime may internally use appropriate representations). You can write numbers in usual decimal form or using underscores and words for readability (e.g., `1000000`, `1_000_000`, and `1 million` are all valid and represent the same number). Arithmetic operations (`+`, `-`, etc.) produce `number` results. There’s typically no fixed limit on magnitude beyond what the underlying platform supports (likely akin to JavaScript’s Number or bigints if needed).

- **Text:** A sequence of characters (string). Text literals are enclosed in quotes `"..."`. WFL supports Unicode and allows embedding typical escape sequences if necessary (though since the language is high-level, it might handle things like newlines and Unicode characters directly). Strings can be concatenated with the word **`with`** or by using `join/and` as shown earlier, rather than using `+`. The type is referred to as **text** in type declarations (as we saw in function parameters).

- **Boolean (Yes/No):** A truth value, represented by the literals **yes** and **no**. Internally this is the boolean type. You can also use **true** and **false** (WFL accepts those synonyms), but the language defaults to yes/no in examples to keep the English style. Boolean values typically result from comparisons (e.g., `x is greater than 5` yields yes/no) or explicit logical operations. They can be combined with `and`, `or`, and negated with `not`.

- **Nothing (Null):** WFL has a concept of “no value”, often called **nothing**. This is similar to null or undefined in other languages, but WFL gives it a natural name. In contexts where a value might be missing or not applicable, you can use **nothing** (or the synonyms **missing** or **undefined**, which the language treats as the same). `nothing` can be thought of as belonging to any type (or a special bottom type) – you can assign nothing to a text variable to indicate it has no text, for example. However, using nothing where a concrete value is required might trigger an error that should be handled (WFL would say something like “Expected a X but found nothing”). There are often better patterns (like using option types or default values) than heavily relying on nothing, but it’s available and safely handled (e.g., you can check “if value is nothing” in a condition).

- **Date/Time:** (If provided) WFL might include a date/time type for convenience, given web needs. If so, it would allow literals like `"2025-12-31"` or natural expressions (“today”, “tomorrow”) that the compiler understands. We won’t elaborate since it’s not explicitly mentioned in our core spec request, but conceptually such types would be handled with the same natural syntax approach (e.g., `if date is before today`, etc.).

These primitives are the building blocks. Every variable has a type that is one of these or a compound type. WFL does **not** implicitly convert between types in a way that might surprise you; you have to explicitly convert if needed (and WFL provides easy conversion phrases like **`convert X to number`**, **`X as text`**, etc.). For example, you cannot accidentally use a number where text is expected – the compiler will complain, saying something like “Expected text but found a number”.

### Compound Types (Collections and Records)  
Beyond primitives, WFL supports compound types to group multiple values:

- **List of T:** An ordered list (sequence) of elements, all of type T. Think of this like an array or list in other languages. In WFL, you might declare a list and its contents in one go, or build it up incrementally. The type of a list includes the element type, e.g. “list of number” or “list of text”. Lists allow duplicate elements and preserve insertion order. You can access list elements by iterating (using a for-each loop as shown) or possibly by index (though WFL tends to avoid direct index manipulation in favor of high-level operations). To create a list, you use a block syntax:

  ```wfl
  create list colors:
      add "red"
      add "green"
      add "blue"
  end list
  ``` 

  This example defines a list called `colors` (so a variable `colors` of type “list of text”) with three entries ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=,end%20list)). Inside the block, each **`add "value"`** appends an element to the list. After execution, `colors` contains `["red","green","blue"]` in that order. You can also start an empty list: e.g. **`create list tasks: end list`** would make an empty list. Once you have a list, you can use statements like **`add <item> to <list>`** and **`remove <item> from <list>`** to manipulate it ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=After%20creating%20a%20list%2C%20you,now%20shopping%20is%20empty)). Removing by value will remove the first matching element (if present). You can also **`clear <list name>`** to empty a list ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=%60%60%60wfl%20add%20,now%20shopping%20is%20empty)). Lists in WFL are dynamic (you can add/remove at runtime) but type-homogeneous (all elements must be the same type, or a subtype). If you try to add an element of the wrong type, it’s a compile-time error (or runtime error if somehow it wasn’t caught, but generally the compiler knows the list’s element type).

- **Set of T:** A collection of unique elements of type T (no duplicates, order not guaranteed). The type is “set of T”. Syntax is similar to list but using **set**:

  ```wfl
  create set unique tags:
      add "news"
      add "sports"
      add "news"
  end set
  ```

  After this, `unique tags` (a set of text) will contain `"news"` and `"sports"` only once each – the second attempt to add "news" is ignored to maintain uniqueness ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=,end%20set)). Sets support operations like add, remove, and clear, just like lists, but when you add an element that is already in the set, it has no effect (and WFL may ignore it silently or give a gentle warning). Sets are useful when you just care about membership and not ordering or duplicates (for example, a set of user IDs that have logged in).

- **Map (Dictionary):** A map holds key-value pairs, like a dictionary or associative array. Each entry maps a **key** to a **value**. In WFL, keys are typically text (string identifiers), and values can be any type, but all entries in a single map usually share the same value type. The type is “map of K to V”, commonly “map of text to T”. The syntax to create a map is:

  ```wfl
  create map settings:
      theme is "dark"
      volume is 75
      notifications are on
  end map
  ```

  This defines a map called `settings` with three entries ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=,are%20on%20end%20map)). For each entry, we write `key is value` (or “are” for plural-named keys, as in **notifications are on** ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=Here%20we%20created%20a%20map,on%20is%20synonymous%20with%20yes%2Ftrue))). Here, the keys are `theme`, `volume`, `notifications` and their values are `"dark"` (text), `75` (number), and `on` (which is yes/true). After this, `settings` is a map of text to variant types – although in this example the values aren’t all the same type (string vs number vs boolean). In a real program, you might keep map values uniform (all text, or all some custom type), but WFL doesn’t strictly require it, since a configuration map might legitimately hold different kinds of values. However, if you plan to enforce a type, you could declare the map type explicitly or ensure you only add matching types. Accessing a map’s entry might be done with a phrase like **`get <key> from <map>`**. For instance, `get volume from settings` would retrieve 75. You can update an entry by assigning a new value to a key (perhaps with a syntax like `change settings volume to 80` or simply using the same `... is ...` construct on an existing map). Removing an entry can be done with **`remove <key> from <map>`** (e.g., `remove notifications from settings` would delete that entry, perhaps making it as if it was never set). Maps allow flexible, dictionary-style data storage while keeping a natural syntax for access.

- **Record:** A record is similar to a map in structure (it’s a collection of named fields), but it is usually used as a fixed structured type with a known set of fields (like a lightweight object or struct). Think of a record as representing one “entity” with multiple attributes, each attribute having its own type. For example, a `person` record might have a name (text), age (number), and email (text). We saw how to create a record:

  ```wfl
  create record person:
      name is "John Smith"
      age is 30
      email is "john@example.com"
      is active is yes
  end record
  ```

  This defines a record variable `person` with four fields ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=create%20record%20person%3A%20name%20is,john%40example.com)). The type of `person` is a record type with schema `{ name: text, age: number, email: text, is active: boolean }`. Unlike maps, records are meant to have a predetermined set of fields (though WFL allows removing a field, it’s somewhat like setting it to nothing or truly deleting it, depending on semantics). Accessing record fields is done in a very English way. You can use a possessive style or an “of” style. For example, **`get person's email`** would retrieve "john@example.com" ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=Using%20a%20record%20feels%20intuitive%3A,might%20not%20actually%20be%20required)). Here we use the possessive form `person's email` to denote the email field of person. (In code, the apostrophe might not be strictly required; WFL might allow `get person email` or more clearly **`get email of person`**. The documentation suggests the `'s` is a natural-language flourish, and the actual syntax might treat it as optional or just use “of”. In any case, the intent is to make field access read like “the person’s email”.) Similarly, **`display person age`** might show the age, or `change person age to 31` would update the age field.

  Records are like custom types you can create on the fly. You can pass a record to an action expecting that record type, and it will know the field types. WFL likely has a concept of **structural typing** for records – if two records have the same field names and types, they might be interchangeable. Or it might treat each `create record X:` as creating an anonymous type just for that variable. In any case, records give you a way to bundle related data. They improve program clarity because instead of parallel arrays or loose maps, you group data under one name (`person` in this case).

  You can remove or add fields to a record at runtime if needed: e.g., `remove person's email` would drop the email field from the `person` record ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=match%20at%20L288%20,nothing%20or%20making%20it%20unavailable)). If you remove a field and then try to access it, it might be treated as `nothing` or an error, depending on context. WFL’s type system would ideally catch incorrect field usage, but if records are open-ended, it might be a runtime check. However, since the record was created with an email, code written expecting `person` to have an email should be valid until we remove it. (This is an advanced corner – generally, you wouldn’t remove fields unless truly needed.)

In summary, compound types in WFL carry their type information (like “list of text” or the specific record schema) which the compiler uses for type checking. They can all be nested: you can have a list of records, a record that contains a map, a map whose values are lists, etc. The syntax for nesting just follows from the constructs (e.g., you could `add` a record into a list, or have a record field whose value *is* a list by writing `phones is list: ... end list` inside a record definition).

### Type Inference  
WFL’s compiler automatically infers the types of variables and expressions based on how you use them, which means you rarely need to explicitly state types except in function signatures or when you want to enforce a certain type. Here are some rules and examples of type inference in action:

- **Literal Inference:** A literal number (e.g. `5`, `3.14`) is inferred as a `number`. A quoted value (`"hello"`) is a `text`. **yes/no** are booleans. The literal **nothing** is a special case – typically it can fit any reference type, but the compiler will try to tie it to an expected type context (if you do `store value as nothing`, it might default to a general “unknown” type until you use `value` in a context that gives it a type, or require a cast).

- **Variable Declaration:** In **`store x as <Expr>`**, the type of `x` is inferred from `<Expr>`. For example, `store total as 0` gives `total` type `number` (since 0 is a number literal). `store greeting as "Hi"` makes `greeting` type `text`. If you do `store list1 as []` (imagine an empty list literal, if supported), the compiler might not know the element type immediately and could default to something like “list of any” which then gets refined once you add a specific type element. However, since WFL uses English block syntax for list creation rather than inline `[]`, the type is clear from the `add` lines. For instance, after:
  ```wfl
  create list items:
      add 1
      add 2
  end list
  ```
  the compiler infers `items` is a list of number because we added numbers to it. If you tried to `add "hello"` (a text) to that list later, it would be a type error.

- **Expressions:** If you write `x plus y` and `x` and `y` are numbers, the result is a number. If one was not a number, the compiler would complain or attempt a conversion if it’s unambiguous. For instance, `text1 with text2` yields a text (string concatenation), and if you accidentally wrote `text1 with 5` (number with text), WFL might either auto-convert the 5 to text `"5"` or give an error advising you to convert it. Given the emphasis on type safety, it likely requires you to explicitly convert the number to text (e.g., `5 as text`) before concatenation, to avoid surprises. This is where an error like “Expected text but found number – try converting it first” would pop up, guiding you.

- **Function Calls:** When you call an action, the compiler knows what type it returns from the action’s definition. For example, performing `add numbers` (from our earlier example) is known to return a number, so `store result as perform add numbers with ...` infers `result` as number. If you pass arguments of the wrong type (say `perform add numbers with x as "5" ...` where x should be a number but you gave text), that’s a compile error with a clear message. The names in the `with` clause ensure even if you swap arguments mistakenly, it’s caught.

- **Conditional Inference:** Within branches of an `if` or `when`, the compiler could infer certain things. For example, if you do:
  ```wfl
  check if value is nothing:
      // handle missing
  otherwise:
      // here value is definitely not nothing
      display value
  end check
  ```
  In the otherwise branch, `value` can be treated as non-null (not nothing). This is a form of flow-sensitive typing or refinement. Similarly, if you had a union type or an action overload by type, a `when ...` might narrow the type. (This is speculative, but given the pattern matching hint in guiding principles, WFL might allow things like `when X is a number:` vs `when X is text:` to handle different types in one block – effectively a type-based pattern match.)

- **Type Annotations:** If needed, WFL allows explicit type annotations in some places to assist the compiler or enforce a specific type. For instance, in function definitions we explicitly write types for parameters and return. You might also be able to annotate a variable if the compiler couldn’t infer it or if you want to ensure it’s a certain type. Perhaps something like `store count as number 0` (though the grammar currently uses `as` for assigning value, so maybe a different keyword like `store count of type number as 0` – but that’s clunky). More likely, if you wanted to declare a variable without initializing (not common in WFL since you typically initialize immediately), they showed an example with `shared create user count as 0` in a module – here `0` tells the type anyway. Or `shared create flag as no` (type boolean). If you truly wanted to declare a type without a value, maybe WFL has a syntax for default values or a placeholder. However, best practice is to always initialize, letting inference do its job.

- **Conversions:** WFL provides simple phrases to convert types, which helps with type compatibility. For example, **`convert X to number`** (if X is text that looks like a number), or **`X as text`** (to explicitly treat/convert X to text). These produce a value of the target type at runtime (or fail in a controlled way if conversion isn’t possible, which you can handle with `when invalid` as shown in the safe conversion example ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=match%20at%20L361%20,where%20the%20value%20was%20missing%2Fundefined))). Using these conversion keywords informs the compiler of the intended type of the result (and also inserts a runtime conversion). For instance, after `store n as convert "123" to number`, `n` is a number (with value 123). If conversion might fail, WFL forces you to handle that failure via a `when invalid` block attached to the convert operation, ensuring type safety (the program won’t blithely continue with a wrong type).

In essence, the type system is designed to be **strong** (no silent type coercions that could lead to bugs) but also **flexible** thanks to inference and high-level conversions. Beginners can often ignore the fact that types are there because WFL rarely requires them to write type names – the code *“Let the age be 25”* or *“store age as 25”* is enough for the compiler to assign a type. But behind the scenes, this prevents a whole class of errors (e.g., using a text where a number is expected will be caught at compile time). 

### Scoping Rules  
Scope determines where a variable or definition is visible (can be accessed) in the program. WFL uses a **lexical scope** model with clear rules, using natural terms like *local*, *global*, and *shared* to describe scope boundaries.

- **Local Variables:** By default, a variable created inside an action (function) or other block is local to that block. For example, if you `store temp as 5` inside a function or inside a `check if` block, that `temp` only exists within that block and its sub-blocks. When the block ends, the variable goes out of scope (cannot be accessed beyond it). Each invocation of an action gets its own copies of local variables (no sharing between calls, unless explicitly using shared/global). This is analogous to traditional function-local variables in other languages.

- **Parameters:** Function parameters (`needs:` variables) are local to the function body (they act like pre-initialized locals). They cease to exist after the action ends, except for the returned value which is passed out.

- **Global Variables:** If you declare a variable at the top level of a program (outside any action or module), it is a **global** variable. Global variables are accessible from any part of the program (any action or code block) unless shadowed by a local of the same name. WFL likely discourages excessive use of globals in favor of passing things as parameters or using modules, but they are available for truly global state or configuration. You might mark a global explicitly with the keyword **`global`** for clarity, e.g. `global store max connections as 10`. If not explicitly marked, any top-level `store` might be considered global by context.

- **Shared (Module-Level) Variables:** WFL introduces the concept of **shared** scope, which is a middle ground between local and global. A **shared** variable is one that is shared across a group of related code (like within a module or file), but not visible globally elsewhere ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=,wfl)) ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=shared,which%20helps%20prevent%20accidental%20misuse)). Think of modules as encapsulated units (like files or namespaces). If you declare `shared create user count as 0` at the module level, all actions and code in that module/file can see `user count`, but code in a different module cannot. This helps organize code by separating internal module state from truly global state. Shared variables prevent name collisions across modules and encourage encapsulation (for example, each module could have a `shared error message` variable for internal use, and they wouldn’t conflict). To use shared variables, you typically declare them at the top of a module with the **shared** keyword, as shown. Shared variables are often used for configuration or state that needs to be accessed by multiple actions in the same module (e.g., a cache, or a reference to a shared resource).

- **Block Scope:** Any time you open a block with a colon (like in loops, if/else, try/when, etc.), you can create new variables inside that block. Those variables are not visible outside the block. For instance, in `for each item in shopping: ... end for`, the loop variable `item` is only accessible inside that loop’s block. After `end for`, if you try to use `item`, it won’t exist (or refers to an outer `item` if one was declared outside). Similarly, variables inside a `try` or an `if` branch are scoped to that block. WFL uses indentation and explicit end markers, but behind the scenes it’s creating a new lexical scope for each block.

- **Name Shadowing:** If you declare a new variable with the same name as an existing variable in an outer scope, it will **shadow** the outer one within the inner block. For example:
  ```wfl
  store count as 5
  display count          // displays 5
  if condition:
      store count as 10   // this is a new local 'count' in the if-block, shadows outer 'count'
      display count      // displays 10 (inner count)
  end check
  display count          // displays 5 (outer count, since inner is out of scope now)
  ```
  This behavior is similar to most languages. It’s often best to avoid reusing names in this way unless there’s a good reason, to keep code clear. WFL likely encourages descriptive names to minimize accidental shadowing (and may warn if you shadow a variable unintentionally).

- **Constants:** While not explicitly asked, WFL might allow declaring constants (immutable values). For instance, perhaps using `constant` or by context (maybe if you use `create` without the intent to change). The variables doc mentioned a possibility: *“You can mark a value that never changes”* – possibly a syntax like `create constant PI as 3.14159`. Constants would follow the same scope rules (global constant vs local constant) but just cannot be reassigned after initialization. They are useful for configuration values, fixed thresholds, etc.

- **Scope of Functions (Actions):** An action defined at the top level (module global) is accessible globally or within that module depending on if modules are used. If WFL supports modules, you might have to import or qualify action names from other modules. The spec doesn’t detail modules beyond the `shared` concept. Likely, if you have multiple modules, actions and shared variables can be imported by name. If no module system is considered here, assume all action names are global (or module-local if files act as modules, with an import mechanism).

- **Lifecycle:** Global and shared variables are typically initialized at program start (or module load) and persist for the program’s duration (or until the module is unloaded, if that concept exists). Local variables are created when their block is entered and destroyed when it exits. Memory for them is managed automatically (discussed below). There is no manual deletion of variables; going out of scope suffices for the system to reclaim them.

To put it plainly, **global** means accessible everywhere, **shared** means accessible to a specific region (module), **local** means only in the current action or block, and WFL uses those terms to make scope intentions explicit ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=shared,which%20helps%20prevent%20accidental%20misuse)). This approach is intended to be easy to understand – “shared” does what it sounds like (shared in some, but not all, places), “local” means local, etc. ([wfl-vars.md](file://file-MSrTCHF2wFfKpJ1Xxem8Tg#:~:text=match%20at%20L555%20words%20,code%20organized%20as%20it%20grows)).

### Memory Management  
WFL abstracts away manual memory management, providing automatic memory handling so developers don’t have to worry about allocation or deallocation of objects. When you create variables, lists, records, etc., the WFL runtime manages the memory behind the scenes. There are a few aspects to highlight:

- **Automatic Garbage Collection:** WFL’s implementation uses a garbage-collected model (or equivalent) to reclaim memory. This means when values (objects, lists, records, etc.) are no longer referenced by any variable, they will eventually be cleaned up by the system. Developers do not explicitly free memory. For example, if an action creates a record and returns it, and later no one references that record, it will be garbage-collected. This approach is chosen to prevent memory leaks and dangling pointers, aligning with the goal of safety and simplicity. Most likely, WFL is built on top of a VM (perhaps compiling to JavaScript or running on a managed runtime), so it leverages a **tracing garbage collector** similar to JavaScript’s or Java’s. This collector will periodically find objects with no references and free them.

- **Reference Counting (alternate):** It’s possible an implementation might use reference counting (with cycle detection) as an underlying strategy, but this is an implementation detail. From the language spec perspective, it doesn’t matter; what matters is that memory is automatically managed. There is no user-visible difference unless one considers performance characteristics. The spec doesn’t require the programmer to know which method is used, just that memory is handled for them.

- **No Manual `free` or `delete`:** Unlike lower-level languages, WFL has no commands to manually release memory. The programmer cannot accidentally double-free or forget to free memory – these issues are eliminated by design. If a programmer comes from C/C++, they might ask “how do I free this list?” The answer in WFL is: you don’t need to. Just remove references (perhaps setting a variable to nothing or letting it go out of scope) and the runtime will free it when appropriate. This significantly reduces classes of errors such as use-after-free or memory leaks, which are common pitfalls in manual memory management. (Of course, if you keep references around unintentionally, that’s a logical memory leak; but that’s solved by code review, not by language mechanism.)

- **Memory Safety:** WFL’s strong typing and managed memory means it’s memory-safe. You cannot read from an invalid pointer or write to memory out of bounds because those concepts aren’t exposed. For example, if you try to access an element outside the bounds of a list, WFL will throw a runtime error (like “index out of range” or simply not allow direct index access at all, forcing safe iteration). Buffer overflow, dangling pointer – these simply do not occur in valid WFL programs. This is crucial for security and robustness.

- **Ownership Model:** Languages like Rust use an explicit ownership model to manage memory without GC, but that introduces complexity (borrow checker, lifetimes). WFL opts for a **garbage-collected** or automated approach to keep things simple for the developer. The guiding principles emphasized accessibility and not overwhelming the user ([wfl-foundation.md](file://file-F9EzfjPYAxGa86rwaEV7UB#:~:text=11)), so an ownership model (which is powerful but complex) would conflict with that. Instead, WFL likely uses the time-tested GC approach that works well with a high-level language that often runs in web environments.

- **Resource Management:** While memory is GCed, some resources like file handles or network connections should be closed to avoid resource leaks. WFL encourages using `close` and/or `finally` blocks for these (as shown in I/O and error sections). Even if you forget, the garbage collector finalizer might close a file when its file object is collected, but relying on that is not best practice. So memory is automatically freed, but external resources should be explicitly closed.

- **Performance:** The spec might note that WFL implementations optimize allocation and garbage collection to minimize pauses (maybe using generational GC, etc.), but these details are beyond the concern of most users. However, it’s important that a WFL program can run reasonably efficiently; thus, under the hood, memory management is tuned. Short-lived objects (like a record inside a quick loop) are cleaned quickly, and long-lived ones (like a big list that persists) are managed in a way that doesn’t incur repeated overhead. The principle of performance transparency ([wfl-foundation.md](file://file-F9EzfjPYAxGa86rwaEV7UB#:~:text=13)) suggests WFL tries to optimize without making the programmer think about it.

- **Memory Model for Concurrency:** Since WFL supports async and possibly parallel tasks, one might wonder about memory consistency across threads. If WFL is single-threaded (like JavaScript event loop style), there’s no race condition issue. If it allows multi-threading (not explicitly indicated, likely not at the language level beyond async tasks that run concurrently but maybe still on one thread or using worker threads), the memory model would need to ensure thread-safe GC and possibly that shared data is synchronized. However, given the focus is on simplicity, WFL might abstract concurrency as asynchronous tasks in a single thread or cooperative multi-tasking. So we likely don’t have to specify a low-level memory model for threads (no atomic or volatile in WFL at the language level).

In conclusion, WFL uses a **managed memory approach** – either garbage collection or similar automatic reclamation – to handle memory safely. Developers can allocate freely (create lists, records, etc.) and trust the system to clean up. The absence of manual memory chores aligns with WFL’s goal of letting programmers focus on logic and not on pitfalls of memory management.

## Conclusion  
The WebFirst Language brings together the above syntax and semantic rules to create a programming experience that is both beginner-friendly and powerful. Its formal grammar is defined to enforce consistency (so that tools can parse and compile it), but every rule in the grammar corresponds to a readable English-like construct. From **variables** (“Let X be Y” style declarations) to **control flow** (if/else and loops that read like instructions), **functions** (actions defined and called in descriptive ways), and **error handling** (“try ... when ...” blocks that narrate failure cases), WFL stays true to its guiding philosophy of **natural-language alignment, minimal symbols, clarity, and safety**. 

The semantics ensure that programs behave reliably: the strong type system catches mistakes early (with helpful messages), scoping rules prevent unintended interactions, and automatic memory management lets developers build complex web applications without worrying about low-level errors. WFL’s design is informed by the needs of modern web development (with first-class support for async operations and integration with web APIs) while keeping the syntax accessible to someone who might be writing their first lines of code.

By following this specification, implementers of WFL can create compilers or interpreters that uphold these syntax rules and semantics, and developers can write WFL code with confidence that it will do what it intuitively says. The end result is a language specification that reads almost like a tutorial – just as WFL code reads like plain English – fulfilling the language’s mission of making web programming more intuitive, inclusive, and robust.  

