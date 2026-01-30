import os
import sys

def sync_agents_md(check_only=False):
    # Determine the project root relative to this script
    script_dir = os.path.dirname(os.path.abspath(__file__))
    project_root = os.path.dirname(script_dir)

    claude_md_path = os.path.join(project_root, "CLAUDE.md")
    agents_md_path = os.path.join(project_root, "AGENTS.md")

    if not os.path.exists(claude_md_path):
        print(f"Error: {claude_md_path} not found.")
        sys.exit(1)

    with open(claude_md_path, "r", encoding="utf-8") as f:
        lines = f.readlines()

    start_index = -1
    end_index = len(lines)

    for i, line in enumerate(lines):
        if line.startswith("## ") and start_index == -1:
            start_index = i
        if line.startswith("## Claude Code Hooks"):
            end_index = i
            break

    if start_index == -1:
         print(f"Error: Could not find first section in {claude_md_path}")
         sys.exit(1)

    # Extract content
    content_lines = lines[start_index:end_index]

    # Strip trailing empty lines from extracted content
    while content_lines and content_lines[-1].strip() == "":
        content_lines.pop()

    # Construct new content
    new_content = "# Repository Guidelines\n\n" + "".join(content_lines)

    # Ensure exactly one newline at the end
    new_content = new_content.rstrip() + "\n"

    if check_only:
        if not os.path.exists(agents_md_path):
            print(f"Error: {agents_md_path} missing.")
            sys.exit(1)

        with open(agents_md_path, "r", encoding="utf-8") as f:
            current_content = f.read()

        if current_content != new_content:
            print(f"Error: {agents_md_path} is not in sync with {claude_md_path}.")
            sys.exit(1)
        else:
            print(f"Success: {agents_md_path} is in sync.")
    else:
        with open(agents_md_path, "w", encoding="utf-8") as f:
            f.write(new_content)
        print(f"Successfully synchronized {agents_md_path} with {claude_md_path}.")

if __name__ == "__main__":
    check_mode = "--check" in sys.argv
    sync_agents_md(check_mode)
