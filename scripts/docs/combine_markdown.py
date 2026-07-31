#!/usr/bin/env python3
"""
WFL File Combiner

This script combines multiple markdown (.md) files from the Docs directory and/or Rust (.rs) files from the src directory into a single markdown file
and a matching text file. Original files are preserved. Table of contents is included by default.

The script outputs to both .md and .txt formats by default, unless the --no-txt option is specified.

Author: WFL Team
"""

import os
import sys
import glob
import argparse
from datetime import datetime
import re

def parse_arguments():
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(
        description="WFL File Combiner - Combine multiple files into markdown and text files"
    )
    parser.add_argument(
        "-o", "--output",
        help="Path and filename for the combined output file"
    )
    parser.add_argument(
        "-i", "--input",
        help="Directory containing files (default: based on --type)"
    )
    parser.add_argument(
        "--type",
        choices=["docs", "src", "both"],
        help="Type of files to process: 'docs' for markdown files, 'src' for Rust files, or 'both' to process both types"
    )
    parser.add_argument(
        "-r", "--recursive",
        action="store_true",
        help="Search subdirectories for files (always enabled for src)"
    )
    parser.add_argument(
        "--no-toc",
        action="store_true",
        help="Disable table of contents (enabled by default)"
    )
    parser.add_argument(
        "-s", "--sort",
        help="Sort files by: 'alpha', 'time', or comma-separated list for custom order"
    )
    parser.add_argument(
        "-l", "--header-level",
        type=int,
        default=1,
        help="Starting level for file headers (default: 1)"
    )
    parser.add_argument(
        "-p", "--separator",
        default=None,
        help="Custom separator between files (default: horizontal rule)"
    )
    parser.add_argument(
        "-a", "--all-files",
        action="store_true",
        help="Include all .md files in Docs, not just those with 'wfl-' in the name"
    )
    parser.add_argument(
        "--no-txt",
        action="store_true",
        help="Disable output to .txt format (by default outputs to both .md and .txt)"
    )
    return parser.parse_args()

def find_files(directory, extension, recursive=False, prefix_filter=False):
    """Find all files with the specified extension in the directory."""
    pattern = os.path.join(directory, "**/*" + extension if recursive else "*" + extension)
    files = glob.glob(pattern, recursive=recursive)
    if prefix_filter:
        files = [f for f in files if "wfl-" in os.path.basename(f)]
    
    # Create a list of tuples (absolute_path, relative_path)
    files_with_rel_paths = []
    for file_path in files:
        # Calculate path relative to the input directory
        rel_path = os.path.relpath(file_path, directory)
        files_with_rel_paths.append((file_path, rel_path))
    
    return files_with_rel_paths

def sort_files(files_with_rel_paths, sort_option):
    """Sort files based on the provided option."""
    if not sort_option:
        return files_with_rel_paths
    
    if sort_option.lower() == 'alpha':
        # Sort by relative path
        return sorted(files_with_rel_paths, key=lambda x: x[1])
    
    if sort_option.lower() == 'time':
        # Sort by modification time of the absolute path
        return sorted(files_with_rel_paths, key=lambda x: os.path.getmtime(x[0]))
    
    if ',' in sort_option:
        custom_order = [f.strip() for f in sort_option.split(',')]
        # Create a map using basename as key
        file_map = {os.path.basename(rel_path): (abs_path, rel_path) 
                    for abs_path, rel_path in files_with_rel_paths}
        sorted_files = []
        
        for name in custom_order:
            if name in file_map:
                sorted_files.append(file_map[name])
                file_map.pop(name)
        
        # Add any remaining files not in the custom order
        sorted_files.extend(file_map.values())
        return sorted_files
    
    return files_with_rel_paths

def extract_title(content):
    """Extract title from markdown content."""
    match = re.search(r'^#\s+(.+)$', content, re.MULTILINE)
    if match:
        return match.group(1).strip()
    return None

def generate_toc(files_with_rel_paths, type, base_level=1):
    """Generate table of contents based on file titles or filenames."""
    toc = ["# Table of Contents\n"]
    for i, (abs_path, rel_path) in enumerate(files_with_rel_paths, 1):
        try:
            with open(abs_path, 'r', encoding='utf-8') as file:
                content = file.read()
            
            if type == "docs":
                title = extract_title(content) or os.path.basename(rel_path)
            else:  # type == "src"
                # For src files, include the directory in the title
                title = rel_path.replace('\\', '/')  # Normalize path separators
            
            # Create a link-friendly title for the anchor
            link_title = title.lower().replace(' ', '-').replace('/', '-').replace('\\', '-')
            indent = "  " * (base_level - 1)
            toc.append(f"{indent}{i}. [{title}](#{i}-{link_title})")
        except Exception as e:
            print(f"Warning: Could not process {abs_path} for TOC: {e}", file=sys.stderr)
            toc.append(f"{indent}{i}. {rel_path}")
    return "\n".join(toc) + "\n\n"

def generate_combined_content(files_with_rel_paths, type, include_toc=False, header_level=1, separator=None):
    """Generate the combined content from files."""
    combined_content = []
    if include_toc:
        combined_content.append(generate_toc(files_with_rel_paths, type, header_level))
    if separator is None:
        separator = "\n\n---\n\n"
    else:
        separator = f"\n\n{separator}\n\n"
    
    for i, (abs_path, rel_path) in enumerate(files_with_rel_paths, 1):
        try:
            with open(abs_path, 'r', encoding='utf-8') as file:
                content = file.read()
            
            # Use relative path for display, especially for src files
            display_path = rel_path.replace('\\', '/')  # Normalize path separators
            
            if type == "docs":
                title = extract_title(content) or os.path.basename(rel_path)
                processed_content = re.sub(r'^#\s+.+\n', '', content, count=1, flags=re.MULTILINE).strip()
            else:  # type == "src"
                # For src files, include the directory in the title
                title = display_path
                processed_content = f"```rust\n{content}\n```"
            
            file_header = '#' * header_level + f" {i}. {title}"
            if i > 1:
                combined_content.append(separator)
            
            combined_content.append(f"**Start of file: {display_path}**")
            combined_content.append("\n\n")
            combined_content.append(file_header)
            combined_content.append("\n\n")
            combined_content.append(processed_content)
            combined_content.append("\n\n")
            combined_content.append(f"**End of file: {display_path}**")
        except Exception as e:
            print(f"Error processing file {abs_path}: {e}", file=sys.stderr)
            combined_content.append(f"# Error: Could not process {rel_path}\n\n")
    
    return "\n".join(combined_content), len(files_with_rel_paths)

def process_file_type(args, project_root, file_type, combined_dir):
    """Process files of a specific type (docs or src) and generate combined output."""
    # Set input directory based on file type if not provided
    if args.input is None:
        if file_type == "docs":
            input_dir = os.path.join(project_root, "Docs")
        else:  # file_type == "src"
            input_dir = os.path.join(project_root, "src")
    else:
        input_dir = args.input

    # Set file extension and filtering based on file type
    if file_type == "docs":
        extension = ".md"
        prefix_filter = not args.all_files
    else:  # file_type == "src"
        extension = ".rs"
        prefix_filter = False

    # Enable recursion for src always, for docs only if -r is specified
    recursive = args.recursive or (file_type == "src")

    print(f"Searching for {extension} files in {input_dir}{' and subdirectories' if recursive else ''}...")
    files = find_files(input_dir, extension, recursive, prefix_filter)
    if not files:
        print(f"No {extension} files found in {input_dir}.")
        return
    
    print(f"Found {len(files)} files.")
    if args.sort:
        print(f"Sorting files using '{args.sort}' method...")
        files = sort_files(files, args.sort)
    
    print(f"Combining {file_type} files...")
    content, count = generate_combined_content(files, file_type, not args.no_toc, args.header_level, args.separator)

    # Set output files
    if args.output is None:
        md_output_file = os.path.join(combined_dir, f"{file_type}.md")
        txt_output_file = os.path.join(combined_dir, f"{file_type}.txt")
    else:
        # For the 'both' case with custom output, append file_type to differentiate
        if args.type == "both":
            base, ext = os.path.splitext(args.output)
            md_output_file = f"{base}_{file_type}{ext}"
            txt_output_file = f"{base}_{file_type}.txt"
        else:
            md_output_file = args.output
            base, ext = os.path.splitext(args.output)
            txt_output_file = base + ".txt"
    
    # Write the combined content to the output files
    with open(md_output_file, 'w', encoding='utf-8') as f:
        f.write(content)
    
    # Write to txt file unless --no-txt is specified
    if not args.no_txt:
        with open(txt_output_file, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"Successfully combined {count} {file_type} files into {md_output_file} and {txt_output_file}.")
    else:
        print(f"Successfully combined {count} {file_type} files into {md_output_file}.")

def main():
    """Main function to orchestrate the file combining process."""
    args = parse_arguments()
    project_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

    # Prompt user for type if not specified
    if args.type is None:
        while True:
            choice = input("Do you want to process 'docs', 'src', or 'both'? ").strip().lower()
            if choice in ["docs", "src", "both"]:
                args.type = choice
                break
            else:
                print("Invalid choice. Please enter 'docs', 'src', or 'both'.")

    print(f"WFL File Combiner - Started at {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    
    combined_dir = os.path.join(project_root, "combined")
    os.makedirs(combined_dir, exist_ok=True)
    
    # Handle the "both" case specially
    if args.type == "both":
        # Process docs first
        process_file_type(args, project_root, "docs", combined_dir)
        # Then process src
        process_file_type(args, project_root, "src", combined_dir)
    else:
        # Process the specified type (docs or src)
        process_file_type(args, project_root, args.type, combined_dir)
    print(f"WFL File Combiner - Completed at {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")

if __name__ == "__main__":
    main()
