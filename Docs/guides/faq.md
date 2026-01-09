# Frequently Asked Questions (FAQ)

Common questions about WFL answered.

## General Questions

### What is WFL?

WFL (WebFirst Language) is a programming language that uses natural English-like syntax instead of cryptic symbols. It's designed to make coding intuitive and accessible.

**[Learn more →](../01-introduction/what-is-wfl.md)**

---

### Is WFL production-ready?

**No.** WFL is in alpha (v26.1.17). It's great for:
- ✅ Learning programming
- ✅ Side projects
- ✅ Prototypes
- ✅ Experimentation

**Not recommended for:**
- ❌ Production applications
- ❌ Critical systems
- ❌ Business-critical services

Wait for stable 1.0 release for production use.

---

### Why natural language syntax?

**Easier to learn:** Beginners can understand code without memorizing cryptic symbols.

**Easier to read:** Code reads like English, making it self-documenting.

**Easier to maintain:** Come back months later and understand instantly.

**[Philosophy explained →](../01-introduction/natural-language-philosophy.md)**

---

### Is WFL slow?

**No slower than other interpreted languages.** WFL is:
- Interpreted (like Python, Ruby)
- Built on Rust (fast foundation)
- Uses Tokio for async (efficient I/O)

**Future:** Bytecode VM planned for better performance.

**[Performance tips →](../06-best-practices/performance-tips.md)**

---

### Can I use WFL for...?

**✅ Web servers and APIs:** Yes! Built-in HTTP server.

**✅ Automation scripts:** Yes! File I/O, subprocess execution.

**✅ Command-line tools:** Yes! REPL and argument handling.

**✅ Data processing:** Yes! Pattern matching, lists, file operations.

**❌ Mobile apps:** No (not designed for mobile).

**❌ Desktop GUI apps:** No (CLI/web only).

**❌ Game development:** No (not optimized for real-time graphics).

**❌ Systems programming:** No (use Rust, C, C++).

---

### How does WFL compare to...?

**vs JavaScript:**
- More readable syntax
- Type-safe
- No semicolons/braces
- Built-in web server
- No npm dependency hell

**vs Python:**
- Similar readability philosophy
- Natural language instead of minimal syntax
- Type-safe by default
- Built-in web server

**vs Rust:**
- Easier to learn
- Higher level
- Interpreted (not compiled)
- Natural syntax instead of systems syntax

**[Detailed comparisons →](../01-introduction/first-look.md)**

---

## Getting Started

### How do I install WFL?

**Windows:** Download MSI installer from GitHub Releases

**All platforms:** Build from source with Rust/Cargo

**[Installation guide →](../02-getting-started/installation.md)**

---

### Where do I start learning?

**Complete beginner:**
1. [What is WFL?](../01-introduction/what-is-wfl.md)
2. [Installation](../02-getting-started/installation.md)
3. [Your First Program](../02-getting-started/your-first-program.md)

**Experienced developer:**
1. [First Look](../01-introduction/first-look.md) (code comparisons)
2. [Installation](../02-getting-started/installation.md)
3. [Language Basics](../03-language-basics/index.md) (skim)

**[Learning paths →](../README.md#-learning-paths)**

---

### What's the best editor?

**VS Code (recommended):**
- Syntax highlighting
- LSP integration
- MCP integration with Claude
- Auto-completion

**But any editor works!** WFL is just text files.

**[Editor setup →](../02-getting-started/editor-setup.md)**

---

## Technical Questions

### Does WFL have a package manager?

**Not yet.** Planned for future versions.

**Current workaround:** Copy WFL files between projects.

---

### Can I import/include other WFL files?

**Not yet.** Module system planned for future.

**Current workaround:** Copy-paste shared code or use subprocess to call other WFL programs.

---

### Does WFL compile or interpret?

**Interprets.** WFL executes the AST directly.

**Future:** Bytecode VM planned for better performance.

---

### Is WFL type-safe?

**Yes!** Static type checking with inference.

```wfl
store age as 25
store name as "Alice"
// age plus name  // ERROR: Cannot add Number and Text
```

Compiler catches type errors before runtime.

---

### Can I use JavaScript libraries?

**Not yet.** JavaScript interop is planned.

**Current workaround:** Use subprocess to call Node.js scripts.

**[Interoperability →](../04-advanced-features/interoperability.md)**

---

## Contributing

### Can I contribute to WFL?

**Yes!** All contributions welcome:
- Bug reports
- Feature requests
- Documentation improvements
- Code contributions
- Examples
- Tests

**[Contributing guide →](../development/contributing-guide.md)** *(coming soon)*

---

### Where do I report bugs?

**GitHub Issues:** https://github.com/WebFirstLanguage/wfl/issues

Include:
- WFL version
- Operating system
- Code that reproduces the bug
- Expected vs actual behavior

---

### How do I suggest features?

**GitHub Discussions:** https://github.com/WebFirstLanguage/wfl/discussions

Or open an issue with `enhancement` label.

---

## Backward Compatibility

### Will my code break in future versions?

**No.** WFL guarantees backward compatibility.

> Code you write today will work with all future versions.

We won't break features unless a critical security bug forces us. Even then, 1+ year deprecation notice.

---

### What's the version scheme?

**YY.MM.BUILD** (calendar-based)

Example: `26.1.17` = January 2026, build 17

Easy to understand when releases happen.

**[Version scheme explained →](../README.md#-version-scheme)**

---

## Support

### Where do I get help?

**Documentation:** Start with [Docs/README.md](../README.md)

**Community:**
- GitHub Issues (bugs)
- GitHub Discussions (questions)
- Email: info@logbie.com

**Resources:**
- 90+ TestPrograms examples
- Complete standard library reference
- Troubleshooting guide

**[All resources →](../02-getting-started/resources.md)**

---

### Is there a WFL community?

**Growing!** Connect via:
- GitHub Discussions
- GitHub Issues
- Email

Share projects, ask questions, help others!

---

## Future of WFL

### What's next for WFL?

**Planned features:**
- Bytecode VM (performance)
- Module system (code organization)
- Package manager (dependency management)
- JavaScript interop (library access)
- Native JSON support (parsing/generation)
- Database adapters (PostgreSQL, SQLite)

**[Check roadmap in GitHub →](https://github.com/WebFirstLanguage/wfl)**

---

### When will WFL be stable?

No timeline yet. WFL will reach 1.0 when:
- Core features complete and tested
- Performance optimized
- No major breaking changes needed
- Community feedback incorporated
- Production-ready quality

**Alpha status** means active development and improvements.

---

## Miscellaneous

### Who created WFL?

**Logbie LLC** with AI assistance from:
- Devin.ai (development)
- ChatGPT (code review)
- Claude (documentation)

Open source under Apache 2.0 license.

---

### Why "WebFirst"?

WFL is designed for **web development first**:
- Built-in web servers
- HTTP support
- File I/O for web apps
- Async for web requests

But also great for general scripting!

---

### Can I use WFL commercially?

**Yes!** Apache 2.0 license allows commercial use.

**But remember:** Alpha software, not production-ready yet.

---

**Still have questions? Ask in [GitHub Discussions](https://github.com/WebFirstLanguage/wfl/discussions)!**

---

**Previous:** [← Migration from Python](migration-from-python.md) | **Next:** [Reference →](../reference/)
