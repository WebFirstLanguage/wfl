# WFL package system and package format — proposed PRD

**Status: Proposed design; not implemented or approved for release.**  
**Draft date:** 2026-09-23  
**Decision authority:** WFL Maintainers, under [GOVERNANCE.md](../../GOVERNANCE.md).  
**Implementation baseline:** `77ac72a02290433563957ff81ad453fffaffda9c`, WFL `26.9.18`.  
**Change classification:** R0, design documentation only. Future implementation is R3.

This document proposes a package built around an actual WFL **container
definition**. The package adds a static description, dependency graph, source
files, resources, and a distributable artifact to that definition. A manifest,
directory, archive, operating-system container, or container instance is not the
language container meant here.

WFL currently has **no package manager**. Every package command, package import,
format rule, and package-specific validation rule below is proposed. Sections
explicitly headed “Current” report verified existing behavior. `MUST`, `SHOULD`,
and `MAY` define requirements of this proposed standard, not new repository
policy. The recommendations are concrete enough to plan implementation; adoption
and the external trust-root decisions in §13 remain with Maintainers.

## 1. Purpose, users, scope, and success

### 1.1 Problem and users

Authors can already put reusable containers in files and share them using
`include from`. Copying those files between applications does not identify their
origin, record compatible dependencies, protect their public contract, or ensure
that every consumer receives the same code and resources. An archive alone does
not solve those problems.

Primary users are beginners sharing their first reusable container, library
authors maintaining stable contracts, application developers consuming libraries,
teams reproducing builds offline, and tool/registry implementers who need a
bounded, interoperable format. Editors and automation are consumers of the same
static metadata and diagnostics as people.

**PKG-001 — Container foundation.** A package MUST designate exactly one root
container definition in a declared source file and expose that definition as its
entry contract. A package without that definition is invalid, even if its
manifest and archive are syntactically valid.

**PKG-002 — No unlearning.** A beginner's reusable container MUST remain the
same definition when packaged. Adding dependencies, tests, resources, supporting
containers, or release metadata MUST NOT require replacing it with a singleton,
factory convention, special superclass, or second programming language.

### 1.2 Responsibility boundaries

| Layer | Owns | Does not own |
|---|---|---|
| Package format | Static metadata grammar, source membership, root contract, identity, archive and lock schemas | Running constructors, fetching code, hosting a service |
| Runtime/compiler integration | Package import binding, isolated definition environments, contract checks, resource lookup, execution lifecycle | Choosing newer versions or contacting registries during a run |
| Package tooling | Create, inspect, validate, resolve, install, build, test, update, remove, share; atomic local state | Implicit execution during inspection/install/build; registry ownership policy |
| Registry | Namespace ownership, immutable release records, archive availability, authenticated publication, yank/advisory information | Executing submitted packages or defining WFL container semantics |

### 1.3 v1 and non-goals

v1 includes source-only library packages, one root definition, private supporting
definitions, static typed contracts, development dependencies, local directory
and local archive sources, explicit registry sources, portable web resources,
lockfiles, deterministic artifacts, and the complete workflows in §8. A consuming
application may have dependency metadata without being a distributable package;
it does not need a dummy root container.

Not in v1: native binaries, installation/build hooks, code generation during
build, remote URL imports, Git dependency resolution, plugins loaded implicitly,
multiple root exports, cross-package inheritance, separate interface packages,
dynamic package names, multiple versions of one source-qualified package in one
graph, workspace orchestration, dependency vendoring, a registry website, or a
new general-purpose permission sandbox. Package delivery MUST NOT depend on
building WFLHub in WFL. Publishing a registry service is a separate project.

### 1.4 Measurable success criteria

| Measure | v1 acceptance target |
|---|---|
| Beginner journey | In a moderated study, at least 4 of 5 first-time WFL package users create, validate, build, and consume §9 in 15 minutes using its instructions |
| Growth | The §9 root container source remains byte-identical through packaging; adding §10's structure requires no alternate instance/call syntax |
| Inspection safety | All adversarial fixtures produce zero package-originated file writes, processes, network requests, or action calls during inspect/resolve/install/build |
| Reproducibility | Independent writers produce identical canonical metadata, lock bytes, and archive bytes for the same specified inputs on Windows, Linux, and macOS |
| Offline operation | A locked warm-cache application runs and builds with zero network attempts; a missing object fails with its identity and digest |
| Compatibility | Existing file-module, container, and removal regressions stay intact, subject only to explicit additive CLI help tests when tooling lands |
| Public boundaries | Every attempted private-definition access, undeclared dependency import, and invalid public type crossing fails before the target action body runs |
| Resource bounds | All §5/§6 size and graph limits have boundary and over-limit tests; failures leave no partial install or changed lock |

These are launch targets, not measured product claims. macOS is currently
best-effort, not a gated repository tuple; adding a package portability gate is
a delivery dependency, not an assertion that it exists today.

### 1.5 Terms

| Term | Meaning |
|---|---|
| Root container | The existing language definition selected as the package entry contract |
| Instance | Mutable runtime value created with `create new`; belongs to its consumer |
| Supporting container | A definition used internally by the root; not directly importable by another package |
| Module | A WFL source file and the existing file-loading mechanism; not itself a package |
| Manifest | Static data describing package/application inputs, never an executed program |
| Contract | Root members, types, construction rules, and documented behavioral obligations |
| Artifact | A canonical `.wflpkg` archive containing one package, without bundled dependencies |
| Source identity | Explicit registry origin or local locator, part of dependency identity |
| Lock | The exact selected dependency graph and digests; not a compatibility promise |

## 2. Research and current behavior

### 2.1 Governing sources

This proposal applies [CLAUDE.md](../../CLAUDE.md), all 19 principles and the
No-Unlearning Invariant in [wfl-foundation.md](../../Docs/wfl-foundation.md), the
[documentation policy](../../Docs/wfl-documentation-policy.md),
[testing.md](../../testing.md), and
[REPOSITORY_HYGIENE.md](../../REPOSITORY_HYGIENE.md). It also follows
[GOVERNANCE.md](../../GOVERNANCE.md), [CODE_OF_CONDUCT.md](../../CODE_OF_CONDUCT.md),
[AI_POLICY.md](../../AI_POLICY.md), [CONTRIBUTING.md](../../CONTRIBUTING.md), and
[SECURITY.md](../../SECURITY.md). No authority, release, or security-reporting
process is changed by this PRD.

### 2.2 Current: verified implementation, not inferred promises

| Subject | Observed behavior and evidence | Design consequence |
|---|---|---|
| Definition vs instance | `ContainerDefinition` and `ContainerInstantiation` are separate AST variants in [containers.rs](../../src/parser/stmt/containers.rs); corresponding runtime values live in [value.rs](../../src/interpreter/value.rs) | The root is a real definition, not a metadata map or pre-created object |
| Properties and methods | Parser accepts typed properties, methods, static members, `extends`, events, and `implements`; property visibility is set to `Public` | Do not invent current `private` syntax or claim it is enforced |
| Definition-time work | Interpreter's `ContainerDefinition` evaluates instance default expressions and static defaults; instance defaults use `.ok()` on evaluation, whereas static defaults propagate errors | Static package inspection cannot execute the definition; package declarations need a restricted initialization profile |
| State | Static values are held in shared `Rc<RefCell<...>>`; instance default values are cloned, including reference-valued defaults | Do not promise deep isolation for mutable defaults; v1 package defaults are scalar literals |
| Construction | The interpreter calls the direct `initialize` method when constructor arguments are supplied; an argument-free construction does not automatically call it | Packaging preserves this distinction; no hidden auto-start hook |
| Types | [typechecker_container_contract_test.rs](../../tests/typechecker_container_contract_test.rs) covers property types, inherited properties, constructor arguments, events, and inheritance cycles; [static_container_member_test.rs](../../tests/static_container_member_test.rs) covers shared static state | Reuse these contracts and add package-boundary checking, not an unrelated type model |
| Interfaces | [interface_contract_test.rs](../../tests/interface_contract_test.rs) exercises required methods, arity, return contracts, inheritance, and rejection of static methods as instance requirements | Interfaces are useful internally, but are not a replacement for the root container |
| Method overloads | Runtime method maps are keyed by name; `value.rs` documents the container-overload gap | Reject duplicate package method names; do not advertise container overloads |
| File includes | `IncludeStatement` executes in the caller environment. Successful includes that add bindings are remembered per visible scope; side-effect-only files rerun; failed includes can leave earlier definitions | Preserve file behavior; package binding needs separate atomic registration |
| File loads | `LoadModuleStatement` creates an isolated child scope with parent access and executes each time; definitions do not escape | Loading a file is not importing a package API |
| Type diagnostic severity | Ordinary included-file type errors are warnings; loaded-module type failures are fatal in the interpreter | Strict package validation is a new opt-in boundary, not a silent change to file modules |
| Included container discovery | The §9 two-file consumer runs successfully but its top-level type pass warns that `Greeting` is not found; the expanded definition/consumer checks cleanly | Package-aware static symbol discovery is a real implementation gap, not already complete |
| Exports | [module.rs](../../src/parser/stmt/module.rs) parses container/action/constant exports; runtime validates existence/kind. It does not selectively hide other included definitions | Package export enforcement is new work |
| Paths and aliases | File paths are expressions, resolved against the source file; `load module ... as ...` is explicitly rejected | Package binding is separately proposed syntax |
| Execution limits | Module loading uses the shared execution budget and circular-load guard | Package import must share limits and cancellation, not reset them |
| Removal | [removal record](../../History/dev-diary/2026/2026-07-26-remove-wflpkg-system.md), [archive README](../../Archive/retired-systems/wflpkg/README.md), and [package_protocol_removed_test.rs](../../tests/package_protocol_removed_test.rs) agree that package commands and special `package:` resolution were removed | No legacy wire format, CLI, default registry, or protocol is presumed implemented |

The principal runtime evidence is the named statement branches in
[interpreter/mod.rs](../../src/interpreter/mod.rs), including
`create_container_instance_with_inheritance`, with scope behavior in
[environment.rs](../../src/interpreter/environment.rs). Maintained
[container documentation](../../Docs/04-advanced-features/containers-oop.md) and
[module documentation](../../Docs/04-advanced-features/modules.md) were checked
against those branches and their tests. Existing annotations are not uniformly
runtime guards; for example `FunctionValue` explicitly preserves dynamic behavior
for ordinary single actions. “Strict type-safe packages” below is proposed work.

### 2.3 Prior-design reconciliation

Archived records remain intact. “Accepted” inside an archive is a historical
status, not authorization to restore the removed implementation. The following
source links are the prior art reviewed for this proposal.

| Source | Original decision | Disposition | Rationale and replacement |
|---|---|---|---|
| [Master design](../../Archive/retired-systems/wflpkg/wflpkgdesign.md) §§3–4 | Flat unquoted `project.wfl`, CalVer packages, English constraints | **Replace/adapt** | Use WFL literal-map records; strings are quoted to make names, paths, and versions unambiguous. Library SemVer is separate from WFL's unchanged CalVer |
| Master design §§5–6, 13 | Flat commands, optional separate binary, revived `package:` paths | **Replace** | Group commands under `wfl package`; use an explicit container-binding statement; preserve ordinary path semantics |
| Master design §§7, 14 | Lock checksums, one version per package; build may regenerate locks, yet updates are described as update-only | **Retain/resolve** | One source-qualified identity/version per graph; build never changes locks; only explicit resolution mutations do |
| Master design §§8–9 | WFLHASH, no install scripts, install-time permission prompts | **Retain/replace** | Retain no scripts; use SHA-256 exact archive bytes. Declarations are informational, not a sandbox or permission grant |
| Master design §§11–12 | Workspace file, local `packages/` tree, workspace-first resolution | **Defer/replace** | No implicit local override; explicit local dependency sources and `target/` materialization; workspace orchestration later |
| [Frozen grammar](../../Archive/retired-systems/wflpkg/wflpkg-manifest-grammar-1.0.md) §§3–6 | Flat sequence of literal `create map` blocks, strict lexical rejection, version envelope | **Adapt** | Retain the data-only grammar and shared first-party implementation; give container selection a schema; use unique block bindings so examples are also valid ordinary WFL |
| Frozen grammar §§7, 10 and Appendix A | Canonical form; also claims all accepted bytes are injective despite accepting extra whitespace | **Replace contradiction** | Author syntax may have whitespace/comments/CRLF; canonical artifact syntax has one encoding. Parsing is many-to-one; canonical round-trip is deterministic, not injective over author bytes |
| Frozen grammar §§7–8 | SHA-256, ASCII identities, NFC/Unicode pins, no comments | **Adapt** | ASCII identity/path alphabet retained. Free text preserved as Unicode scalars without normalization; comments allowed in author input, excluded from canonical metadata; exact archive hash covers distributed bytes. No unverified Unicode-table pin is copied |
| [Resolved decisions](../../Archive/retired-systems/wflpkg/wflpkg-open-decisions-resolved.md), Decision 1 | Native static literal format and deterministic JSON projection | **Retain** | Metadata is inspectable without interpreter evaluation; JSON is an export, never a competing editable source |
| Resolved decisions, Decision 2 | Name identity, but normalized language-AST hash is the v1 lock trust anchor | **Replace/defer** | AST hash omits resources and can omit source distinctions. SHA-256 of the entire artifact is mandatory; structural/API hashes may be advisory later |
| [Brainstorm](../../Archive/retired-systems/wflpkg/wflpkg-brainstorm-results.md), convergence | SemVer, scopes, MVS, major-in-import-path | **Adapt/replace** | Retain SemVer and scope. Source is also part of identity. Bounded English ranges require a deterministic constraint solver, not MVS; no major suffix or nested duplicate versions in v1 |
| Brainstorm, security recommendations | Sigstore/OIDC, transparency/sumdb, capability sandbox, anti-squatting | **Stage/defer** | Retain immutable source binding and exact verification. A public trust profile is a separate launch gate; do not claim a registry-independent trust guarantee from a registry-supplied hash. Full runtime sandbox and behavioral detection are separate work |
| [ADR-001](../../Archive/retired-systems/wflpkg/wflpkg-adr-001-binary-and-crate-structure.md) | One main binary, library core, thin alias, shared lexer extracted to `wfl_core` | **Retain/adapt/defer** | One user-facing binary and shared implementation retained; grouped CLI avoids reviving twelve verbs. Crate topology follows today's dependency graph; no currently existing `wflpkg`/`wfl_core` assumed. Alias binary deferred |
| [WFLHub PRD](../../Archive/retired-systems/wflpkg/wflpkg_prd.md) | tar.gz, flat names, CalVer, SHA-256 labeled `wflhash:`, hardcoded registry and website | **Replace/defer** | Canonical uncompressed ustar, explicit source/scope, honest algorithm tag, no default trust root. Define registry capabilities; defer portal, accounts UI, hosting, and metrics |
| WFLHub PRD, upload/yank requirements | Immutable versions, bounded archives, yank keeps existing locks usable | **Retain/adapt** | Core interface requirements retained with client-side containment checks and explicit revoked-vs-yanked distinction |
| [WFLHub language gaps](../../Archive/retired-systems/wflpkg/wflhub_language_gaps_prd.md) and archive README | Language work motivated by a WFL registry | **Defer as package dependency** | General language capabilities can ship independently; this PRD does not commission a registry implementation |
| Removal record and removal tests | No package protocol/subcommands; file modules intact | **Retain** | New proposal must be additive and must not erase the evidence of removal |

Historical ecosystem/security incident assertions in the brainstorm are not
treated as verified current facts. The design rests on repository evidence and
the precise standard references cited where used below.

## 3. Foundation traceability

The No-Unlearning Invariant takes precedence: authors learn one container
definition, one instance syntax, one literal metadata form, and one dependency
binding from their first package through a production-sized library. They add
records and files; they do not replace forms. Packaging an arbitrary side-effect
script is deliberately not the beginner lesson. The simple reusable definition
in §9 already meets the production declaration profile.

| # | Principle | Practical effect | Requirement |
|---|---|---|---|
| 1 | Natural-Language Syntax | `create map`, `is`, English version constraints, `include container from package` | PKG-002, 005, 008, 014 |
| 2 | Minimize Special Characters | Reuse existing colon/map/list syntax; quote text for unambiguous parsing; no `@` scope prefix or constraint operators | PKG-008, 014 |
| 3 | Readability & Clarity | Explicit root, source, aliases, dependency edges, and package inspection | PKG-001, 009, 016 |
| 4 | Actionable Error Reporting | Stable codes, source spans, cause chains, and safe repair commands | PKG-024 |
| 5 | Type Safety & Compatibility | Closed public contracts, inferred or declared concrete types, package boundary guards | PKG-004, 018 |
| 6 | Modern Features | Async/pattern code remains ordinary WFL inside methods; shared execution budget | PKG-006, 021 |
| 7 | Web Standards | Byte-preserved HTML/CSS/JS resources, explicit MIME and serving responsibility | PKG-012 |
| 8 | Built-in Security | No hooks, verified bytes, containment, no ambient source fallback | PKG-019–023 |
| 9 | Beginner Accessibility | Create from an existing container; safe defaults; no mandatory custom interface | PKG-002, 017 |
| 10 | Expert Expressiveness | Supporting modules, composition, explicit contracts, lock inspection, automation output | PKG-003, 016, 017 |
| 11 | Simplicity & Power | One root definition at every scale; application metadata avoids artificial root wrappers | PKG-001–003 |
| 12 | Community & Collaboration | Required documentation/license information, reviewable releases and origin | PKG-009, 023 |
| 13 | Performance Optimization | Cache immutable verified artifacts/definition graphs; never silently resolve during run | PKG-006, 016 |
| 14 | Standard Libraries | Existing text/list/web/testing facilities; built-ins identified by WFL version, not downloaded as shadow packages | PKG-007, 014 |
| 15 | Scalability & Maintainability | Private support graph, exact locks, compatibility comparison and phased components | PKG-003, 015, 018, 025 |
| 16 | Gradual Learning | Same file and map forms grow by adding records; no beginner singleton later replaced by factories | PKG-002, 017 |
| 17 | Error Transparency | Explain why a version/source was chosen; surface declaration limits and trust limitations | PKG-016, 020, 024 |
| 18 | Best Practices | Release contracts, negative tests, no manufactured green, portable artifact checks | PKG-018, 025–026 |
| 19 | Avoid Unnecessary Conventions | SemVer serves library compatibility; tar serves inspectability; JSON serves tooling; neither dictates WFL author syntax | PKG-008, 011, 014 |

## 4. Container-based package model

### 4.1 Definition, composition, and public boundary

**PKG-003 — Package structure.** The manifest selects `entry` (a source path)
and `container` (a definition name). That file MUST define the named container
at top level and contain exactly one `export container <name>` for it. v1
package source files MUST NOT contain any other export statement. Supporting
containers, interfaces, constants, and helper actions remain internal, even
though ordinary file includes would expose them.

The entry contract is the **definition**, including its member signatures and
construction contract. Import binds a reference to that definition; it does not
create an instance. No dedicated package interface or required inheritance is
introduced. An interface alone cannot describe state, static members, or
construction. A mandatory base container would consume the inheritance choice
and make existing definitions harder to package. A singleton would entangle
unrelated consumers. Authors compose supporting containers inside root actions
and may use ordinary inheritance/interfaces within the package.

All root members are public in v1, including inherited members. There is no
second member whitelist that can drift from code, and no invented `private`
keyword. An internal helper belongs on a supporting container/action, not as an
undocumented root member. The package boundary hides supporting definitions;
it does not pretend that today's property visibility implementation is private.
Cross-package subclassing and direct import of supporting definitions are
rejected in v1. Composition through dependency root instances is supported.

This is language-level API encapsulation, not secrecy of distributed source.
A person can read/copy source files, and existing file includes are not an OS
sandbox. Explicitly loading extracted source as an ordinary file creates ordinary
file-scope definitions, not access to a registered package's private environment
or shared state; it is outside the supported package API/identity contract.

**PKG-004 — Public type contract.** Validation MUST statically derive a closed
root signature: member names/kinds, static-vs-instance, ordered parameters and
types, return types, event parameter types, property read/write types,
inherited effective members, and direct `initialize` signature. Duplicate member
names, ambiguous overrides, and unresolved names/types are errors.

Public v1 value types are `Text`, `Number`, `Boolean`, `Nothing`, `Date`, `Time`,
`Pattern`, recursively typed lists, and the package's own root instance. The
package type checker MUST support these structurally even where existing parser
positions need completion. `Any`, untyped maps, functions, futures, opaque I/O
handles, supporting instances, and dependency instances cannot cross the public
boundary in v1. They may be used internally. This deliberately small public
type set avoids private types leaking through public values; widening it later
needs a format/type-profile revision. Missing annotations MAY be inferred only
when the checker proves one concrete type across all paths; no silent widening
to `Any`. The source definition is authoritative, not generated metadata.

Instance properties without defaults are required construction inputs unless
the direct `initialize` contract is proven to initialize them on every successful
path. Static properties require scalar defaults. Validation rejects a potentially
uninitialized public property rather than supplying an implicit `Nothing` value.
Parameter order/names and the ability to set each public property on construction
are part of the contract. Public events use the same closed parameter types.

Package checks are fatal for package declarations/bodies and package boundary
calls. Runtime guards validate arguments, property writes, and returned values
that cross from dynamically checked legacy code before dispatch/return. A guard
failure identifies package/member, expected type, and actual type; it never
coerces. Existing non-package programs keep their diagnostic and runtime rules.

### 4.2 Proposed import, identity, and naming

**PKG-005 — Explicit container binding.** Proposed syntax:

```text
include container from package "greeting" as Greeting
```

Grammar: `include container from package STRING as IDENTIFIER` followed by the
normal statement boundary. The string MUST be a literal dependency alias in the
importing package/application manifest, or `self` for that root's tests. Alias
`self` is reserved. `package` is contextual only after this exact prefix; the
existing `include from EXPRESSION` grammar is unchanged. Arbitrary expressions,
subpaths, and transitive aliases are rejected. Binding is usable in
`create new Greeting`, static member access, and annotations as an alias of the
same root type. The local type name is mandatory, preventing inferred-name
collisions; it need not equal the author's root name.

For `wfl <program>`, select the application/package context once: search from
the top-level program's directory upward for the nearest `package.wfl`, stopping
at the filesystem root. A new explicit runtime `--project DIRECTORY` override
may select that context instead. Missing metadata is an error only when a
package operation is requested. Subsequent ordinary file includes/loads inherit
this selected context; they do not discover another manifest mid-run. Registered
package code resolves dependency aliases in its own manifest, never the caller's.
The interpreter receives the selected lock graph before the first package
binding. `wfl package test` supplies its project context explicitly. This runtime
lookup is distinct from the CLI tools' current-directory default in §8.

The runtime nominal type identity is `(source, scope, name, version, digest,
root-definition)`; local spelling does not alter it. Internal symbols are keyed
by package identity plus source path/name, not a global bare-name map.
Two distinct dependencies may both define `Greeting` internally without
colliding. Rebinding the same alias to the same definition in a visible scope
is idempotent; binding it to anything else fails without overwriting. Several
local aliases of the same definition share its static state.

This new statement is intentional: overloading `include from` with archive
semantics would obscure its current scope behavior. The `package:` protocol is
**not revived**. `include from "package:demo"` and `load module from
"package:demo"` remain ordinary path operations, including their existing
failure behavior. `load module from` cannot expose a package contract.

### 4.3 Initialization, state, and lifecycle

**PKG-006 — Inert registration, explicit execution.** Package source has a
declaration profile. At top level it permits container/interface definitions,
helper action definitions, scalar-literal constants, static literal-path
includes of declared source files, proposed package bindings, and the root
export. It forbids `load module`, executable expressions, ordinary variables,
top-level instance creation, I/O, and action calls. Every property default,
including static defaults, MUST be a scalar WFL literal matching its declared
or inferred type. Mutable lists/maps and computed defaults are rejected; create
per-instance collections explicitly in an action or supply them on construction.
This avoids changing the current shallow cloning of mutable defaults.

Registration parses and checks the full declaration graph, allocates internal
definition environments, and installs literal values; it does not interpret
top-level statements. Registration MUST be atomic: either the complete public
binding appears or no binding/static state survives. Internal includes follow
source-relative paths and register declarations into the owning package's
environment, which contains declared dependencies and WFL built-ins, never
consumer variables. Keep that environment alive while definitions, instances,
or callbacks reference it; current weak method environments alone are insufficient.

Construction and method calls use ordinary WFL behavior. `create new` creates a
new instance; explicit property initializers execute in the consumer context.
Constructor arguments invoke the direct `initialize` action under the current
rules; no arguments mean no automatic `initialize`. An author wanting explicit
startup/cleanup exposes ordinary actions, with their behavior documented. Imports
do not open sockets, run migrations, start tasks, or call shutdown actions.

Each independent top-level interpreter execution owns one package-definition
graph. Repeated imports and multiple consumers in that execution reuse definitions
and static state, but separately created root instances have separate scalar
properties. Lists/objects deliberately passed between consumers retain ordinary
WFL reference-sharing behavior. Independent runs share only immutable cached
bytes/analysis, never mutable static state. No process-global package singleton.

Statics are a deliberate shared-state contract: mutations are visible to all
references in that execution and are not rolled back on ordinary action errors,
matching existing container behavior. Async code is subject to the same WFL
scheduling and execution budget; a package adds no implicit synchronization.
The package system owns no unload hooks, background worker, or finalizer.
Explicitly started resources remain the calling program's responsibility;
cancellation/shutdown uses existing runtime mechanisms. Atomicity of registration
does not promise rollback of later user code or constructor side effects.

Dependency cycles and source-include cycles are rejected before registration,
with the complete cycle path. Diamond graphs share one definition graph. In
concurrent handlers, registration must serialize per identity or share an
in-progress result, distinguish recursion from an independent waiter, and clear
failed/cancelled state. No partially initialized definition can escape.

### 4.4 Compatibility with file modules

**PKG-007 — Preserve existing behavior.** Unpackaged `include from` and
`load module from` retain their current scope, repeat, path, side-effect, error,
and export behavior. Existing examples continue to work without a manifest.
Inside a package, declaration includes are statically registered; dynamic file
loads/includes inside actions are rejected by the v1 profile because they could
bypass the declared source graph. This is a validation restriction on opt-in
package code, not a change to the general language.

Moving a declaration-only container into a package adds a manifest and root
export. Consumers replace a file include with the proposed binding; their
instance creation and method calls remain the same. A side-effect-heavy script
may need deliberate refactoring into actions before becoming a library; the
creation command diagnoses this and does not automatically move code or change
initialization semantics.

## 5. Package metadata and format standard

### 5.1 Files and source membership

**PKG-008 — Static metadata.** The source manifest is `package.wfl`. A consuming
application uses the same filename/grammar with an `application` record instead
of a `package` record. Both use a `format` envelope first. `package.lock` is the
lock filename. `.wflpkg` is the artifact extension. The new envelope/schema is
not the removed `project.wfl` format and MUST NOT be interpreted as it.

The recommended root source is `src/main.wfl`, but `entry` is always explicit;
there is no hidden filename fallback. `sources` explicitly lists every code file
in the package declaration graph. The default creation tool generates the list
from static includes after review; consumers never scan arbitrary neighboring
files. Every listed source must be reachable from `entry`, and every referenced
source must be listed. Case-exact path spelling is required on all platforms.

Required archive files: canonical `package.wfl`, every `sources` member, the
README file, and at least one license file. Optional files: explicitly listed
documentation and declared resources. Tests, `package.lock`, application code,
VCS state, credentials, `.wflcfg`, output directories, and dependency archives
are excluded. The manifest may name author-local test files that are not shipped;
consumers must not look for or execute them. There are no glob patterns or
implicit file inclusion rules.

Author-tree validation checks that listed tests exist; archive/dependency
validation checks their path syntax only. Documentation must explain the root's
construction, state, errors and examples; a license label alone is not a
substitute for the supplied license text.

### 5.2 Literal grammar

The following is the **proposed author grammar**, not a call to the interpreter.
It reuses existing WFL map syntax while constraining its values. Whitespace is
not indentation-significant. Keywords are lowercase and case-sensitive in this
format. Horizontal whitespace is spaces or tabs; logical newlines are LF or
CRLF, never bare CR. Comments start with `//` or `#` outside strings and continue
to newline. Blank/comment-only lines may appear between records/entries.

```text
document = envelope, one-or-more record
envelope = "create map format:", newline,
           entry("grammar", "wfl-package-1"), newline,
           "end map", newline
record   = "create map", space, binding, ":", newline,
           one-or-more entry-line, "end map", newline
entry-line = key, space, "is", space, value, newline
key      = identifier | quoted-ascii-key
binding  = lowercase non-reserved WFL identifier
value    = string | integer | "yes" | "no" | list
list     = "[", [scalar, {",", optional-space, scalar}], "]"
scalar   = string | integer | "yes" | "no"
integer  = "0" | nonzero-digit, {digit}
```

Spaces may expand to one or more horizontal whitespace characters; indentation
is allowed before entries and record delimiters. A final newline is required.
Trailing horizontal whitespace and comments are allowed after complete lines.
List elements must be on one logical line, use commas, and have no trailing
comma. No nested lists, maps in values, floats, negative numbers, nulls,
references, interpolation, expressions, calls, imports, or computed keys.
Integers range from 0 through 9007199254740991 with no leading zeros.

Strings are double-quoted UTF-8 Unicode scalar sequences, with only `\n`, `\t`,
`\\`, and `\"` escapes. Reject BOM anywhere, invalid UTF-8, raw controls
U+0000–001F/U+007F–009F in strings, and bidi/invisible characters U+200B–200F,
U+202A–202E, U+2060, U+2066–2069. Escaped newline/tab are permitted only in free
text, never identities, paths, or constraints. Strings are preserved without
Unicode normalization; comparison of free text is codepoint-exact. Identity
and path fields are ASCII only, so normalization cannot create an identity
alias. Metadata is data even when prose addresses an assistant; tools MUST NOT
treat descriptions, notes, documentation, or comments as instructions.

Record bindings are unique and designate schema roles: `format`, `package` or
`application`, then zero or more `dependency_<alias>`, and `resource_<alias>`.
Aliases use `[a-z][a-z0-9_]*`, at most 48 characters; `self` is reserved. The
record suffix supplies the alias; there is no conflicting alias field. Quoting
keys handles WFL keyword collisions. Canonical output quotes **all** keys.
For example, `dependency_greeting` remains an ordinary WFL binding even when a
second dependency is added; repeated executable variable names are not taught.

This is one shared, version-selected first-party parser/formatter library for
CLI, runtime, LSP, and registry. It may use the existing lexer with source spans
and a restricted acceptance layer, but MUST check skipped gaps and bound numeric
tokens before callbacks. It MUST NOT execute a general AST or let future lexer
changes silently expand this grammar. Independent implementations are permitted
and judged by the same conformance corpus.

### 5.3 Schema, defaults, and unknown fields

**PKG-009 — Manifest schema.** Field types below are exact: text means string,
paths means list of distinct path strings, and texts means list of distinct
strings. Order of `sources`, `tests`, `documentation`, `license_files`, and
`permissions` is semantically irrelevant; tool-generated canonical lists sort
them bytewise. `authors` order is preserved as author intent.

| Record/field | Required/default | Validation and meaning |
|---|---|---|
| `format.grammar` | Required | Exactly `wfl-package-1` |
| `package.scope`, `package.name` | Required text | Each `[a-z][a-z0-9]*(-[a-z0-9]+)*`, 1–64 ASCII bytes; no automatic case folding |
| `package.version` | Required text | Package SemVer profile in §6 |
| `package.description` | Required text | 1–4096 decoded UTF-8 bytes |
| `package.authors` | Required texts | At least one nonempty display name; no email required |
| `package.license` | Required text | Nonempty human-readable license identifier/expression; tooling displays it, never substitutes it for the supplied license text |
| `package.license_files` | Required paths | At least one file; byte-preserved license/notice material |
| `package.readme` | Required path | Nonempty UTF-8 Markdown; not rendered as trusted HTML |
| `package.entry` | Required path | `.wfl` file contained in `sources` |
| `package.container` | Required text | Exact case-sensitive non-reserved WFL identifier; resolves to the root definition |
| `package.sources` | Required paths | Nonempty reachable declaration graph, §5.1 |
| `package.wfl` | Required text | WFL CalVer runtime range, §6.1; creator records its actual tested runtime |
| `package.tests` | Optional, `[]` | Author-local `.wfl` test paths, run only on explicit `test` |
| `package.documentation` | Optional, `[]` | Additional archive documentation paths |
| `package.permissions` | Optional, `[]` | Closed declaration vocabulary, §7.3; never a grant |
| `package.repository` | Optional, absent | HTTPS source repository URL; informational, not a dependency locator |
| `package.notes` | Optional, absent | Free text, at most 4096 decoded bytes |
| `application.name` | Required text | Same grammar as package name; local display identity, no publication scope/version |
| `application.wfl` | Required text | Same runtime range field as package |
| `application.tests` | Optional, `[]` | Explicit application test paths |
| `application.notes` | Optional, absent | Same free-text rule |
| `dependency_<alias>.scope`, `.name` | Required text | Target package identity components |
| `.version` | Required text | Exact or compatible/range constraint, §6.1 |
| `.registry` OR `.path` OR `.archive` | Exactly one required | Explicit HTTPS origin, relative local directory, or relative local `.wflpkg` path |
| `.development` | Optional, `no` | Dependency is used only in this author's testing/development graph |
| `resource_<alias>.path` | Required path | One regular file; no directory/glob implicit expansion |
| `.media_type` | Required text | Lowercase `type/subtype` ASCII, no parameters; explicitly supplied, never guessed on serving |
| `.public` | Optional, `no` | Whether consumer may request the resource through the package API |

No other records or fields are accepted in v1. Duplicate keys (including a
quoted/unquoted duplicate), bindings, paths within a list, or resource aliases
fail. `package` and `application` are mutually exclusive and singular.
Dependency aliases are unique even if development-only. Two aliases may refer
to the same target identity; they still select one version. A file may serve
more than one metadata role (for example README also in documentation); it is
stored once. Resource aliases referencing the same file must agree on MIME and
public status. A source cannot also be a resource. Unknown permissions, type
names, grammar versions, and algorithms fail closed, with spelling suggestions
that never automatically repair the input.

Limit each manifest/lock to 1 MiB, each decoded string to 64 KiB (narrower field
limits above take precedence), each record to 32 keys, each list to 4096 items,
and each manifest to 8192 records. Locks permit 8194 records to accommodate two
4096-node graphs plus envelope/resolution. Count before allocation; overflow is
an error. Each package may declare at most 4096 dependency aliases, matching the
per-node edge-list bound; total graph limits still apply.

### 5.4 Canonicalization and format evolution

**PKG-010 — Canonical bytes.** Author metadata may vary in whitespace, comments,
and line endings. `format` rewrites it explicitly; `build` emits canonical
metadata into its staging directory without changing the author's manifest.
Verification never repairs downloaded bytes. A distributed manifest and every
lock MUST already equal their canonical serialization or be rejected.

Canonical serialization is UTF-8 without BOM, LF, final LF, no comments,
`create map <binding>:` and `end map` unindented, entries indented four spaces,
one space around `is`, quoted keys, comma-space list separators, minimal
integers, and `yes`/`no`. Escape quote/backslash/newline/tab with their respective
escapes; preserve other allowed Unicode scalars. Exactly one blank line
separates records. Envelope first, main record second, dependency records sorted
by alias, then resources sorted by alias. Keys sort by ASCII bytes. Omitted
defaulted fields are materialized (`[]`, `no`); absent optional nondefaulted
fields remain absent. Application records have only their own defaults.

`inspect --json` projects to `{ "grammar": "wfl-package-1", "records": [...] }`,
where each record is `{ "binding": STRING, "fields": OBJECT }`, in canonical
record order, excluding the envelope already represented by `grammar`. Values
retain their types, with `yes/no` becoming booleans. Output uses
[RFC 8785 JCS](https://www.rfc-editor.org/rfc/rfc8785.html). The JSON is a derived
interchange view, never an alternate source manifest. Integer restrictions and
ASCII keys keep its numeric/key ordering rules straightforward.

`wfl-package-1` freezes grammar, schema, defaults, canonical bytes, archive
profile, and public type profile. Changes that alter acceptance, meaning, or
canonical output require a new envelope (`wfl-package-2`); editor enhancements
and clearer prose do not. Future extensions cannot silently change resolution
or security. Unsupported versions must fail before package execution and report
the supported versions. Old readers are not required to guess new fields.
Future readers must retain v1 readers while v1 is supported; migration is an
explicit author operation, never an install-time rewrite. No migration from
the retired formats is automatic; a converter may propose a reviewed new file.

### 5.5 Deterministic archive

**PKG-011 — Artifact bytes.** A v1 `.wflpkg` is **uncompressed POSIX ustar**,
with no enclosing directory. This deliberately replaces historical tar.gz:
ordinary archive inspection is useful, while omitting compression eliminates
compressor-version variability and decompression bombs. HTTP transfer compression
must not change the bytes whose artifact digest is checked. The ustar base
header is documented in the [GNU tar format reference](https://ftp.gnu.org/old-gnu/Manuals/tar-1.12/html_node/tar_123.html);
the restrictions below define this package profile completely.

Files appear in ascending ASCII path-byte order. Only typeflag ASCII `0`
(regular file) is permitted. No directory records, symlinks, hard links, devices,
sparse records, GNU/PAX extensions, or concatenated archives. Headers are 512
bytes: `magic = ustar\0`, `version = 00`, mode `0000644\0`, uid/gid
`0000000\0`, mtime `00000000000\0`, size is eleven zero-padded ASCII octal digits
plus NUL; uname/gname/linkname and unused bytes are zero. Device fields are
`0000000\0`. Checksum is the unsigned byte sum of the header with its checksum
field replaced by eight ASCII spaces, encoded as six zero-padded octal digits,
NUL, space. File content is followed by zero padding to the next 512-byte
boundary. Exactly two zero blocks terminate the archive, with no further bytes.

If a path fits in 100 bytes, store it entirely in `name` and leave `prefix`
zero. Otherwise split at the rightmost slash for which prefix is at most 155
bytes and name is 1–100 bytes; omit that slash from both fields. Zero-pad both
fields without adding a terminator if full. Reject paths with no such split.
Validate the logical reconstructed path before writing anything.

Package member paths are relative ASCII paths at most 240 bytes long, components
1–64 bytes using `[A-Za-z0-9][A-Za-z0-9._-]*`. Slash is the sole separator.
Reject empty components, `.`/`..`, trailing dots, backslashes, colons, rooted or
drive/UNC paths, and components whose basename before the first dot equals
`con`, `prn`, `aux`, `nul`, `com1`–`com9`, or `lpt1`–`lpt9`, case-insensitively.
Reject case-insensitive collisions and file/directory-prefix collisions across
the complete member set. Manifest local dependency locators are separately
governed by §6.4 and are not archive member paths.

Maximum artifact size is 64 MiB including headers/padding; maximum regular file
size 32 MiB; maximum files 4096. All consumers must support inputs through these
limits. Stricter deployment quotas must report a policy rejection rather than
call the format malformed. Reject truncated headers/bodies, bad checksums,
size arithmetic overflow, duplicate members, noncanonical header fields, and
extra/missing files relative to the manifest's exact membership union.

All `.wfl` source is well-formed UTF-8 without BOM, uses LF, and ends in LF in
the artifact. Build rejects CRLF/BOM source with a formatting suggestion rather
than silently changing source/string bytes. The creation workflow writes LF;
authors may use `.gitattributes` to preserve it. README/docs must be valid UTF-8;
license and resource files are byte-preserved. Build copies an immutable
snapshot of declared inputs into staging, detects concurrent source mutation,
and hashes/emits that snapshot. It includes no timestamps, host paths, executable
bits, or compiler version stamps. The same canonical manifest and file bytes
therefore produce the same archive bytes, independent of platform or file order.

Recommended output: `target/package/<scope>-<name>-<version>.wflpkg`. This name
is cosmetic; identity is validated from metadata and the expected source record.

### 5.6 Resources

**PKG-012 — Explicit resource access.** Resources are package-relative,
read-only byte data. Installing an HTML/JS resource never serves or executes it.
Proposed runtime statements are:

```text
read package resource "page" from "self" as page_bytes
read package resource "page" from "welcome" as page_bytes
```

Both names are literal aliases. `self` reads any own declared resource; a direct
dependency alias reads only resources with `public is yes`. Binding produces
existing WFL `Binary`, not a filesystem path or an implicitly decoded string.
This is a new statement, requiring parser/typechecker/runtime/LSP work; it is
not today's `read file` syntax. Type annotations on the package root need not
expose Binary for internal resource use. Lookup checks identity, canonical
membership and bounds, not the process working directory. No traversal or
undeclared fallback. A binary result follows existing binary-value ownership.

Serving is an explicit application action using normal web APIs. Package metadata
does not grant a route, content-security policy, HTML escaping, or JavaScript
sandbox. MIME is returned by inspection/resource metadata; consumers must use
the declared type deliberately and treat HTML/JS as active content when served.

## 6. Dependencies, compatibility, and local state

### 6.1 Identity and versions

**PKG-013 — Source-qualified identity.** Registry package identity is
`(registry origin, scope, name)`. Version identifies one immutable artifact at
that identity. A registry origin is canonical `https://lowercase-ascii-host`
with an optional non-default decimal port, no path/query/fragment/userinfo or
trailing slash. No implicit default registry, mirror fallback, or “highest
version anywhere” search. Registry redirects may not change source identity or
send credentials cross-origin. An explicitly configured mirror is transport
only and must return the expected digest. Scope is an ownership namespace;
`scope/name` is readable without an `@` prefix. ASCII restrictions reduce
confusable identities but do not eliminate typosquatting.

**PKG-014 — Version profiles.** Libraries use
[Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html), with v1 package
restrictions: exactly three unsigned components, each at most 2147483647;
no leading zeros; optional valid SemVer prerelease; no `+build` suffix; at most
128 ASCII bytes total. Normal
SemVer precedence applies. This gives library compatibility a deliberate signal;
dates do not express substitutability. The WFL product remains `YY.MM.BUILD`
under governance, and the format remains `wfl-package-1`. Neither inherits a
package version or changes when a package releases. The Windows MSI year limit
does not constrain third-party library majors.

Permitted dependency constraint strings (single ASCII spaces, full versions):

| Form | Exact meaning |
|---|---|
| `1.2.3 exactly` | Only that version; the only form that may select prereleases |
| `compatible with 1.2.3` | At least 1.2.3, below 2.0.0, stable versions only |
| `compatible with 0.2.3` | At least 0.2.3, below 0.3.0, stable only |
| `compatible with 0.0.3` | Exactly 0.0.3 |
| `at least 1.2.3 but below 1.8.0` | Inclusive lower and exclusive upper bound, stable only, lower less than upper |

No wildcards, partial versions, `any version`, disjunctions, caret/tilde syntax,
or unbounded “or newer.” v1 favors ranges whose meaning can be stated in words.
The creator defaults to `1.0.0` for a new public contract; explicitly experimental
authors may choose `0.x`. `add` selects a concrete stable version and writes
`compatible with <selected>` unless the user supplies an exact/range constraint.

`wfl` fields use the same `V exactly` or `at least V but below V` words over
numeric CalVer triples, with year 0–255, month 1–12, nonnegative build, no
prerelease. There is no `compatible with` for WFL CalVer. Validation intersects
all selected runtime requirements with the actual runtime version. A new
package-capable WFL release must be named in generated metadata; `26.9.18`
must not be claimed to support the proposed import statements.

### 6.2 Resolution

**PKG-015 — Deterministic graph.** Resolve one version per source-qualified
identity. Every package's runtime dependencies are followed transitively.
Only the root project's development dependencies join its development/test
graph; development dependencies of dependencies are not followed. Runtime
source cannot import a development alias. Cycles in either selected graph
fail with the edge path. Separate runtime and development graphs may select
different versions and have separately labeled lock nodes; a test run uses the
development graph consistently, never a mixture of the two.

The normative selection rule is deterministic depth-first constraint search:
start with root edges; choose the lexicographically smallest unresolved identity
(source-kind, source, scope, name byte order); try satisfying candidates in descending version
precedence; add their dependency constraints; reject an inconsistent assignment
and backtrack in reverse choice order. Recheck already selected nodes whenever
a new edge constrains them. First complete valid assignment wins. Candidate
lists/metadata are immutable inputs to a solve, not a moving registry view.
Equivalent algorithms are allowed only if they produce this specified result.
This is not MVS: exact pins and upper bounds need conflict search. Implementation
library selection is a later engineering choice, not a historical dependency bet.

Existing locks remain authoritative during install/build/run. An `add` or
`remove` preserves every existing selection still reachable and valid, failing
with an explanation if that prevents a solution. `update <alias>` unlocks its
reachable runtime dependency closure; `update` unlocks all selected identities.
Outside that closure pins stay fixed. New dependencies use the rule above.
Updates show exact version, digest, source, contract and permission changes;
they never widen a manifest constraint without an explicit user edit/command.

Solver bounds: 4096 selected nodes per graph, 65536 edges, 128 dependency depth,
1024 candidate versions per identity snapshot, and 100000 attempted assignments.
Count attempts each time a candidate is assigned; stop with `PKG-LIMIT` on
overflow. Exceeding a budget is not “no compatible versions.” Report the limit
and allow the author to narrow constraints. Determinism is for identical input
snapshots; fresh registry snapshots may legitimately choose newer versions.

### 6.3 Lockfile and caching

**PKG-016 — Exact local state.** A `package.lock` is required even for zero
dependencies. It uses §5.2 literals and canonical formatting, with envelope
`grammar is "wfl-package-lock-1"`. It starts with a `resolution` record, then
`locked_0001`, `locked_0002`, etc. A lock parser accepts only this schema:

| Record | Fields |
|---|---|
| `resolution` | `manifest_digest`: tagged SHA-256 of canonical root manifest bytes; `runtime_roots`, `development_roots`: sorted text lists of `alias=node-id` edges; `resolver`: exactly `descending-1` |
| `locked_NNNN` | `graph`: `runtime` or `development`; `source_kind`: `registry`, `path`, or `archive`; `source`: canonical origin/relative locator; `scope`, `name`, `version`; `digest`: exact artifact SHA-256; `dependencies`: sorted `alias=node-id` texts |

All fields are required; no timestamps or machine paths. Number nodes from
0001 in order `(graph, source_kind, source, scope, name)`, ASCII byte order;
graph groups share one numbering sequence. Roots/edges use record bindings as
IDs, e.g. `greeting=locked_0001`. Canonical key order is ASCII as in §5.4;
record order is envelope, resolution, numbered nodes. Identical packages used
in both graphs have two labeled records but can reuse one cached artifact.
Development graph roots include runtime root edges as well as development-only
edges. Zero-dependency lock has only envelope/resolution and two empty lists.

Digest grammar is exactly `sha256:` followed by 64 lowercase hex digits. The
root package itself is not a locked dependency; its output digest is reported
by build/share separately, avoiding a circular self-hash. A lock MUST match
the root manifest digest, declared source identities/constraints, all selected
nodes' manifests and dependency edges, graph reachability, and runtime range.
Reject orphan nodes, unknown edges, duplicate identities in a graph, cycles,
noncontiguous IDs, and disagreement between node identity and artifact metadata.
Dependencies' own lockfiles never control a consuming application's resolution.

`build` with no lock fails with “Run `wfl package resolve`”; build never contacts
the network. `resolve` creates a missing lock or validates an existing lock;
it does not silently upgrade an existing graph. `install` materializes the
locked graph, fetching missing verified bytes if network use is allowed; it
cannot rewrite the lock. `--locked` on mutating resolution operations forbids
any lock change; `--offline` forbids even attempted network requests.

Use an OS-appropriate user cache keyed by SHA-256 of artifact bytes. Cache
entries are immutable; verify before use and quarantine corruption. Project
materialization and staging live under `target/packages/` and `target/package/`,
not a tracked root `packages/`. Copies are the required portability baseline;
hardlinks, junctions, symlinks, reflinks, and administrator privileges are not
required. Do not execute from a writable partial extraction. Rehash files or
the canonical archive before trusting a materialized tree; filesystem read-only
attributes are not an integrity proof. Offline cached metadata permits resolve
only from an explicit complete recorded candidate snapshot; otherwise request
an online resolve rather than pretend the cache is exhaustive.

Manifest+lock edits are transactional: stage both, validate both, write a
recovery journal under `target/packages/`, then replace. Readers detect the
journal and recover/finish or refuse mismatched pairs. Cancellation/crash must
leave the old usable graph or a recoverable incomplete transaction, never a
silently mixed pair. Content-addressed cache writes use unique staging and
atomic promotion; concurrent installers coalesce verified identical content.

### 6.4 Local sources, conflicts, updates, and compatibility

Local `.path` and `.archive` locators are relative to the declaring manifest
directory, with `/`, no drive/UNC/absolute path or backslash. `..` is allowed
only for local author dependencies; resolve it explicitly and show the target.
Lock `source` stores the normalized project-relative locator, never a personal
absolute path. Path package artifacts are built inertly with the same canonical
builder, then pinned by artifact digest. On a locked run/build, a changed local
tree fails; explicit `update` records new bytes even if its version is unchanged.
There is no editable live bypass of the lock. Nested local locators normalize
relative to the consuming root for lock identity. Local symlinks/reparse points
are rejected while snapshotting, and recursion back to an active package path
is a cycle. Cross-volume local locators cannot be represented portably and fail.

Registry-published package manifests MUST contain only registry dependency
sources (including development declarations); local-only artifacts may contain
relative local sources but cannot be shared to a registry. A local archive may
depend only on registry sources: it has no durable neighboring source tree.
Local directory dependencies with nested local sources remain author workflows.
No environment variable or local name match can replace a registry dependency.

**PKG-017 — Explicit workflows.** Tools MUST support §8's author/consumer flows
with the same forms for beginners and automation. Adding/removing/updating must
explain direct/transitive effects. Removing an alias does not remove a package
still needed transitively and never rewrites source calls. Report remaining
references with source locations before applying removal; fail unless the
caller explicitly requests removal despite those references. Cache garbage
collection is separate and cannot delete a referenced or in-use object.

**PKG-018 — Compatibility.** A package's public contract includes behavior,
not just its type signatures. Removal, type changes, constructor changes,
changed property mutability, removed resources/public status, or strengthened
preconditions are breaking changes. Within a stable major, additions must
remain compatible; a new required constructor/property input is breaking.
Runtime minimum increases and permission increases are surfaced as compatibility
risks requiring an explicit release decision. v1's compatibility checker compares
derived source signatures/resource metadata against the prior immutable release
and errors on structurally detectable breaks without an appropriate version
change. It cannot prove behavioral compatibility; author tests, changelog, and
review remain necessary. A major bump is not authority to break WFL's own
governed compatibility commitments; WFL-owned packages must also follow the
repository's announced deprecation/migration policy.

Conflicts name both dependency paths, constraints, source-qualified identity,
and pins preventing progress. Never solve by silently taking two versions,
changing registries, or discarding an upper bound. Yanked versions are excluded
from new solves but remain usable with existing exact locks and an explicit
notice. A separately authenticated security revocation fails closed on online
verification; offline operation reports the age of locally known trust status
and cannot claim current revocation knowledge.

## 7. Security and trust

### 7.1 Threat model and integrity

Assets are source/resource bytes, dependency identity, local files, credentials,
and the user's execution environment. Attackers may control an archive, registry
response, dependency metadata, README, cache file, or package code. Boundaries
are metadata parsing, resolution, network fetch, extraction, publication, and
eventual execution. A compromised publisher may distribute malicious code with
a perfectly valid digest.

**PKG-019 — Exact-byte integrity and containment.** SHA-256 of the **entire
canonical artifact**, including metadata, source, license, documentation, and
web resources, is the installation/lock integrity anchor. No `wflhash:` prefix
for another algorithm. WFLHASH and language-AST hashes are not verification
substitutes. An AST hash cannot protect a CSS/JS file, README, license, or omitted
source spelling; future API/structure comparisons are advisory only. Metadata
digests in locks bind the root description but do not replace artifact digests.

The installer verifies expected digest and size before extraction, checks the
complete archive profile/member set, and stages into a newly created private
directory. It uses containment-aware file creation, refuses symlinks and Windows
reparse points in every destination component, never follows archive links,
and never overwrites arbitrary existing paths. Validate both lexical paths and
actual filesystem containment to resist races. Promote only a completely
verified tree; on failure/cancellation remove staging, preserve prior install,
and release locks. No file permissions or executable bits come from the archive.
Repeated identical installs are idempotent.

Source snapshots, parsing, and archive reading have the fixed limits above and
the caller's cancellation/deadline budget. Runtime source/import limits may be
stricter than format size limits: the runtime must report which execution limit
prevents this application, rather than incorrectly declare the artifact invalid.

### 7.2 Which operations execute code

**PKG-020 — No implicit execution.** The following is an enforceable tooling
boundary, independent of whether the runtime has a sandbox:

| Operation | Package code execution | Other effects |
|---|---|---|
| Create, inspect, format, validate, compatibility check | None | Local metadata/source reads; explicit create/format writes |
| Resolve, add, update, remove | None | Explicit metadata fetch where allowed; transactional manifest/lock writes |
| Download/install/build/share | None | Verified fetch/extraction/archive creation or explicit upload |
| LSP hover/completion/diagnostics | None | Static reads only; no network fetch without explicit user command |
| Package import statement | No action/default-expression execution | Inert verified registration and binding |
| `wfl <program>`, explicit `create new`, calls, resource reads | Yes where WFL semantics require it | Ordinary execution under caller's runtime and OS authority |
| `wfl package test` | Yes, explicitly | Runs only root's listed tests with the development graph; never transitive tests/hooks |

Build means static checking plus canonical archiving, not executing a build
script or silently running tests. `share` uploads the already-built artifact;
it cannot run code supplied by the package, registry, or dependencies. Tests
are an explicit separate action and publication policy may require their
recorded results without executing them on install.

### 7.3 Permissions and runtime limits

**PKG-021 — Honest permission declarations.** Permitted declaration labels are
`file-read`, `file-write`, `network-client`, `network-listen`, `process`,
`environment`, and `database`. Empty means “author declares none,” not “verified
harmless.” Inspect/resolve show each package's declarations and the graph union,
including changes in an update. File scope, host allowlists, credentials and
database authority are not encoded by these broad labels.

v1 tooling enforces no-hooks and declared source/resource boundaries. The
ordinary WFL runtime/OS enforces its existing execution budgets and OS access
rules. **The metadata does not sandbox runtime code.** Installation approval
does not grant capabilities; declared `network-client` is not an enforceable
host restriction. Do not display “permission denied” as if a manifest claim
were an OS sandbox. Running package tests or a consuming program is the explicit
execution boundary; tools show the unsandboxed execution status, with a concise
first-run explanation and a noninteractive machine-readable equivalent.

A future deny-by-default runtime capability system requires propagation through
all stdlib/native boundaries, callbacks, async tasks, and transitive calls,
including confused-deputy defenses; it must be a separate R3 project with real
denial tests. No `--sandbox` flag may silently fall back to unsandboxed operation.

### 7.4 Registry interface and trust profile

**PKG-022 — Explicit trust.** HTTPS origin binding and a digest from that same
origin establish transport/source consistency, not independent publisher
authenticity. A lock pins bytes after selection but does not prove that first
selection was honest. Direct archive sharing should exchange the digest through
an independently trusted channel. Signature verification must bind source,
scope/name, version, digest, and artifact size, with named trust roots and key
rotation/revocation rules; never merely “a signature exists.”

**PKG-023 — Registry contract.** A registry adapter MUST provide the following
semantics; endpoint spelling/authentication transport are a separately versioned
registry protocol, not silently inherited from WFLHub:

| Capability | Required request/result behavior |
|---|---|
| Enumerate releases | Source + scope/name → immutable candidate snapshot ID and ordered exact versions with digest, size, manifest, yank/revocation status |
| Fetch | Source + scope/name/version/digest → exact artifact bytes or typed not-found/integrity failure; bounded streaming, never execute content |
| Share | Authenticated owner + exact artifact → immutable identity/digest receipt; same identity/version/digest retry is idempotent; different bytes for existing version rejected |
| Ownership | Verify scope ownership for publication/transfer; transfers cannot rewrite existing releases |
| Yank/revoke | Authenticated status record with reason/time; distinguish distribution preference from security invalidation; retain immutable release evidence |
| Inspect/search | Return structured metadata/provenance and exact origin; suggestions never silently select a similarly spelled package |
| Failure handling | Distinct unauthorized, forbidden, not-found, conflict, quota, unavailable, invalid-metadata and integrity errors; redact secrets |

Candidate manifests used in solving MUST equal the canonical manifest from the
artifact before committing a lock; disagreement aborts, rather than resolving
one graph and installing another. Requests to private sources cannot fall back
to public sources. Credentials belong in OS credential storage or an explicit
short-lived credential provider; never in the manifest, lock, archive, logs,
or source URL. Downloads need cancellation, bounded buffers/timeouts, cleanup,
and visible errors. Retrying an explicitly idempotent request must not replace
bytes or hide an integrity failure.

v1 local format/tooling and privately configured registry transport can be
developed without a public registry. **Public registry launch is gated** on a
Maintainer-approved trust profile covering publisher authentication, release
authorization, signed immutable metadata, rotation/revocation, transparency or
equivalent externally verifiable history, incident handling, and availability.
Recommendation: evaluate short-lived OIDC publication and a witnessed release
log against these requirements; no provider/domain/key is selected here.
The website, search ranking, dashboards, download counts, account UI, SBOM feed
operations, malware analysis, and cross-registry detection are separate scope.

## 8. Proposed user workflows and CLI

All commands in this section are **proposed**. The primary binary remains
`wfl`; package operations use `wfl package <verb>`. Existing `wfl init` remains
configuration/agent-file creation. Existing `wfl <file>` and `wfl --test <file>`
remain program/test execution. To preserve an existing input file literally
named `package`, existing-file dispatch wins; `wfl ./package` is always an
unambiguous file invocation. Tooling can run from another directory using
`--project PATH`. Test this collision explicitly before adopting dispatch.

| Workflow | Proposed command | Observable result/default |
|---|---|---|
| Create from container | `wfl package create --from greeting.wfl --container Greeting --scope demo --name greeting --version 1.0.0 --license MIT --license-file LICENSE` | Validate declaration profile; create missing metadata/README; add root export only as a shown explicit edit; never overwrite existing code/license. Missing fields prompt only on a TTY |
| Inspect | `wfl package inspect` or `wfl package inspect FILE.wflpkg` | Root contract, source, dependencies/resources, declared permissions and trust status; `--json` for tools |
| Format metadata | `wfl package format` | Canonical author metadata; `--check` verifies without writing |
| Validate | `wfl package validate` | Inert source/schema/type/profile validation; offline by default; unresolved dependencies are an error with install guidance |
| Resolve | `wfl package resolve` | Create a missing lock from explicit sources; validate existing lock without upgrading |
| Install | `wfl package install` | Fetch/materialize exact locked dependencies, no code execution |
| Build | `wfl package build` | Locked, offline validation and exact archive; output under `target/package/`; no tests/hooks |
| Test | `wfl package test` | Explicitly execute root's listed tests via WFL test mode against development graph; empty list reports “no tests,” not “tests passed” |
| Add | `wfl package add demo/greeting --registry https://packages.example.invalid --as greeting --version "compatible with 1.0.0"` | Explicit source; stage manifest edge, solve, install, atomically commit lock/manifest |
| Local add | `wfl package add --path ../greeting --as greeting` or `wfl package add --archive ../greeting/target/package/demo-greeting-1.0.0.wflpkg --as greeting` | Read identity from local metadata, show it, pin exact canonical bytes |
| Explain | `wfl package why greeting` | Direct/transitive reason chains, selected versions and sources; no code |
| Update | `wfl package update greeting` | Show/apply permitted closure changes; `--dry-run` produces a review without mutation |
| Remove | `wfl package remove greeting` | Check source references, remove direct alias and now-unreachable nodes transactionally; shared dependencies remain |
| Check compatibility | `wfl package compatibility --against FILE.wflpkg` | Static API/resource diff and release-version assessment; no behavioral proof claim |
| Share file | Copy `.wflpkg` and independently communicate its reported SHA-256 | Recipient can inspect/verify/add the same bytes offline |
| Share registry | `wfl package share FILE.wflpkg --registry https://packages.example.invalid` | Explicit upload of verified existing artifact after authentication; publication gates in §7.4 apply |

The example registry hostname is intentionally non-operational. No command here
claims it exists. Authentication/login transport must follow the adopted registry
profile; credentials are not positional CLI arguments.

Tools default to the manifest in the current directory, not an implicit parent
workspace search. `--project` selects a directory explicitly. `add` may create
an application manifest if missing, explaining this and writing its actual WFL
version range; it must not create an artificial container. `--development` marks
an author's test dependency. `--non-interactive` never prompts; missing required
inputs fail. `--json` writes one structured result to stdout, human diagnostics
to stderr, and never terminal control sequences. `--offline`, `--locked`, and
`--dry-run` have consistent meanings; unsupported combinations fail with usage.
Exit codes: 0 success, 2 usage, 3 validation/type/format, 4 resolution/conflict,
5 source/network/cache unavailable, 6 integrity/trust, 7 execution/test failure,
8 cancelled/resource limit. Diagnostic codes are more specific than exit codes.

**PKG-024 — Actionable diagnostics.** Errors include a stable code, operation,
file/member span where available, package source/identity, the observed problem,
and a concrete safe next action. Examples:

```text
PKG-ROOT: package.wfl selects Greeting in src/main.wfl, but that file defines Greeter.
Change "container" to "Greeter", or select the file that defines Greeting.

PKG-DECLARATION: src/main.wfl:4 calls load_settings in a property default.
Package declarations must be inspectable without running code.
Use a literal default and call load_settings explicitly in an action.

PKG-CONFLICT: demo/site -> greeting requires compatible with 1.0.0.
demo/legacy -> greeting requires 2.0.0 exactly, at the same registry origin.
No single version satisfies both. Update one dependent's constraint or remove it.
Run wfl package why greeting to see both paths. No files were changed.

PKG-OFFLINE-MISSING: demo/greeting 1.0.0 is locked but its sha256 object is absent.
Run wfl package install with access to its configured source, then retry offline.

PKG-INTEGRITY: downloaded bytes do not match the locked sha256 digest.
The download was quarantined. The previous installation and lock were preserved.
Check the source or restore the expected artifact; do not overwrite the lock to accept it.

PKG-TYPE: Greeting.greet expects Text for person_name; this call supplied Number.
Pass a text value. The action was not called.

PKG-FORMAT-VERSION: this reader supports wfl-package-1, but the artifact declares wfl-package-2.
Use a reader supporting that format; no package code was executed.
```

## 9. Complete minimal example: container to distribution to consumption

### 9.1 Current: a reusable container

This source is ordinary current WFL. The body remains unchanged when packaged.
Save as `greeting.wfl`:

```wfl
create container Greeting:
    property prefix: Text defaults "Hello"

    action greet needs person_name: Text: Text
        return prefix with ", " with person_name
    end
end

export container Greeting
```

Today's consumer, `current_consumer.wfl`, uses an ordinary file include:

```wfl
include from "greeting.wfl"

create new Greeting as greeter:
end

display greeter.greet("Ada")
```

Run today with `wfl current_consumer.wfl`; expected program output is
`Hello, Ada`. The definition/consumer are validated as described in §14. An
export line is harmless today but does not hide other included definitions.

On the inspected baseline, the two-file consumer also emits non-fatal type
warnings on stderr saying that `Greeting` is not found during the top-level
static pass. It still exits 0 and prints the expected line. Checking the
definition and consumer together has no type errors. This known gap is why
package-aware static discovery appears explicitly in §11; the example is not
claimed to be warning-free today.

### 9.2 Proposed: add the static package description

The creation command in §8 produces this author tree; README and LICENSE
contain real documentation and the author's actual license text:

```text
greeting/
  greeting.wfl
  package.wfl
  package.lock                 generated by resolve, committed by author
  README.md
  LICENSE
  tests/
    greeting.test.wfl
  target/package/               generated, not committed
    demo-greeting-1.0.0.wflpkg
```

**Proposed `package.wfl` author syntax** (defaults may be omitted; build fills
them in canonical staging metadata):

```text
create map format:
    "grammar" is "wfl-package-1"
end map

create map package:
    "scope" is "demo"
    "name" is "greeting"
    "version" is "1.0.0"
    "description" is "A greeting container with an editable prefix"
    "authors" is ["Example Author"]
    "license" is "MIT"
    "license_files" is ["LICENSE"]
    "readme" is "README.md"
    "entry" is "greeting.wfl"
    "container" is "Greeting"
    "sources" is ["greeting.wfl"]
    "wfl" is "at least 27.1.0 but below 28.1.0"
    "tests" is ["tests/greeting.test.wfl"]
end map
```

`27.1.0` is an **illustrative future package-capable WFL release**, not a
schedule, current requirement, or claim of implementation. Replace it with the
actual first supported release when implementing the fixtures; the creator
always writes the actual tested runtime range. Package version `1.0.0` is
independent of that runtime range.

The root test uses proposed self binding plus existing test constructs:

```text
include container from package "self" as Greeting

describe "Greeting contract":
    test "greets with the default prefix":
        create new Greeting as greeter:
        end
        expect greeter.greet("Ada") to be "Hello, Ada"
    end test
end describe
```

Complete proposed author sequence, from the shown directory:

```text
wfl package resolve
wfl package validate
wfl package test
wfl package build
wfl package inspect target/package/demo-greeting-1.0.0.wflpkg
```

Expected: zero dependency nodes, one passing test, one archive with exactly
`LICENSE`, `README.md`, `greeting.wfl`, `package.wfl` in that order, and a reported
`sha256:<64 hex digits>` computed from those exact bytes. The hex placeholder is
not a conformance vector or a fabricated digest. Tests/lock are author files,
not members. No code runs during resolve/validate/build/inspect.

Distribute that archive with the reported digest, or use the proposed explicit
registry share command after a trust profile is adopted. The registry does not
rename the root container or rewrite the archive.

### 9.3 Proposed: consume the archive

In sibling directory `hello-app`, run:

```text
wfl package add --archive ../greeting/target/package/demo-greeting-1.0.0.wflpkg --as greeting
wfl package install --locked --offline
wfl main.wfl
```

`add` creates an application manifest and exact lock. Its dependency record is:

```text
create map dependency_greeting:
    "archive" is "../greeting/target/package/demo-greeting-1.0.0.wflpkg"
    "name" is "greeting"
    "scope" is "demo"
    "version" is "1.0.0 exactly"
end map
```

Local archive additions default to an exact version plus digest; registry
additions default to a compatible range. Both use the same record/binding form.
The application main record contains `name is "hello-app"` and the same
illustrative WFL range; it has no root container. The canonical lock has one
runtime node and one development node referencing the same artifact digest,
both reached by `greeting`. Its manifest digest is computed, never hand-entered.

Proposed `main.wfl`:

```text
include container from package "greeting" as Greeting

create new Greeting as greeter:
end

display greeter.greet("Ada")
```

Expected program output: `Hello, Ada`. Instance/call syntax is exactly the
current consumer's syntax. Repeating the import does not create another instance
or execute a constructor. A second `create new Greeting` creates a separate
instance. Neither program execution nor editor completion fetches dependencies.

After reviewing source and tests, an author can release `1.0.1`, and a registry
consumer with a compatible constraint can explicitly update. An exact local
archive dependency changes only when the user selects a new archive or updates
the local bytes explicitly. Removing the dependency while this source still
references `greeting` reports the line and leaves the application usable.

## 10. Larger example: support containers, dependencies, and web resources

This entire package workflow is **proposed**, including its imports and resource
reads. It expands the same model, without inventing a different “expert package.”

```text
welcome/
  package.wfl
  package.lock
  README.md
  LICENSE
  docs/usage.md
  src/
    main.wfl
    counter.wfl
  web/
    page.html
    theme.css
    greeting.js
  tests/
    welcome.test.wfl
  target/package/demo-welcome-1.0.0.wflpkg
```

Manifest (author form):

```text
create map format:
    "grammar" is "wfl-package-1"
end map

create map package:
    "scope" is "demo"
    "name" is "welcome"
    "version" is "1.0.0"
    "description" is "Greeting data and web resources for a welcome page"
    "authors" is ["Example Author"]
    "license" is "MIT"
    "license_files" is ["LICENSE"]
    "readme" is "README.md"
    "entry" is "src/main.wfl"
    "container" is "Welcome"
    "sources" is ["src/main.wfl", "src/counter.wfl"]
    "wfl" is "at least 27.1.0 but below 28.1.0"
    "documentation" is ["docs/usage.md"]
    "tests" is ["tests/welcome.test.wfl"]
    "permissions" is []
end map

create map dependency_greeting:
    "registry" is "https://packages.example.invalid"
    "scope" is "demo"
    "name" is "greeting"
    "version" is "compatible with 1.0.0"
end map

create map resource_page:
    "path" is "web/page.html"
    "media_type" is "text/html"
    "public" is yes
end map

create map resource_theme:
    "path" is "web/theme.css"
    "media_type" is "text/css"
    "public" is yes
end map

create map resource_script:
    "path" is "web/greeting.js"
    "media_type" is "text/javascript"
    "public" is yes
end map
```

`src/counter.wfl` is ordinary WFL source, used privately by the package:

```wfl
create container Counter:
    property total: Number defaults 0

    action increment: Number
        change total to total plus 1
        return total
    end
end
```

Proposed `src/main.wfl` (only the package binding is new syntax):

```text
include from "counter.wfl"
include container from package "greeting" as Greeting

create container Welcome:
    action message needs person_name: Text: Text
        create new Greeting as greeter:
        end
        create new Counter as tally:
        end
        store visits as tally.increment()
        return greeter.greet(person_name) with " — visit " with visits
    end
end

export container Welcome
```

The counter is created per call here intentionally: two calls both report
visit 1. A long-lived session counter would be an explicitly owned instance
or documented static property, not a hidden package singleton. `Counter` is
not importable outside the package and does not appear in the public types.

Resource bodies may be as small as:

```html
<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>Welcome</title>
<link rel="stylesheet" href="theme.css"></head>
<body><p id="greeting">Welcome</p><script src="greeting.js"></script></body></html>
```

```css
body { font-family: sans-serif; }
```

```javascript
document.getElementById("greeting").textContent = "Welcome, visitor";
```

The files are byte-preserved resources. They do not interpolate WFL values or
run during build/install. An application serving them must explicitly map the
three routes and MIME types; this PRD does not imply that WFL automatically runs
JavaScript or escapes arbitrary templates.

Proposed consumer after adding `welcome` from an explicitly configured registry:

```text
include container from package "welcome" as Welcome

create new Welcome as page_service:
end

display page_service.message("Ada")
read package resource "page" from "welcome" as page_bytes
```

Expected output: `Hello, Ada — visit 1`; `page_bytes` is exactly `web/page.html`
as Binary. `Greeting` is a transitive internal dependency, unavailable by alias
in this application unless separately declared. `Counter` is private. There
are nine artifact files: manifest, two source files, README, LICENSE, one extra
document, and three resources. No test, lock, or dependency artifact is bundled.

To add a development-only helper later, add a `dependency_<alias>` record with
`development is yes` and use it in listed tests. It joins only the author's
development graph; consumers of Welcome do not install that helper. Built-in
`describe`/`test` already serves this example, so a dependency is not invented
solely to obtain a test runner.

## 11. Implementation gaps and phased delivery

**PKG-025 — Staged implementation with honest evidence.** Each phase needs a
bounded implementation issue, risk classification, acceptance mapping, and
Maintainer review. This design is not a markdown substitute for the live issue
backlog. No package implementation is included in this document change.

| Component | Existing basis | Required work |
|---|---|---|
| Metadata/format core | WFL map lexer/parser; no current package crate | Frozen restricted parser, canonical writer, schema/lock validation, archive builder/reader, shared limits and diagnostics |
| Parser/AST | Containers, file include/load/export, resource/file operations | Proposed binding/resource statements; contextual tokens without stealing valid syntax; complete public type forms |
| Analyzer | Container/parent scope and include analysis | Package symbol identities, private support environment, direct dependency visibility, declaration profile, cycle/source graph, exports |
| Type checker | Existing container/interface contracts | Derive closed API, reject unresolved public types, strict package errors, cross-package nominal identity, compatibility diff |
| Interpreter | Container definitions/instances, static state, scoped imports, budgets | Inert atomic registration, durable closure environments, package boundary guards, static ownership, cache verification, concurrent/cancelled import handling, resource access |
| CLI | Current file runner and config/init commands; removed package dispatch | Grouped commands, noninteractive/JSON behavior, local transactions/recovery, resolver/cache, explicit test execution |
| LSP | File/module analysis, MCP tools | Parse metadata as data, show root API/dependency/resource completion, definition navigation, source/digest display, no implicit downloads or execution |
| Registry adapter | Historical contracts only | Explicit origin config, snapshot/fetch/share interface, trust provider boundary, authenticated identity and immutable receipts |
| Documentation | Current modules/containers guides and keyword references | New package guide and validated examples when features land; update both keyword references together; retain removal history |

Suggested dependency order:

1. **P0 — Adopt decisions and executable conformance plan.** Approve this
   format/profile/import/CLI scope; define golden metadata/archive/lock vectors,
   typed errors and cross-platform test harness. No public registry commitment.
2. **P1 — Inert local format tools.** Shared metadata parser, profile checks,
   exact canonical archive, inspect/create/validate/build, source contract
   extraction, negative/fuzz cases. Local zero-dependency end-to-end artifact.
   Do not advertise runtime package consumption yet.
3. **P2 — Local dependency graph.** Deterministic solver, lock, local artifact
   cache, offline semantics, transactional add/update/remove, identity checks,
   recovery and reproducibility tests. P1 is a prerequisite.
4. **P3 — Runtime and editor integration.** Proposed binding/resource syntax,
   strict package types/guards, inert registration, private environments,
   lifecycle/concurrency behavior. P1/P2 precede execution. Complete §9 and
   local equivalents of §10 with fixtures; publish honest user documentation.
5. **P4 — Registry transport and release assurance.** Adapter contract tests,
   immutable source-qualified receipts, explicit credentials, compatibility
   checks, full critical journeys through a local test registry. Public launch
   additionally requires the adopted trust profile in §7.4 and portability gate.

Explicit deferrals: runtime capability sandbox, public registry website and
operations, AST normalization/hash identities, full content-addressed filesystem
deduplication, workspace format, native packages, alternate source protocols,
public interface packages, dynamic imports, multiple-version graphs, and wider
public type interchange. Deferrals do not waive any v1 integrity boundary.

## 12. Requirements, acceptance criteria, and future verification

**PKG-026 — Policy-compliant verification.** Future package implementation is
R3 because it touches untrusted input, protocols, lifecycle, and compatibility.
Follow [testing.md](../../testing.md) §§5–8, 11, 14–15: auditable Red before
Green, lowest useful test layer plus every affected higher layer, real binary/
filesystem/socket boundaries, negative assertions, independent review, and
retained evidence. Do not call mock-only tests end-to-end. Do not skip/retry
required failures to manufacture a pass.

The table gives stable acceptance IDs and proposed test homes/names. These tests
are **planned**, not present or passing by virtue of this PRD. Fixture paths
belong under `tests/fixtures/packages/`; intentionally invalid program fixtures
belong under `tests/fixtures/diagnostics/` with exit-status assertions. Positive
gated WFL programs belong under `TestPrograms/packages/`. Temporary artifacts
must be cleaned under temp or `target/test-artifacts/packages/`; reports under
`target/reports/`. No outputs or generated archives are tracked without the
hygiene policy's explicit exception process.

| Requirement | Acceptance criterion | Future tests/layers |
|---|---|---|
| PKG-001 | AC-001: missing/wrong/multiple root export is rejected; a manifest/archive alone is never a package | `tests/package_root_test.rs`: AST + real archive validation |
| PKG-002 | AC-002: §9 root source is unchanged from file reuse through archive import; five-user study target recorded | `TestPrograms/packages/growth.test.wfl`, binary author/consumer journey; usability evidence |
| PKG-003 | AC-003: private support names cannot be imported or returned; internal inheritance/composition works | `tests/package_visibility_test.rs`: analyzer/type/runtime negative cases |
| PKG-004 | AC-004: wrong parameters/return/property writes fail before dispatch/return; inference preserves concrete contract | `tests/package_contract_test.rs`: static and dynamic consumers, no-body-side-effect assertions |
| PKG-005 | AC-005: aliases bind same identity, collisions fail atomically, undeclared/transitive alias rejected | `tests/package_import_test.rs`; binary two-consumer/diamond fixtures |
| PKG-006 | AC-006: imports never call defaults/constructors; direct initialization rules preserved; separate instances/runs, shared statics, cancellation and concurrent waiters correct | `tests/package_lifecycle_test.rs`: real execution, counts, errors, shutdown, budgets and races |
| PKG-007 | AC-007: all existing file modules/containers retain behavior, including repeated side effects and `package:` failure | Existing removal, diamond, scope, interface/static/type tests plus all gated `TestPrograms/` |
| PKG-008 | AC-008: accepted literal corpus parses identically in CLI/LSP/runtime/registry; expressions/imports rejected without evaluation | `tests/package_grammar_test.rs`, parser differential/property corpus and fuzz target |
| PKG-009 | AC-009: required/default/duplicate/unknown fields and exact membership agree across readers | `tests/package_schema_test.rs`: all field types, quoted-key collisions, every size boundary |
| PKG-010 | AC-010: author forms canonicalize identically; canonical round-trip/idempotence; unknown version fails before execution | `tests/package_canonical_test.rs`: fixed golden bytes, Unicode, CRLF/comment cases, old/new-reader fixtures |
| PKG-011 | AC-011: two independent writers produce identical artifacts; timestamp/order/OS changes do not affect bytes; invalid headers/membership rejected | `tests/package_archive_test.rs`: golden ustar bytes, real filesystem, cross-platform build comparison |
| PKG-012 | AC-012: resources preserve bytes/MIME, remain inert, private/missing/traversal reads fail | `tests/package_resources_test.rs`, `TestPrograms/packages/resources.test.wfl`; real serving tests only if a web-serving feature is changed |
| PKG-013 | AC-013: same names from different origins do not substitute; aliases cannot change pinned source | `tests/package_identity_test.rs`: local HTTP boundary adapter tests, redirect/credential redaction |
| PKG-014 | AC-014: SemVer ordering and all range endpoints/pre-release rules; runtime CalVer/format versions remain distinct | `tests/package_versions_test.rs`: table/property tests and incompatible-runtime CLI cases |
| PKG-015 | AC-015: solver gives specified solution from permuted identical snapshots; diamonds/conflicts/cycles/limits/dev graphs correct | `tests/package_resolution_test.rs`: oracle/property and diagnostic-path assertions |
| PKG-016 | AC-016: locks/edges/digests round-trip; install is immutable; offline makes zero requests; corrupted cache/mismatched graph rejected; crash recovery preserves old state | `tests/package_lock_test.rs`, `tests/package_install_test.rs`: real filesystem/process, fault injection, concurrent installs |
| PKG-017 | AC-017: create→add→test→build→consume→update→remove works via actual binary, with piped stdin and spaces in project path | `tests/package_cli_test.rs`, hermetic registry fixture and local archives; exact exits/outputs/side effects |
| PKG-018 | AC-018: removed member/resource/type/constructor changes block incompatible release; additions accepted; behavior-proof limits stated | `tests/package_compatibility_test.rs`: prior release/current consumer fixtures |
| PKG-019 | AC-019: tampered metadata/source/resource changes hash; traversal, links, reparse races, duplicate/case collision, oversized/truncated archives never escape staging | `tests/package_security_test.rs`: real filesystem negative assertions, retained fuzz corpus, independent review |
| PKG-020 | AC-020: malicious top-level/default/hook payloads never execute during every inert operation or editor request | `tests/package_no_execution_test.rs`: sentinel files, real listening sockets, subprocess sentinels; assert absence, not only exit code |
| PKG-021 | AC-021: declarations/union/escalations are displayed accurately; no false runtime sandbox claim; ordinary execution budgets still constrain packages | `tests/package_permissions_test.rs`: CLI/LSP contract and runtime budget integration |
| PKG-022 | AC-022: trusted source mismatch, invalid signature/revocation/expired trust data are handled per adopted profile; unsigned mode never claims verification | `tests/package_trust_test.rs`: versioned trust fixtures and real adapter boundaries |
| PKG-023 | AC-023: registry manifest/artifact mismatch rejected; idempotent share, immutable versions, owner authorization, yank/revoke and credential failures observable | `tests/package_registry_contract_test.rs`: real local server, tokens restricted to fixture origin |
| PKG-024 | AC-024: every failure identifies code, cause, relevant span/source and safe next action; JSON/TTY modes agree and redact credentials | `tests/package_diagnostics_test.rs`, LSP protocol tests, byte-offset/Unicode span cases |
| PKG-025 | AC-025: phase cannot advertise features before its linked gates/docs pass; no prototype becomes a release by renaming it | Phase issue/PR review, docs manifest, release evidence |
| PKG-026 | AC-026: all required layers and immutable-artifact evidence pass for declared tuples with Red chronology and independent review | `testing.md` presubmit/release profile and retained evidence manifest |

Malformed corpus additionally includes integer overflow, UTF-8 overlong/surrogate
encodings, BOM, control/bidi text, unknown keys/grammar, duplicate records,
non-literal metadata, list nesting, truncated lock, wrong package/root identity,
source imports escaping membership, prerelease selection, mutable local sources,
and unsupported hash tags. Fixed seeds/minimized failing inputs are retained.

Lifecycle/transport tests must cover ordering/races, cancellation at each stage,
timeouts, disconnects, bounded buffering, slow/failing peer isolation, clean
shutdown, writes after cancellation/close, and atomic recovery. Tests needing
network use local ephemeral servers, not the illustrative public hostname.

Portability: require Windows on NTFS without Developer Mode, Linux, and macOS
with case-insensitive filesystem coverage, plus a case-sensitive tree. Compare
artifact/lock hashes for the same LF input fixture; test CRLF rejection/edit
guidance, path case/reserved names, Unicode free text, space-containing outer
directories, cache permissions and interrupted rename. Add macOS CI before
claiming gated macOS package support. Current repository CI does not supply it.

Future behavioral presubmit includes `cargo fmt --all -- --check`,
`cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all`,
the locked fuzz-workspace compile check, release build and the integration/web
runners where affected, docs validation, and both applicable hygiene modes.
Required release journeys test the **actual distributed artifacts**, record
digests, run upgrade/rollback/cleanup checks, and never rebuild after the gate
without invalidating its evidence. Coverage and evidence retention follow
`testing.md`; this PRD introduces no waiver.

## 13. Assumptions and maintainer decisions

Assumptions: v1 distributes source libraries; the root is the sole public type;
the existing container/module system remains backward compatible; local tools
may ship before a public registry. These are design assumptions, not claims
about user adoption or current enforcement.

The following require Maintainer adoption **before implementation/publication**,
not answers before this design document can be reviewed:

| Decision | Recommendation | Consequence if changed |
|---|---|---|
| D-01 Container/API shape | Definition root, all root members public, private support graph, explicit instances | Changes contract extraction, examples and import behavior together |
| D-02 Metadata/archive | Literal-map schema and uncompressed canonical ustar | Freeze golden vectors only after adoption; do not ship an interim incompatible format |
| D-03 Dependency contract | SemVer libraries, bounded English constraints, deterministic solver, one version/source identity | Revisit ranges and solver together; MVS cannot be substituted while retaining upper bounds |
| D-04 CLI/import additions | Grouped commands and explicit container-binding/resource syntax | Parser compatibility and file-name dispatch tests must approve the chosen grammar |
| D-05 Trust/release scope | Local-first delivery, explicit sources, public registry blocked on a separate trust profile | Maintainers choose origin, trust keys/providers, custody, publication authority and operations |
| D-06 Runtime security scope | No sandbox claim in v1; enforce no-execution tooling boundary and current runtime limits | If manifest-enforced capabilities become a v1 requirement, add the full enforcement project before release |
| D-07 Portability commitment | Add a macOS package gate alongside Windows/Linux before claiming portable v1 conformance | Otherwise label macOS experimental and do not claim the three-platform success target |

Risks are bounded but real: unique package identities require changes through
analyzer/typechecker/runtime, type inference may be insufficient for some public
contracts, retained definition environments need lifecycle care, and maintaining
a new data format carries long-term costs. The phased sequence validates those
risks before registry names and trust roots become public commitments.

## 14. Draft validation and consistency review

### 14.1 Evidence for current claims

This R0 change does not implement a format or alter runtime behavior, so
`testing.md` §6.3 requires no manufactured Red step. Current-behavior research
uses source inspection, focused existing Rust tests, and direct execution of
the ordinary WFL examples. Proposed syntax is fenced as `text` and labeled
proposed; it is not placed among executable shipped examples or marked as a
passing skipped test. WFL map syntax acceptance alone would not prove the
proposed metadata schema exists.

Validation commands/results for this draft are recorded in the accompanying
[Dev Diary entry](../../History/dev-diary/2026/2026-09-23-package-system-prd.md).
That record distinguishes current examples and existing tests from future
acceptance tests, and records any existing ignored tests or tooling limitations.

### 14.2 Final consistency review

| Cross-check | Result of design review |
|---|---|
| Container → manifest → import | `Greeting` is a definition in `greeting.wfl`, selected by metadata and root export; import binds it; only `create new` creates consumer state |
| Beginner → larger package | Both use literal maps, explicit source membership and ordinary instances. Supporting `Counter` and dependency `Greeting` do not become alternative package forms |
| Visibility → type contracts | All root members form the API; support/dependency instances cannot leak through the restricted public type set; no unsupported private syntax is taught |
| Inspection → initialization | Metadata never executes. Source profile excludes computed defaults/top-level effects. Import performs atomic declaration registration; tests/actions execute only explicitly |
| Examples → source layout | Minimal artifact has four members; larger artifact has nine. Tests and lock stay with the author; resources are explicitly named; dependency bytes remain external |
| Identity → version → resolution | Source+scope+name is stable; SemVer selects package compatibility; CalVer limits runtime; envelope selects format. Constraint solver supports the actual upper-bound grammar |
| Lock → build → offline | Lock pins exact dependency artifact bytes; root does not self-hash; build never updates/resolves online; local changes require explicit update |
| Canonicalization → integrity | Author formatting may differ; distributed metadata/ustar has one encoding. SHA-256 covers actual archive bytes, including JS/CSS and licenses; no AST hash substitutes |
| Resources → security | Resource read returns bytes only; no automatic route, browser execution, permission grant, or escaping claim |
| Archive → portability | Fixed paths, headers, ordering, LF source rules and byte-preserved resources agree with three-platform reproducibility tests; no symlink/admin prerequisite |
| File modules → removal tests | Existing include/load semantics and `package:` path failures persist; new binding is distinct; new CLI help tests must describe an explicitly adopted addition |
| Permissions → trust wording | Declarations are informational; no-hooks/containment are enforceable; digest integrity is not honesty or registry-independent authenticity |
| Requirements → delivery | PKG-001–026 each map to an acceptance criterion and test/evidence layer; implementation remains future R3 work, this artifact remains a proposal |
| Historical designs → authority | Contradictions are resolved in §2.3, archived bytes preserved, and registry trust/adoption decisions remain with Maintainers |

The design is ready for implementation planning and Maintainer review. It is
not a claim that package support, a public registry, runtime sandboxing, or any
future acceptance suite has shipped.
