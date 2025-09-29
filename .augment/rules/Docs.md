---
type: "always_apply"
description: "Example description"
---

📂 Where to Put Things

All documentation belongs under docs/ at the project root. If you’re tempted to start a new folder somewhere else, imagine Ritsu smacking your hand away with a drumstick. Centralizing docs makes them easier to find and keeps the repo tidy.

Core language feature docs live in docs/wfldocs/. Each file in this directory should describe a feature that already exists in the language—think variables, control flow, pattern matching, and so on. Name these files with a WFL- prefix followed by a concise, hyphenated description (e.g., WFL-variables.md, WFL-actions.md). Clear, descriptive names and consistent prefixes help readers (and search tools) understand what’s inside
.

Planned or experimental features go in docs/wflspecs/. These “spec” documents outline features that are proposed but not yet implemented. Use descriptive filenames (a SPEC- prefix is recommended) and include the status (draft, planned, under discussion) at the top. Explain the rationale, proposed syntax, semantics, and any open questions.

A single “living AI document” stays at the root of docs/. This file (for example, wfl-living-ai.md) serves as a constantly‑updated cheat sheet for AI agents building WFL apps. It should summarize current language features, list available modules, and provide guidance on composing WFL code using natural language. Whenever the language or its specs evolve, update this document so AI agents aren’t left playing catch‑up.

🧰 How to Structure Your Docs

When adding or updating documentation, follow these best practices:

Choose the right location. Place user‑facing docs in wfldocs/, planned features in wflspecs/, and keep the living AI document at the root. If your content doesn’t fit neatly into one of these, think again—good organization is half the battle

.

Use consistent naming conventions. File names should be lowercase, hyphen‑separated, and start with an appropriate prefix (WFL- or SPEC-). Avoid cryptic abbreviations. Pretend you’re explaining it to a friend who’s never seen the code

.

Update the index. Whenever you add a new document, make sure it appears in the documentation index (or table of contents) so others can find it

.

Follow the WFL documentation policy and foundation guidelines. Write in a friendly, conversational tone, avoid jargon, and use plenty of examples
. Your goal is to be a mentor, not a gatekeeper.

Cross‑reference related docs. Link to other relevant pages so readers can explore topics in depth. For example, a spec for pattern matching improvements should link back to the existing WFL-patterns.md.

Provide clear, actionable information. Use natural language to describe concepts, prioritize clarity over brevity, and make documentation accessible to beginners

. If your doc reads like a textbook, lighten it up—imagine you’re explaining it over coffee.

✍️ Writing Style and Tone

The WFL docs should feel like a conversation with a knowledgeable friend. Keep sentences short, avoid obscure terminology, and include examples wherever possible. Stick to natural language syntax and minimize unnecessary symbols. Remember that WFL is designed for humans first, computers second.

Use a warm, encouraging tone. Explain concepts step‑by‑step and invite readers to experiment. When showing code, favor plain English constructs over terse symbols. For example, “Let the age be 25” is preferable to “int age = 25;”

🔄 Keeping Docs Up to Date

Documentation is a living system, not a one‑time dump. Review your docs regularly to ensure they match the current implementation and planned features. Update the living AI document whenever the language evolves. If a spec graduates to a full feature, move it from wflspecs/ into wfldocs/ and rename it with a WFL- prefix.

Always track changes through version control and include a summary of updates so others understand what’s new. Encourage feedback and contributions from the community—fresh eyes catch mistakes and spark new ideas.