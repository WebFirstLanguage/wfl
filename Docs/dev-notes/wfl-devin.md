Great. I’ll create a Markdown-based guide tailored for AI agents working on the WebFirst Language (WFL). It will cover all major areas of WFL development—including parser/compiler/runtime implementation, error documentation, test writing, UX authoring—and include WFL-specific examples to help AI agents follow the project's standards and style. I’ll let you know once the full guide is ready for review.

# WebFirst Language (WFL) Development Guide for AI Agents

**Target Audience:** Autonomous or semi-autonomous AI agents (like Devin.ai) contributing to the WFL project.

This guide outlines clear **Do’s and Don’ts** for AI contributors, provides examples of well-scoped tasks, and offers best practices for both technical and non-technical work. It emphasizes WFL’s guiding principles – **natural-language syntax, minimal special characters, beginner-friendly design, and web-focused functionality** – and sets success criteria for each task.

## How to Take Tasks

AI agents should approach tasks methodically, ensuring clarity and alignment with WFL’s philosophy from the start. Follow these guidelines when picking up a new task:

- **Confirm Understanding:** Begin by restating or summarizing the task in your own words. This ensures you fully grasp the scope. If the prompt is vague or broad (e.g., “Make the language faster”), **ask for clarification or break it down** before proceeding. AI agents should avoid open-ended instructions that lack clear outcomes.
- **Scope the Work:** Identify whether the task is *technical* (code implementation, parser changes, runtime features) or *non-technical* (documentation, error message wording, UX copy). Well-scoped tasks have a defined goal and measurable outcome. For example, *“Implement support for async/await in the WFL runtime”* is clear, whereas *“Improve concurrency”* is too broad.
- **Check WFL Principles:** Always cross-reference WFL’s guiding principles before starting. Ensure the solution will use **natural-language syntax and minimal symbols**. If a task suggests adding complex symbols or esoteric syntax, reconsider or consult maintainers – it might conflict with WFL’s design (for instance, introducing lots of punctuation would violate the **minimal special characters** rule).
- **Do’s:** 
  - Do break the task into smaller steps or milestones (plan parsing, then code generation, then testing, etc.).
  - Do maintain **readability and clarity** in any code or text you produce – WFL code and docs should read like plain English.
  - Do use the WFL specification and existing test cases as references to ensure consistency.
  - Do communicate uncertainties. If you’re unsure about requirements (e.g., how a new loop should behave), leave a comment or ask a facilitator rather than guessing.
- **Don’ts:** 
  - Don’t accept tasks that are ill-defined. If something is like *“Make the language better”* without specifics, you should request a more concrete goal (e.g., *“Reduce compilation time by 20%”* or *“Add support for X feature”*).
  - Don’t introduce changes that violate WFL’s style or design philosophy. For example, avoid adding a feature that requires heavy symbolic notation or isn’t **beginner-friendly**.
  - Don’t proceed without a plan. Jumping straight into coding without outlining your approach can lead to inconsistent or incomplete solutions – even as an AI, planning is crucial for complex tasks.
  - Don’t ignore error handling or edge cases; WFL prides itself on clear, actionable errors, so any new feature you add must include proper validation and helpful messages.

By carefully **taking on tasks with the right mindset**, AI agents set themselves up for success and ensure their contributions fit seamlessly into the WFL project.

## Patterns to Follow

For each category of work, follow established patterns to maintain consistency and quality. Whether the task is building a parser or writing documentation, use these step-by-step approaches:

### Technical Task Patterns (Code, Parser, Runtime)

When implementing **WFL language features** (syntax analysis, compiler architecture, I/O runtime, async support, etc.), adopt a structured approach:

1. **Review Specifications:** Start by reading the relevant part of the WFL spec or design docs. For example, if adding a loop construct, see how loops are defined in the spec to match keywords and structure. Ensure your approach aligns with examples and grammar rules in the spec.
2. **Plan the Implementation:** Outline how you will implement the feature:
   - For a **parser task**, decide which grammar or parsing technique to use. *Pattern:* Write grammar rules or use parser combinators that reflect WFL’s natural language style. *Example:* If writing a loop parser for `count from 1 to 5`, plan an EBNF/PEG rule for the “count from X to Y [by Z]” syntax before coding. Ensure it covers variations (with/without a step, down-counting, etc.) as per spec.
   - For a **runtime feature**, determine how it fits into the existing architecture. E.g., adding asynchronous support might involve extending the event loop or promise handling in WFL’s runtime. Plan data structures or state machines needed.
3. **Iterative Development:** Implement in small increments and test each part:
   - Write the minimal code to parse or handle one aspect, then run unit tests or sample scripts. 
   - Gradually add complexity. For a parser, start by handling the simple form of the syntax (say, `count from A to B` with default increment) and test with a snippet. Then add optional parts like `by Z` or reversed loops and test those.
   - Use AI strengths: you can generate multiple test cases quickly. For instance, prompt yourself with *“Generate WFL loop examples including edge cases”* to create test inputs.
4. **Testing and Validation:** Ensure there are **unit tests** and, if relevant, integration tests:
   - If the project has a test suite (e.g., in a `tests` directory), add new tests covering your implementation. For our loop example, *write a unit test to confirm that a `count from 1 to 5` loop executes 5 times and that the loop variable iterates correctly.* This might involve parsing a WFL snippet and checking that the AST or executed output matches expected results.
   - Run all existing tests to confirm you didn’t break anything. AI agents should strive for **no regression failures**.
   - Use success criteria: The new code should parse valid WFL and reject invalid WFL according to spec. For instance, if spec says a loop must have an `end count`, your parser should error without it. Verify that error messages follow WFL’s friendly style (e.g., *“It looks like you forgot `end count` to close the loop”* instead of just *“Syntax error”*).
5. **Code Style and Consistency:** Follow the project’s coding conventions. Use clear naming (even in code, variable and function names can reflect WFL’s readability ethos). Comment your code with reasoning in plain language if needed – remember, human maintainers will read AI contributions too. For example, a comment might say: `// The parser captures the 'count from X to Y' syntax and handles optional step or reverse keywords`.

### Documentation and UX Task Patterns (Docs, Error Messages, UX Copy)

For tasks that involve writing text (documentation, user-facing error messages, UX/UI copy):

1. **Use WFL’s Voice and Tone:** WFL documentation aims to be **beginner-friendly and explanatory**, almost like a “for dummies” guide. Emulate this style:
   - Use simple, conversational language.
   - Explain concepts with analogies or plain English first, then show the WFL code example.
   - Keep examples abundant and straightforward. *(E.g., when documenting loops, you might write:* “Loops in WFL let you repeat actions in English. For example: `count from 1 to 5: display "Hello!" end count` will display "Hello!" five times.)*
2. **Structured Format:** Organize documentation into clear sections and use lists when appropriate (just like this guide). Headings should be logical, and content should build progressively. Avoid long paragraphs – break them into steps or bullet points for readability.
3. **WFL Code Samples:** Include inline WFL code snippets to illustrate points. Always test these snippets mentally (or via a compiler if available) to ensure they are accurate. For instance, when showing how to create a variable, you might write:
   ```wfl
   store name as "Alice"
   store count as 42
   ``` 
   and explain that this creates a text variable `name` and a number variable `count`. Make sure your examples reflect actual WFL syntax and follow the principle of minimal symbols (notice the use of words like `store ... as ...` instead of an equals sign).
4. **Error Message Crafting:** When writing error or warning messages, follow WFL’s **clear and actionable** error policy:
   - Phrase errors like a helpful guide. Point out the issue and suggest a fix.  
   - For example, instead of `TypeError: Expected number, got text`, write *“Expected a number but found text — try converting it first.”* This matches WFL’s Elm-inspired friendly tone.
   - Keep messages free of internal jargon or stack traces; focus on what the user did and how to resolve it.
5. **UX Copy and Interface Text:** If generating text for any WFL-related tools or IDE integration (like button labels, tooltips, etc.), maintain consistency in voice. WFL’s tone is encouraging and clear. For instance, a tooltip in a WFL editor might say, *“Run the WFL script”* rather than *“Execute code”*. Such subtle choices keep the language aligned with WFL’s branding.
6. **Review and Edit:** Even as an AI, double-check the output for coherence. Read the documentation or messages from a beginner’s perspective – do they make sense? Are they free of typos and grammatically correct? If possible, have another AI agent or a human reviewer verify that the content is understandable and accurate.

By following these patterns, AI agents ensure that both code contributions and written materials are high-quality and in harmony with the WFL project’s standards.

## Common Pitfalls

Even advanced AI agents can stumble. Be mindful of these common pitfalls and know how to avoid them:

- **Taking on Vague Prompts:** As mentioned, avoid proceeding with tasks that are too broad or unclear. *Pitfall:* Acting on an instruction like “optimize WFL” can lead to aimless changes or conflicts. **Solution:** Break it down. Identify a specific aspect to optimize (e.g., parsing speed, runtime memory usage) and confirm that focus with stakeholders before coding.
- **Overstepping Scope:** AI agents sometimes produce more than asked, which can be risky. For example, if tasked to “write a parser for WFL loops”, a pitfall would be also modifying unrelated parts of the language or adding new loop types not in the spec. **Stick to what’s requested.** If you see an opportunity for improvement outside the task, note it for future work instead of expanding the current scope.
- **Ignoring WFL Syntax Rules:** Make sure you’re thoroughly familiar with WFL’s syntax and grammar before generating code or docs. *Pitfall:* Introducing an incompatible syntax (e.g., using `{}` braces for blocks or adding semicolons) would violate WFL’s design. Always validate against the official grammar or examples. WFL uses `end if`, `end loop` for blocks – forgetting this could cause big errors.
- **Inconsistent Style or Terminology:** All contributors, including AI, must use consistent terminology. If the spec calls the boolean values **yes/no** (and not true/false), your code and documentation should do the same. A common mistake is reverting to typical programming terms (like null, false, etc.) instead of WFL’s natural equivalents (like “nothing” or “no”). Keep a glossary if needed to stay consistent.
- **Lack of Testing and Validation:** Skipping tests is a major pitfall. An AI might assume the code is correct if it compiles, but without running tests (or writing new ones), bugs can slip in. Always run the test suite. If the project doesn’t have one yet, create a simple battery of tests for your feature. For instance, if implementing a new string function, test it with typical, edge-case, and invalid inputs.
- **Insufficient Comments or Explanations:** Remember that human developers will review AI contributions. If your code fix or feature is non-trivial, not leaving any explanation is a mistake. Provide a brief description of *why* you did something in a commit message or code comment. For example, *“// Using a map here for faster lookups in the type checker (improves performance for large scripts)”*. This helps maintainers follow your reasoning.
- **Forgetting the User’s Perspective:** WFL is all about the end-user (often a beginner coder). A pitfall is focusing so much on internal implementation that the end-user experience suffers (e.g., an error that’s technically precise but confusing to a novice). Continuously ask: “If I were new to programming, would this feature or message make sense to me?” If not, adjust accordingly.

By anticipating these pitfalls, AI agents can correct course proactively. It’s often helpful to run a mental (or actual) checklist after completing a task to see if any of these issues might be present.

## Checklist Before You Submit

Before considering a task truly “done,” go through this checklist to ensure all criteria are met and the solution is robust:

1. **Requirements Met:** Verify that you’ve fulfilled the exact task requirements. Re-read the original prompt. If it said “implement X and add tests,” do you have both the implementation and the tests? If it was to “write documentation for Y feature,” did you cover all important aspects of Y?
2. **Consistency with WFL Principles:** Do a final pass for stylistic consistency:
   - Does the code or documentation use **natural-language syntax** and avoid unnecessary symbols?
   - Is everything phrased in a **beginner-friendly** way, without assuming too much prior knowledge?
   - If you introduced a new keyword or construct, is it in line with the simplicity and clarity goals of WFL? *(For instance, a new keyword should probably be an English word or phrase, not a symbol or obscure term.)*
3. **Testing and Validation:** Ensure **all tests pass**. This includes:
   - New tests you wrote for this feature.
   - Existing regression tests – none should fail after your changes. If failures occur, address them or seek input if the changes were intentional.
   - If applicable, run a sample WFL program that uses your new feature end-to-end. Does it behave as expected? (E.g., compile a small WFL script using the new async feature, run it, and confirm the output or effect is correct.)
   - Validate error handling: induce an error related to your feature to see the message. Check that the wording is clear and helpful.
4. **Documentation Updated:** If you added or changed a language feature, did you update or add documentation for it? Search the docs for related sections. For example, if you modified how loops work, update the loop section in the language reference or tutorial. All examples in docs should still be accurate – adjust any that are affected by your changes.
5. **Peer Review / Comments:** Prepare a summary of what you did for human facilitators:
   - If using a version control system, your commit message or pull request description should clearly outline the change. E.g., “Added support for `repeat ... end repeat` loops, including parser rule, AST node, and execution in the interpreter. All tests pass.” Also mention any follow-up needed.
   - If you encountered challenges or made design decisions, note them. *For instance:* “Decided to implement async with a simple queue; in the future, may need a more robust event loop for multiple concurrent tasks.”
   - Ensure your work is annotated where necessary. If you are handing off to a human, leaving a short note like “Ready for review – see added tests for usage examples” can be very helpful.
6. **Final Self-Review:** Take a moment to simulate the user or maintainer experience:
   - **For code:** Read through the code as if you’re a code reviewer. Is it clean? Any obvious inefficiencies or complex parts that need a comment? Does it integrate well with the rest of the codebase (naming, file organization, etc.)?
   - **For documentation:** Read it as if you’re a first-time learner. Does it flow logically? Are all new terms explained? Consider running a spell-check or grammar-check (AI can self-check language).
   - **For output/messages:** If applicable, run a quick demo of any user-facing output. Make sure it looks good (no weird formatting, all placeholders fill in correctly, etc.).

Only once this checklist is all green should the AI agent mark the task as complete or submit it for human review. This diligence ensures each contribution is solid, aligns with WFL’s mission, and requires minimal rework.

---

By adhering to this guide, AI agents collaborating on the WebFirst Language project can produce high-quality, consistent contributions. The key is **communication, clarity, and consistency** – both with the core principles of WFL and with the development practices that the project follows. AI agents that take well-scoped tasks (e.g., *“Write a unit test for the WFL loop parser handling ‘count from 1 to 5’”*), follow the recommended patterns, avoid common pitfalls, and verify their work against the checklist will greatly accelerate WFL’s development while maintaining its vision of an intuitive, web-focused programming language. 

Remember, the ultimate goal is a language that “reads like plain English” and empowers beginners, so every commit – whether code or documentation – should move WFL closer to that ideal. Happy coding (and training)! 

