# Getting Started with WFL

Welcome! In this section, you'll install WFL and write your first programs. By the end, you'll have a working development environment and understand WFL basics.

## What You'll Do

This is a hands-on guide. You'll:

1. **[Install WFL](installation.md)** - Get WFL running on your machine (5 minutes)
2. **[Write "Hello, World!"](hello-world.md)** - Your first WFL program (2 minutes)
3. **[Build Your First Program](your-first-program.md)** - Interactive tutorial (15 minutes)
4. **[Explore the REPL](repl-guide.md)** - Experiment interactively (10 minutes)
5. **[Set Up Your Editor](editor-setup.md)** - VS Code, LSP, and MCP integration (10 minutes)
6. **[Find Resources](resources.md)** - Where to learn more

**Total time:** About 45 minutes from zero to productive.

## Prerequisites

You'll need:

- **A computer** running Windows, Linux, or macOS
- **Rust** (1.75 or later) if installing from source
- **Text editor** (VS Code recommended, but any editor works)
- **Basic command-line knowledge** (opening a terminal, running commands)

Don't worry if you're new to programming! We'll guide you through everything.

## Two Installation Paths

Choose the path that fits your needs:

### **Path 1: Windows MSI Installer** (Recommended for Windows)
- ✅ Easiest installation
- ✅ Includes VS Code extension
- ✅ Automatic PATH setup
- ✅ Optional LSP server
- ⏱️ 5 minutes

**[Windows Installation Guide →](installation.md#windows-msi-installer)**

### **Path 2: Build from Source** (All platforms)
- ✅ Latest features
- ✅ Cross-platform (Windows, Linux, macOS)
- ✅ Full control over installation
- ⏱️ 5-10 minutes (depending on your machine)

**[Source Installation Guide →](installation.md#from-source)**

## Quick Start (For the Impatient)

Already have WFL installed? Jump straight to coding:

```bash
# Create a file called hello.wfl
echo 'display "Hello, World!"' > hello.wfl

# Run it
wfl hello.wfl
```

**Output:** `Hello, World!`

Congratulations! You're programming in WFL.

**[Continue to Your First Program →](your-first-program.md)**

## Learning Paths

Different paths depending on your experience:

### **New to Programming?**
Follow the guide in order:
1. [Installation](installation.md) - Get set up
2. [Hello World](hello-world.md) - First success
3. [Your First Program](your-first-program.md) - Learn by doing
4. [REPL Guide](repl-guide.md) - Experiment safely
5. Then explore [Language Basics](../03-language-basics/index.md)

### **Experienced Developer?**
Quick path:
1. [Installation](installation.md) - 5 minutes
2. [Your First Program](your-first-program.md) - See WFL in action
3. [Editor Setup](editor-setup.md) - Get LSP and VS Code working
4. Jump to [Language Basics](../03-language-basics/index.md) or [Advanced Features](../04-advanced-features/index.md)

### **Want to Experiment First?**
Try before installing:
1. [Hello World](hello-world.md) - See what WFL looks like
2. [First Look](../01-introduction/first-look.md) - More examples
3. Then [install](installation.md) when ready

## What to Expect

### After Installation
You'll be able to:
- ✅ Run WFL programs from the command line
- ✅ Write `.wfl` files in any text editor
- ✅ See helpful error messages
- ✅ Use the interactive REPL

### After Editor Setup
You'll get:
- ✅ Syntax highlighting
- ✅ Real-time error checking
- ✅ Auto-completion
- ✅ Go-to definition
- ✅ Hover documentation

### After This Section
You'll know:
- ✅ How to install and run WFL
- ✅ Basic syntax (variables, loops, conditionals)
- ✅ How to experiment with the REPL
- ✅ Where to find help and resources

## Common Questions

**Q: Do I need to know programming already?**
A: No! WFL is designed for beginners. We'll teach you everything.

**Q: How long does installation take?**
A: 5 minutes with the MSI installer, 5-10 minutes from source.

**Q: Can I use my favorite editor?**
A: Yes! WFL works with any text editor. VS Code has the best support (syntax highlighting, LSP), but you can use Vim, Emacs, Sublime, or anything else.

**Q: Is WFL ready for production?**
A: Not yet. WFL is in alpha. Great for learning and side projects, but wait for stable release for production.

**Q: Will my code break when WFL updates?**
A: No. We guarantee backward compatibility. Your code will work with future versions.

**Q: Where do I get help?**
A: Check [Resources](resources.md) for documentation, GitHub issues, and community links.

## Ready to Start?

Let's install WFL and write your first program!

**[Begin Installation →](installation.md)**

---

Already familiar with WFL basics? Skip to:
- **[Language Basics](../03-language-basics/index.md)** - Variables, loops, functions
- **[Advanced Features](../04-advanced-features/index.md)** - Web servers, async, patterns
- **[Standard Library](../05-standard-library/index.md)** - Built-in functions

---

**Previous:** [← Why WFL?](../01-introduction/why-wfl.md) | **Next:** [Installation →](installation.md)
