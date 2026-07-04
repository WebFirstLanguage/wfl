# A Friendly WFL Tour 🧭

Eight tiny programs that take you from *"Hello, World!"* to objects — each one
**runs today** on the release build and prints exactly what you see below. They
live in this folder as real `.wfl` files, so you can run them yourself:

```bash
cargo build --release
target/release/wfl TestPrograms/docs_examples/tour/01_hello.wfl
```

Every example here was run against the release binary, and the code blocks below
match each program's real output. You can re-verify them yourself with the docs
harness pointed at this folder:

```bash
python3 scripts/test_docs_code_blocks.py --docs TestPrograms/docs_examples/tour
```

Read them in order — each builds on the last, and nothing you learn early has to
be unlearned later.

---

## 1. Hello, World! — [`01_hello.wfl`](01_hello.wfl)

One line, no ceremony. `display` prints a line of text.

```wfl
display "Hello, World!"
```
```
Hello, World!
```

## 2. Variables — [`02_variables.wfl`](02_variables.wfl)

`store` makes a variable; `change` updates it. Types are inferred, and you can
join values with `with`. Booleans are the friendly words `yes` / `no`.

```wfl
store name as "Alice"
store age as 30
store is_member as yes

display "Name: " with name
display "Age: " with age
display "Member? " with is_member
display "age is a " with typeof of age

change age to age plus 1
display "Next year: " with age
```
```
Name: Alice
Age: 30
Member? yes
age is a Number
Next year: 31
```

> Tip: `is` is a keyword, so name booleans `is_member`, not `is member`.

## 3. Conditionals — [`03_conditionals.wfl`](03_conditionals.wfl)

Comparisons are words (`is greater than or equal to`). To chain choices, **nest**
the next check inside `otherwise:` (a compact `otherwise check if …:` else-if
form also works — this tour uses the explicit nested style for clarity).

```wfl
store score as 82

check if score is greater than or equal to 90:
    display "Grade: A"
otherwise:
    check if score is greater than or equal to 80:
        display "Grade: B"
    otherwise:
        display "Grade: C or below"
    end check
end check
```
```
Grade: B
```

## 4. Loops — [`04_loops.wfl`](04_loops.wfl)

Three shapes: `count from … to …` (loop variable is `count`), `for each … in …`,
and `repeat while …`.

```wfl
display "Counting to 3:"
count from 1 to 3:
    display "  count is " with count
end count

store fruits as ["apple", "banana", "cherry"]
display "Fruits:"
for each fruit in fruits:
    display "  " with fruit
end for

store n as 3
repeat while n is greater than 0:
    display "  liftoff in " with n
    change n to n minus 1
end repeat
display "  liftoff!"
```
```
Counting to 3:
  count is 1
  count is 2
  count is 3
Fruits:
  apple
  banana
  cherry
  liftoff in 3
  liftoff in 2
  liftoff in 1
  liftoff!
```

## 5. Lists — [`05_lists.wfl`](05_lists.wfl)

`create list …:` (note the colon) or a `[…]` literal. Append with
`push with <list> and <value>`. Here we collect the even numbers.

```wfl
create list evens:
end list

count from 1 to 10:
    store remainder as count % 2
    check if remainder is equal to 0:
        push with evens and count
    end check
end count

display "Even numbers 1..10: " with evens
display "How many: " with length of evens
```
```
Even numbers 1..10: [2, 4, 6, 8, 10]
How many: 5
```

> Tip: compute `count % 2` into a variable first — `count % 2 is equal to 0`
> written inline currently mis-groups (see issue #571).

## 6. Actions — [`06_actions.wfl`](06_actions.wfl)

Define with `with parameters …`. Call a value-returning action with `of`
(`area of 4 and 5`); run a side-effecting one as a statement with `call`.

```wfl
define action called area with parameters width and height:
    return width times height
end action

define action called greet with parameters who:
    display "Hello, " with who with "!"
end action

display "Area 4x5 = " with area of 4 and 5
call greet with "world"
```
```
Area 4x5 = 20
Hello, world!
```

> Tip: `area with 4 and 5` does **not** call the action — `with` is string
> concatenation. Use `of`.

## 7. Error handling — [`07_errors.wfl`](07_errors.wfl)

Wrap risky work in `try:` and catch it with `when error:`. The caught message is
available as `error_message`. Execution continues afterward.

```wfl
try:
    open file at "/no/such/path.txt" for reading as f
    display "opened the file"
when error:
    display "Could not open file: " with error_message
end try

display "Program continues after the error."
```
```
Could not open file: Failed to open file /no/such/path.txt: No such file or directory (os error 2)
Program continues after the error.
```

## 8. Containers (objects) — [`08_containers.wfl`](08_containers.wfl)

Declare `property name: Type` and `action …: … end`; close the container with a
bare `end`. Build an instance with `create new … as …:` and call methods with
parentheses.

```wfl
create container Dog:
    property name: Text
    property tricks: Number

    action show_info:
        display name with " knows " with tricks with " tricks."
    end
end

create new Dog as rex:
    name is "Rex"
    tricks is 3
end

rex.show_info()
```
```
Rex knows 3 tricks.
```

---

**Where to next?** The full guides live in [`Docs/`](../../../Docs/README.md).
Start with [01 Introduction](../../../Docs/01-introduction/index.md) and
[03 Language Basics](../../../Docs/03-language-basics/index.md) — every code
example in those sections has been run and verified against this same binary.
