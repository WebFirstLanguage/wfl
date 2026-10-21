# Container instance binding after successful construction

Review of the `dev` → `main` promotion (#789, review 5529106541) found that
`create new` published the instance name before constructor arguments and
`initialize` ran. Issue #786 made those arguments reachable from source, so a
throwing argument or `initialize` left a half-built object in the environment.
A surrounding `try` could observe it, and creating the object again under the
same name failed.

The interpreter now finishes property defaults, the property body, argument
evaluation, and `initialize` (including an inherited action, with `this`)
before `define_direct`. An error leaves the environment unchanged. The no-arg
form still does not call `initialize`. The #785 constructor-path result
(`v = 6`) is unchanged.

The property write-back issue from the same review (`flush_method_frame`
copying a same-named outer binding) is a separate follow-up.
