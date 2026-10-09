//! Issue #701: a container action calls another action of the same container
//! through the `this` receiver (`this.emit("x")`).
//!
//! `this` means "the object this action was called on", so the form inside a
//! container mirrors the form outside it (`m.emit("x")`). The bare forms
//! (`emit("x")`, `emit with "x"`, `call emit with "x"`) stay errors, but the
//! diagnostic names the `this.` form so the reader knows the fix.
//!
//! The end-to-end tests drive the real `wfl` binary (lex -> parse -> analyze
//! -> typecheck -> interpret) and assert stdout plus the exit status. The
//! analyzer/type-checker tests pin the static layer on its own.

use std::fs;
use std::process::Command;
use tempfile::TempDir;
use wfl::analyzer::Analyzer;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::{Parser, ast::Program};
use wfl::typechecker::TypeChecker;

/// Run `source` with the real `wfl` binary; return (exit code, stdout, stderr).
fn run(source: &str) -> (Option<i32>, String, String) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("main.wfl");
    fs::write(&path, source).expect("write program");
    let output = Command::new(env!("CARGO_BIN_EXE_wfl"))
        .arg(&path)
        .current_dir(dir.path())
        .env("NO_COLOR", "1")
        .output()
        .expect("run wfl");
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// Run `source` and require a clean exit; return stdout.
fn run_ok(source: &str) -> String {
    let (status, stdout, stderr) = run(source);
    assert_eq!(
        status,
        Some(0),
        "program must exit 0\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );
    stdout
}

fn parse(source: &str) -> Program {
    Parser::new(&lex_wfl_with_positions(source))
        .parse()
        .unwrap_or_else(|error| panic!("parse failed: {error:?}"))
}

/// Semantic errors (fatal) the analyzer reports for `source`.
fn semantic_errors(source: &str) -> Vec<String> {
    match Analyzer::new().analyze(&parse(source)) {
        Ok(()) => Vec::new(),
        Err(errors) => errors.into_iter().map(|error| error.message).collect(),
    }
}

/// Type-checker diagnostics for `source`.
fn type_errors(source: &str) -> Vec<String> {
    match TypeChecker::new().check_types(&parse(source)) {
        Ok(()) => Vec::new(),
        Err(error) => error
            .into_diagnostics()
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect(),
    }
}

/// The issue's reproduction, written with the `this` receiver.
const ISSUE_701_THIS: &str = r#"create container M:
    property buf: Text
    action emit needs ch: Text:
        store buf as buf with ch
    end
    action step:
        this.emit("x")
    end
end
create new M as m:
    buf is ""
end
m.step()
display "buf=[" with m.buf with "]"
"#;

#[test]
fn issue_701_reproduction_with_this_receiver_prints_buf_x() {
    let stdout = run_ok(ISSUE_701_THIS);
    assert!(
        stdout.contains("buf=[x]"),
        "sibling call must append to buf: {stdout}"
    );
}

#[test]
fn analyzer_and_type_checker_accept_this_inside_instance_actions() {
    assert_eq!(semantic_errors(ISSUE_701_THIS), Vec::<String>::new());
    assert_eq!(type_errors(ISSUE_701_THIS), Vec::<String>::new());
}

#[test]
fn caller_and_callee_property_writes_stay_coherent() {
    // The caller writes before and after the call; the callee writes in the
    // middle. Every write must survive, and the caller must see the callee's
    // write through both the bare name and `this.buf`.
    let stdout = run_ok(
        r#"create container M:
    property buf: Text
    action emit needs ch: Text:
        store buf as buf with ch
    end
    action step: Text
        store buf as buf with "a"
        this.emit("b")
        store seen as buf
        store buf as buf with "c"
        return seen with "|" with this.buf
    end
end
create new M as m:
    buf is ""
end
display "step=" with m.step()
display "buf=" with m.buf
"#,
    );
    assert!(stdout.contains("step=ab|abc"), "{stdout}");
    assert!(stdout.contains("buf=abc"), "{stdout}");
}

#[test]
fn this_property_read_sees_the_actions_latest_write() {
    let stdout = run_ok(
        r#"create container M:
    property buf: Text
    action step:
        store buf as "new"
        display "bare=" with buf with " this=" with this.buf
    end
end
create new M as m:
    buf is "old"
end
m.step()
"#,
    );
    assert!(stdout.contains("bare=new this=new"), "{stdout}");
}

#[test]
fn sibling_return_values_and_forward_references() {
    // `report` is declared before the actions it calls.
    let source = r#"create container Calc:
    property v: Number
    action report: Text
        return "double=" with this.doubled() with " triple=" with this.tripled()
    end
    action doubled: Number
        return v times 2
    end
    action tripled: Number
        return v times 3
    end
end
create new Calc as c:
    v is 21
end
display c.report()
"#;
    assert_eq!(type_errors(source), Vec::<String>::new());
    let stdout = run_ok(source);
    assert!(stdout.contains("double=42 triple=63"), "{stdout}");
}

#[test]
fn inherited_and_overridden_actions_dispatch_on_the_real_object() {
    // `this.sound()` inside the parent's `introduce` must reach the child's
    // override when the object is a Dog, and the parent's own action when it
    // is a plain Animal. The child reaches the inherited `introduce`.
    let source = r#"create container Animal:
    property name: Text
    action sound: Text
        return "..."
    end
    action introduce: Text
        return name with " says " with this.sound()
    end
end
create container Dog extends Animal:
    action sound: Text
        return "Woof"
    end
    action greet: Text
        return "Hi! " with this.introduce()
    end
end
create new Dog as rex:
    name is "Rex"
end
create new Animal as thing:
    name is "Thing"
end
display rex.greet()
display thing.introduce()
"#;
    assert_eq!(semantic_errors(source), Vec::<String>::new());
    let stdout = run_ok(source);
    assert!(stdout.contains("Hi! Rex says Woof"), "{stdout}");
    assert!(stdout.contains("Thing says ..."), "{stdout}");
}

#[test]
fn nested_call_on_another_object_runs_on_that_object() {
    // Before #701 the inner frame reused the caller's `this` and property
    // bindings, so `b.bump()` called from inside `a.poke(...)` ran on `a`
    // (printing "a.n=1 b.n=1"). Each call must own its object.
    let stdout = run_ok(
        r#"create container Counter:
    property n: Number
    property label: Text
    property last_seen: Text
    action inc:
        change n to n + 1
    end
    action bump:
        this.inc()
        change last_seen to label
    end
    action poke needs other: Counter:
        other.bump()
    end
end
create new Counter as a:
    n is 0
    label is "a"
    last_seen is ""
end
create new Counter as b:
    n is 100
    label is "b"
    last_seen is ""
end
a.poke(b)
display "a.n=" with a.n with " b.n=" with b.n
display "a.seen=[" with a.last_seen with "] b.seen=[" with b.last_seen with "]"
a.poke(a)
display "self-poke a.n=" with a.n with " a.seen=[" with a.last_seen with "]"
"#,
    );
    assert!(stdout.contains("a.n=0 b.n=101"), "{stdout}");
    assert!(stdout.contains("a.seen=[] b.seen=[b]"), "{stdout}");
    // Reaching the same object through another name is still the same
    // object: the caller's frame and the callee agree on its state.
    assert!(stdout.contains("self-poke a.n=1 a.seen=[a]"), "{stdout}");
}

#[test]
fn recursive_sibling_calls_keep_every_frame_coherent() {
    let stdout = run_ok(
        r#"create container Countdown:
    property log: Text
    action tick needs k: Number:
        check if k is greater than 0:
            change log to log with k
            this.tick(k minus 1)
            change log to log with "."
        end check
    end
end
create new Countdown as c:
    log is ""
end
c.tick(3)
display "log=" with c.log
"#,
    );
    assert!(stdout.contains("log=321..."), "{stdout}");
}

/// Wrap one statement as the body of `step` in a two-action container.
fn container_calling(call: &str) -> String {
    format!(
        "create container M:\n    property buf: Text\n    action emit needs ch: Text:\n        store buf as buf with ch\n    end\n    action step:\n        {call}\n    end\nend\ncreate new M as m:\n    buf is \"\"\nend\nm.step()\n"
    )
}

#[test]
fn bare_sibling_calls_are_rejected_with_a_this_suggestion() {
    for call in [r#"emit("x")"#, r#"emit with "x""#, r#"call emit with "x""#] {
        let source = container_calling(call);
        let (status, stdout, stderr) = run(&source);
        let output = format!("{stdout}{stderr}");
        assert_eq!(
            status,
            Some(3),
            "`{call}` must stay a semantic error: {output}"
        );
        assert!(
            output.contains("this.emit("),
            "`{call}` error must point at the this.emit(...) form: {output}"
        );
        assert!(
            output.contains("container 'M'"),
            "`{call}` error must name the container: {output}"
        );
        assert!(
            !stdout.contains("buf="),
            "a rejected program must not run: {output}"
        );
    }
}

#[test]
fn this_is_rejected_in_static_actions_with_a_clear_reason() {
    let source = r#"create container M:
    static property total: Number defaults 0
    action helper: Number
        return 1
    end
    static action build: Number
        return this.helper()
    end
end
display M.build()
"#;
    let errors = semantic_errors(source);
    assert!(
        errors
            .iter()
            .any(|message| message.contains("'this'") && message.contains("static action")),
        "static actions have no object, and the error must say so: {errors:?}"
    );
    let (status, _, _) = run(source);
    assert_eq!(status, Some(3));
}

#[test]
fn the_object_this_cannot_be_reassigned_inside_an_action() {
    // `change this to ...` was already an error before #701 (`this` was
    // undefined); it stays one, and now says why.
    let source = r#"create container M:
    property buf: Text
    action step:
        change this to "other"
    end
end
create new M as m:
    buf is ""
end
m.step()
"#;
    let errors = semantic_errors(source);
    assert!(
        errors
            .iter()
            .any(|message| message.contains("'this'") && message.contains("cannot be changed")),
        "{errors:?}"
    );
    let (status, _, _) = run(source);
    assert_eq!(
        status,
        Some(3),
        "`change this to` must stay a semantic error"
    );
}

#[test]
fn existing_variables_named_this_keep_their_meaning_inside_actions() {
    // GOVERNANCE.md 3.1: `this` is additive. Wherever a program already had
    // its own variable named `this` in scope, that variable keeps its
    // meaning; `this` means the object only where the name was undefined.
    let stdout = run_ok(
        r#"store this as 41
create container G:
    property v: Number
    action run:
        display "global this is " with this
    end
end
create new G as g:
    v is 1
end
g.run()
display "outer this " with this
"#,
    );
    assert!(stdout.contains("global this is 41"), "{stdout}");
    assert!(stdout.contains("outer this 41"), "{stdout}");

    let stdout = run_ok(
        r#"create container L:
    property items: List
    action run:
        store this as 5
        display "local " with this
        for each this in items:
            display "item " with this
        end for
    end
end
create new L as l:
    items is [1, 2]
end
l.run()
"#,
    );
    assert!(stdout.contains("local 5"), "{stdout}");
    assert!(stdout.contains("item 1"), "{stdout}");
    assert!(stdout.contains("item 2"), "{stdout}");
}

#[test]
fn recursion_through_this_keeps_each_calls_locals() {
    // Review finding (#701): a callee's `store a as ...` used to land in the
    // caller's `a`, so this printed "fib(10)=5".
    let stdout = run_ok(
        r#"create container Math:
    property calls: Number
    action fib needs n: Number: Number
        change calls to calls + 1
        check if n is less than 2:
            return n
        end check
        store a as this.fib(n minus 1)
        store b as this.fib(n minus 2)
        return a plus b
    end
end
create new Math as mm:
    calls is 0
end
display "fib(10)=" with mm.fib(10)
display "calls=" with mm.calls
"#,
    );
    assert!(stdout.contains("fib(10)=55"), "{stdout}");
    assert!(stdout.contains("calls=177"), "{stdout}");
}

#[test]
fn a_called_action_cannot_overwrite_its_callers_locals() {
    // Both a sibling caller and an ordinary action calling a method used to
    // see `tmp=99`: the callee's local landed in the caller's `tmp`.
    let stdout = run_ok(
        r#"create container M:
    property total: Number
    action helper: Number
        store tmp as 99
        return tmp
    end
    action run:
        store tmp as 1
        store r as this.helper()
        display "sibling: tmp=" with tmp with " r=" with r
    end
end
create new M as m:
    total is 0
end
m.run()
define action called go with parameters o:
    store tmp as 1
    store r as o.helper()
    display "action: tmp=" with tmp with " r=" with r
end action
call go with m
"#,
    );
    assert!(stdout.contains("sibling: tmp=1 r=99"), "{stdout}");
    assert!(stdout.contains("action: tmp=1 r=99"), "{stdout}");
}

#[test]
fn passing_this_to_an_ordinary_action_keeps_the_object_coherent() {
    // The helper reaches the same object through its parameter. It must see
    // the caller's latest write, and its own write must survive the caller's
    // later writes (it used to read "old" and the result was "az").
    let stdout = run_ok(
        r#"define action called show_it with parameters o:
    display "helper sees buf=[" with o.buf with "]"
    o.emit("H")
end action
create container M:
    property buf: Text
    action emit needs ch: Text:
        store buf as buf with ch
    end
    action step:
        store buf as "a"
        call show_it with this
        store buf as buf with "z"
    end
end
create new M as m:
    buf is "old"
end
m.step()
display "buf=[" with m.buf with "]"
"#,
    );
    assert!(stdout.contains("helper sees buf=[a]"), "{stdout}");
    assert!(stdout.contains("buf=[aHz]"), "{stdout}");
}

#[test]
fn arguments_that_call_the_receiver_run_before_the_callee_starts() {
    // `this.emit(this.next_char())`: the argument's write to `n` must not be
    // lost when `emit` writes its result back.
    let stdout = run_ok(
        r#"create container M:
    property buf: Text
    property n: Number
    action emit needs ch: Text:
        store buf as buf with ch
    end
    action next_char: Text
        change n to n + 1
        return "c" with n
    end
    action step:
        store buf as buf with "a"
        this.emit(this.next_char())
        this.emit(this.next_char())
        store buf as buf with "z"
    end
end
create new M as m:
    buf is ""
    n is 0
end
m.step()
display "buf=[" with m.buf with "] n=" with m.n
"#,
    );
    assert!(stdout.contains("buf=[ac1c2z] n=2"), "{stdout}");
}

#[test]
fn deep_and_cross_object_call_chains_keep_every_write() {
    // Three levels on one object, then a chain that leaves the object and
    // comes back to it through another object (`c.kickoff(d)` ->
    // `d.relay(c)` -> `c.leaf()`).
    let stdout = run_ok(
        r#"create container Node:
    property log: Text
    action leaf:
        change log to log with "L"
    end
    action middle:
        change log to log with "m"
        this.leaf()
        change log to log with "M"
    end
    action top:
        change log to log with "t"
        this.middle()
        change log to log with "T"
    end
    action relay needs caller_node: Node:
        change log to log with "r"
        caller_node.leaf()
    end
    action kickoff needs other: Node:
        change log to log with "s"
        other.relay(this)
        change log to log with "S"
    end
end
create new Node as a:
    log is ""
end
create new Node as c:
    log is ""
end
create new Node as d:
    log is ""
end
a.top()
display "deep=[" with a.log with "]"
c.kickoff(d)
display "chain c=[" with c.log with "] d=[" with d.log with "]"
"#,
    );
    assert!(stdout.contains("deep=[tmLMT]"), "{stdout}");
    assert!(stdout.contains("chain c=[sLS] d=[r]"), "{stdout}");
}

#[test]
fn this_calls_are_type_checked_against_inherited_actions() {
    // Arity and argument types through `this` are checked for an action the
    // container inherits, not only for its own actions.
    for (call, needle) in [
        (
            "this.twice(1, 2)",
            "expects 1 arguments but 2 were provided",
        ),
        (r#"this.twice("a")"#, "Argument 1 of method 'twice'"),
    ] {
        let source = format!(
            "create container P:\n    property v: Number\n    action twice needs x: Number: Number\n        return x times 2\n    end\nend\ncreate container C extends P:\n    action run: Number\n        return {call}\n    end\nend\ncreate new C as c:\n    v is 1\nend\ndisplay c.run()\n"
        );
        let diagnostics = type_errors(&source);
        assert!(
            diagnostics.iter().any(|message| message.contains(needle)),
            "`{call}` must draw '{needle}': {diagnostics:?}"
        );
    }
}

#[test]
fn a_parameter_named_like_a_property_does_not_capture_a_sibling_write() {
    // `step`'s parameter `buf` hides the property inside `step` only. Before
    // #701 the sibling's write landed in that parameter and was lost
    // (printing "buf=p"); it must reach the property.
    let stdout = run_ok(
        r#"create container M:
    property buf: Text
    action emit needs ch: Text:
        store buf as buf with ch
    end
    action step needs buf: Text:
        this.emit(buf)
    end
end
create new M as m:
    buf is "p"
end
m.step("q")
display "buf=" with m.buf
"#,
    );
    assert!(stdout.contains("buf=pq"), "{stdout}");
}

#[test]
fn a_callers_local_named_like_a_property_does_not_leak_into_the_action() {
    // An action reads its own property, as the analyzer already assumes.
    // Before #701 a caller's same-named parameter or loop variable stood in
    // for the property ("label=caller-param", "label=loop-var") and was then
    // written into the object ("after: loop-var").
    let stdout = run_ok(
        r#"create container M:
    property label: Text
    action show: Text
        return "label=" with label
    end
end
create new M as m:
    label is "property"
end
define action called run with parameters label:
    display m.show()
end action
call run with "caller-param"
for each label in ["loop-var"]:
    display m.show()
end for
display "after: " with m.label
"#,
    );
    assert_eq!(
        stdout.matches("label=property").count(),
        2,
        "both calls must read the property: {stdout}"
    );
    assert!(!stdout.contains("caller-param"), "{stdout}");
    assert!(!stdout.contains("label=loop-var"), "{stdout}");
    assert!(stdout.contains("after: property"), "{stdout}");
}

#[test]
fn a_same_named_global_keeps_its_historical_precedence() {
    // Compatibility guard: a global visible where the container is defined
    // still wins over a same-named property inside the action, exactly as
    // before #701 and as the analyzer resolves the name. Changing that
    // precedence is a separate language decision.
    let stdout = run_ok(
        r#"store n as 7
create container C:
    property n: Number
    action show:
        display "inside n=" with n
    end
end
create new C as a:
    n is 1
end
a.show()
"#,
    );
    assert!(stdout.contains("inside n=7"), "{stdout}");
}

#[test]
fn a_sibling_error_caught_by_the_caller_keeps_both_frames_writes() {
    let stdout = run_ok(
        r#"create container M:
    property log: Text
    action risky:
        change log to log with "r"
        store bad as 1 divided by "x"
    end
    action run: Text
        change log to log with "a"
        try:
            this.risky()
        when error:
            change log to log with "!"
        end try
        change log to log with "z"
        return log
    end
end
create new M as m:
    log is ""
end
display "run=" with m.run()
display "log=" with m.log
"#,
    );
    assert!(stdout.contains("run=ar!z"), "{stdout}");
    assert!(stdout.contains("log=ar!z"), "{stdout}");
}

#[test]
fn calling_a_missing_or_misused_sibling_fails_loudly() {
    // An unknown action and a wrong argument count are reported by the type
    // checker and stop the program at run time; neither silently succeeds.
    for (call, needle) in [(r#"this.nowhere("x")"#, "nowhere"), ("this.emit()", "emit")] {
        let source = container_calling(call);
        let diagnostics = type_errors(&source);
        assert!(
            diagnostics.iter().any(|message| message.contains(needle)),
            "`{call}` must draw a type-checker diagnostic naming '{needle}': {diagnostics:?}"
        );
        let (status, stdout, stderr) = run(&source);
        assert_ne!(
            status,
            Some(0),
            "`{call}` must not exit cleanly\n{stdout}{stderr}"
        );
    }
}

#[test]
fn this_stays_an_ordinary_name_outside_container_actions() {
    // `this` is not a reserved word: programs that already use it as a plain
    // variable at the top level, or inside an ordinary action, keep working.
    let stdout = run_ok(
        r#"store this as 5
display "top=" with this
define action called twice with parameters amount:
    return amount times 2
end action
display "twice=" with twice of this
"#,
    );
    assert!(stdout.contains("top=5"), "{stdout}");
    assert!(stdout.contains("twice=10"), "{stdout}");
}
