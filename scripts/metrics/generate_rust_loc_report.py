#!/usr/bin/env python3
"""
Rust Line Counter

This script counts the lines of Rust code in the src directory and generates a report.
It supports counting total lines, code lines (excluding comments and blank lines),
and provides a breakdown by directory and file.
"""

import os
import re
from collections import defaultdict
from datetime import datetime

def count_lines_in_file(file_path):
    """
    Count lines in a Rust file, categorizing them as code, comments, or blank.
    
    Args:
        file_path: Path to the Rust file
        
    Returns:
        tuple: (total_lines, code_lines, comment_lines, blank_lines)
    """
    total_lines = 0
    code_lines = 0
    comment_lines = 0
    blank_lines = 0
    
    in_block_comment = False
    
    try:
        with open(file_path, 'r', encoding='utf-8') as file:
            for line in file:
                total_lines += 1
                line = line.strip()
                
                # Empty line
                if not line:
                    blank_lines += 1
                    continue
                
                # Handle block comments
                if in_block_comment:
                    comment_lines += 1
                    if "*/" in line:
                        in_block_comment = False
                        # Check if there's code after the end of the block comment
                        code_after = line.split("*/", 1)[1].strip()
                        if code_after and not code_after.startswith("//"):
                            code_lines += 1
                    continue
                
                # Line starts with block comment
                if line.startswith("/*"):
                    comment_lines += 1
                    if "*/" not in line:
                        in_block_comment = True
                    continue
                
                # Line is a single-line comment
                if line.startswith("//"):
                    comment_lines += 1
                    continue
                
                # Otherwise, it's a code line
                code_lines += 1
                
                # Check for block comment in the middle of a line
                if "/*" in line and "*/" not in line[line.index("/*"):]:
                    in_block_comment = True
    
    except Exception as e:
        print(f"Error processing {file_path}: {e}")
        return 0, 0, 0, 0
    
    return total_lines, code_lines, comment_lines, blank_lines

def format_path(path, base_path):
    """Format the file path relative to the base path"""
    rel_path = os.path.relpath(path, base_path)
    return rel_path

def count_rust_lines(directory):
    """
    Recursively count lines in all Rust files in the given directory
    
    Args:
        directory: Base directory to start the search
        
    Returns:
        tuple: (stats_by_file, stats_by_dir, total_stats)
    """
    stats_by_file = {}
    stats_by_dir = defaultdict(lambda: [0, 0, 0, 0])  # [total, code, comments, blank]
    total_stats = [0, 0, 0, 0]  # [total, code, comments, blank]
    
    for root, _, files in os.walk(directory):
        for file in files:
            if file.endswith('.rs'):
                file_path = os.path.join(root, file)
                file_stats = count_lines_in_file(file_path)
                
                # Store file statistics
                stats_by_file[format_path(file_path, os.path.dirname(directory))] = file_stats
                
                # Update directory statistics
                dir_path = format_path(root, os.path.dirname(directory))
                for i in range(4):
                    stats_by_dir[dir_path][i] += file_stats[i]
                    total_stats[i] += file_stats[i]
    
    return stats_by_file, stats_by_dir, total_stats

def generate_report(stats_by_file, stats_by_dir, total_stats):
    """
    Generate a formatted report of the line count statistics
    
    Args:
        stats_by_file: Dictionary mapping file paths to their statistics
        stats_by_dir: Dictionary mapping directory paths to their statistics
        total_stats: List containing the total statistics
        
    Returns:
        str: Formatted report
    """
    report = []
    report.append("=" * 80)
    report.append("RUST CODE LINE COUNT REPORT")
    report.append(f"Generated on: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    report.append("=" * 80)
    
    # Overall statistics
    report.append("\nOVERALL STATISTICS:")
    report.append(f"Total files processed: {len(stats_by_file)}")
    report.append(f"Total lines: {total_stats[0]}")
    report.append(f"Code lines: {total_stats[1]} ({total_stats[1]/total_stats[0]*100:.1f}%)")
    report.append(f"Comment lines: {total_stats[2]} ({total_stats[2]/total_stats[0]*100:.1f}%)")
    report.append(f"Blank lines: {total_stats[3]} ({total_stats[3]/total_stats[0]*100:.1f}%)")
    
    # Directory statistics
    report.append("\nLINES BY DIRECTORY:")
    sorted_dirs = sorted(stats_by_dir.items(), key=lambda x: x[1][0], reverse=True)
    report.append(f"{'Directory':<40} {'Total':<10} {'Code':<10} {'Comments':<10} {'Blank':<10}")
    report.append("-" * 80)
    
    for dir_path, dir_stats in sorted_dirs:
        report.append(f"{dir_path:<40} {dir_stats[0]:<10} {dir_stats[1]:<10} {dir_stats[2]:<10} {dir_stats[3]:<10}")
    
    # File statistics
    report.append("\nLINES BY FILE:")
    sorted_files = sorted(stats_by_file.items(), key=lambda x: x[1][0], reverse=True)
    report.append(f"{'File':<50} {'Total':<10} {'Code':<10} {'Comments':<10} {'Blank':<10}")
    report.append("-" * 80)
    
    for file_path, file_stats in sorted_files:
        report.append(f"{file_path:<50} {file_stats[0]:<10} {file_stats[1]:<10} {file_stats[2]:<10} {file_stats[3]:<10}")
    
    return "\n".join(report)

def save_report_to_markdown(report, output_path):
    """
    Save the report to a markdown file
    
    Args:
        report: String containing the report
        output_path: Path to save the markdown file
    """
    markdown_lines = []
    markdown_lines.append("# Rust Code Line Count Report")
    markdown_lines.append(f"*Generated on: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}*")
    markdown_lines.append("")
    
    # Convert the plain text report to markdown
    in_section = False
    in_table = False
    
    for line in report.split('\n'):
        if line.startswith('='):
            continue
        elif line.startswith("RUST CODE LINE COUNT REPORT") or line.startswith("Generated on:"):
            continue
        elif line.strip() == '':
            markdown_lines.append("")
        elif line.startswith("OVERALL STATISTICS:"):
            markdown_lines.append("## Overall Statistics")
        elif line.startswith("LINES BY DIRECTORY:"):
            markdown_lines.append("## Lines by Directory")
            in_section = True
        elif line.startswith("LINES BY FILE:"):
            markdown_lines.append("## Lines by File")
            in_section = True
        elif in_section and line.startswith('-'):
            markdown_lines.append("| " + " | ".join(prev_line.split()) + " |")
            markdown_lines.append("| " + " | ".join(["---"] * len(prev_line.split())) + " |")
            in_table = True
        elif in_table:
            parts = []
            remaining = line
            for width in [40, 10, 10, 10, 10]:  # Adjust these based on your format
                part = remaining[:width].strip()
                remaining = remaining[width:]
                parts.append(part)
            markdown_lines.append("| " + " | ".join(parts) + " |")
        else:
            prev_line = line
            markdown_lines.append(line)
    
    with open(output_path, 'w', encoding='utf-8') as f:
        f.write("\n".join(markdown_lines))

def main():
    """Main function to run the script"""
    src_directory = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "src")
    
    print(f"Counting lines of Rust code in {src_directory}...")
    stats_by_file, stats_by_dir, total_stats = count_rust_lines(src_directory)
    
    # Generate and print the report
    report = generate_report(stats_by_file, stats_by_dir, total_stats)
    print(report)
    
    # Save the report as a markdown file in the Docs directory
    docs_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "Docs")
    output_path = os.path.join(docs_dir, "rust_loc_report.md")
    save_report_to_markdown(report, output_path)
    print(f"\nReport saved to {output_path}")

if __name__ == "__main__":
    main()
