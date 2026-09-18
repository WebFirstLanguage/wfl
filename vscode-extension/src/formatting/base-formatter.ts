import * as vscode from 'vscode';

/**
 * BaseFormatter provides independent formatting functionality for WFL
 * without requiring the WFL toolchain to be installed.
 */
export class BaseFormatter implements vscode.DocumentFormattingEditProvider, vscode.DocumentRangeFormattingEditProvider {
  /**
   * Format the entire document
   */
  public provideDocumentFormattingEdits(
    document: vscode.TextDocument,
    options: vscode.FormattingOptions
  ): vscode.TextEdit[] {
    return this.formatDocument(document, options);
  }

  /**
   * Format a specific range in the document
   */
  public provideDocumentRangeFormattingEdits(
    document: vscode.TextDocument,
    range: vscode.Range,
    options: vscode.FormattingOptions
  ): vscode.TextEdit[] {
    // Extract the text in the range
    const text = document.getText(range);
    
    // Format just this range
    const lines = text.split(/\r?\n/);
    const formattedLines = this.formatLines(lines, options);
    const formattedText = formattedLines.join('\n');
    
    // If nothing changed, return empty array
    if (text === formattedText) {
      return [];
    }
    
    return [vscode.TextEdit.replace(range, formattedText)];
  }

  /**
   * Format an entire document
   */
  private formatDocument(
    document: vscode.TextDocument,
    options: vscode.FormattingOptions
  ): vscode.TextEdit[] {
    const text = document.getText();
    const lines = text.split(/\r?\n/);
    
    const formattedLines = this.formatLines(lines, options);
    const formattedText = formattedLines.join('\n');
    
    // If nothing changed, return empty array
    if (text === formattedText) {
      return [];
    }
    
    const fullRange = new vscode.Range(
      0, 0,
      document.lineCount - 1,
      document.lineAt(document.lineCount - 1).text.length
    );
    
    return [vscode.TextEdit.replace(fullRange, formattedText)];
  }

  /**
   * Format an array of lines
   */
  private formatLines(
    lines: string[],
    options: vscode.FormattingOptions
  ): string[] {
    // Track indentation level as we process lines
    let indentLevel = 0;
    const tabSize = options.tabSize;
    const insertSpaces = options.insertSpaces;
    const indent = insertSpaces ? ' '.repeat(tabSize) : '\t';
    
    // Process each line
    return lines.map((line) => {
      // Trim trailing whitespace
      const trimmedLine = line.trimRight();
      
      // Skip empty lines or comment-only lines
      if (trimmedLine === '' || trimmedLine.trim().startsWith('//') || trimmedLine.trim().startsWith('/*')) {
        return trimmedLine;
      }

      // Check if this line should decrease the indent level before indenting
      // (for 'end', 'otherwise', 'else', etc.)
      if (this.isDecrementIndentLine(trimmedLine)) {
        indentLevel = Math.max(0, indentLevel - 1);
      }
      
      // Apply current indentation
      const indentedLine = indent.repeat(indentLevel) + trimmedLine.trimLeft();
      
      // Check if this line should increase the indent level for the next line
      // (for lines ending with ':')
      if (this.isIncrementIndentLine(trimmedLine)) {
        indentLevel++;
      }
      
      // Format operators with spaces
      return this.formatOperators(indentedLine);
    });
  }

  /**
   * Check if a line should increase the indent level
   */
  private isIncrementIndentLine(line: string): boolean {
    // Check for block start patterns (ending with ':')
    return /:\s*$/.test(line) && !this.isCommentLine(line);
  }

  /**
   * Check if a line should decrease the indent level
   */
  private isDecrementIndentLine(line: string): boolean {
    // Check for block end patterns like "end if", "end action", "otherwise", etc.
    const matches = line.match(/^\s*(end\s+\w+|otherwise|else|when)\b/);
    return !!matches && !this.isCommentLine(line);
  }

  /**
   * Check if a line is a comment line
   */
  private isCommentLine(line: string): boolean {
    const trimmed = line.trim();
    return trimmed.startsWith('//') || trimmed.startsWith('/*');
  }

  /**
   * Format operators with proper spacing
   */
  private formatOperators(line: string): string {
    // Regular expression to add spaces around operators
    // but preserve spaces in strings
    
    // Handle arithmetic operators
    let formatted = line
      .replace(/([^"'])\+([^"'])/g, '$1 + $2')
      .replace(/([^"'])-([^"'])/g, '$1 - $2')
      .replace(/([^"'])\*([^"'])/g, '$1 * $2')
      .replace(/([^"'])\/([^"'])/g, '$1 / $2');
    
    // Handle comparison operators
    formatted = formatted
      .replace(/([^"'])=([^"'])/g, '$1 = $2')
      .replace(/([^"'])<([^"'])/g, '$1 < $2')
      .replace(/([^"'])>([^"'])/g, '$1 > $2');
    
    // Handle natural language operators
    formatted = formatted
      .replace(/\b(is|not|and|or)\b/g, ' $1 ')
      .replace(/\s{2,}/g, ' '); // Replace multiple spaces with a single space
    
    return formatted;
  }
}
