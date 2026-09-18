import * as vscode from 'vscode';
import * as cp from 'child_process';
import * as path from 'path';
import * as fs from 'fs';
import * as os from 'os';

/**
 * WflFormatter integrates with the WFL CLI to provide enhanced formatting
 * when the WFL toolchain is available.
 */
export class WflFormatter implements vscode.DocumentFormattingEditProvider, vscode.DocumentRangeFormattingEditProvider {
  private wflPath: string;

  constructor(wflPath: string) {
    this.wflPath = wflPath;
  }

  /**
   * Format the entire document using the WFL CLI
   */
  public async provideDocumentFormattingEdits(
    document: vscode.TextDocument,
    _options: vscode.FormattingOptions // Unused parameter
  ): Promise<vscode.TextEdit[]> {
    return this.formatDocument(document);
  }

  /**
   * Format a specific range in the document
   * Note: WFL CLI doesn't directly support range formatting,
   * so we extract the range, format it, and reintegrate.
   */
  public async provideDocumentRangeFormattingEdits(
    document: vscode.TextDocument,
    range: vscode.Range,
    _options: vscode.FormattingOptions // Unused parameter
  ): Promise<vscode.TextEdit[]> {
    // Extract the text in the range
    const text = document.getText(range);
    
    // Create a temporary file for the range
    const tempFile = path.join(os.tmpdir(), `wfl-format-${Date.now()}.wfl`);
    fs.writeFileSync(tempFile, text);
    
    try {
      // Format the temporary file
      const formattedText = await this.formatFile(tempFile);
      
      // If nothing changed, return empty array
      if (text === formattedText) {
        return [];
      }
      
      return [vscode.TextEdit.replace(range, formattedText)];
    } catch (error) {
      vscode.window.showErrorMessage(`WFL formatting failed: ${String(error)}`);
      return [];
    } finally {
      // Clean up the temporary file
      try {
        fs.unlinkSync(tempFile);
      } catch {
        // Ignore cleanup errors
      }
    }
  }

  /**
   * Format an entire document
   */
  private async formatDocument(
    document: vscode.TextDocument
  ): Promise<vscode.TextEdit[]> {
    // For unsaved documents, create a temporary file
    if (document.isUntitled) {
      return this.formatUntitledDocument(document);
    }
    
    try {
      // Format the file directly
      const formattedText = await this.formatFile(document.fileName);
      
      const text = document.getText();
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
    } catch (error) {
      vscode.window.showErrorMessage(`WFL formatting failed: ${String(error)}`);
      return [];
    }
  }

  /**
   * Format an untitled document by creating a temporary file
   */
  private async formatUntitledDocument(
    document: vscode.TextDocument
  ): Promise<vscode.TextEdit[]> {
    const text = document.getText();
    
    // Create a temporary file
    const tempFile = path.join(os.tmpdir(), `wfl-format-${Date.now()}.wfl`);
    fs.writeFileSync(tempFile, text);
    
    try {
      // Format the temporary file
      const formattedText = await this.formatFile(tempFile);
      
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
    } catch (error) {
      vscode.window.showErrorMessage(`WFL formatting failed: ${String(error)}`);
      return [];
    } finally {
      // Clean up the temporary file
      try {
        fs.unlinkSync(tempFile);
      } catch {
        // Ignore cleanup errors
      }
    }
  }

  /**
   * Execute the WFL CLI to format a file
   */
  private async formatFile(
    filePath: string
  ): Promise<string> {
    return new Promise<string>((resolve, reject) => {
      // Build the WFL CLI command
      const args = [
        '--lint',
        '--fix',
        '--diff', // Use diff to get the formatted output
        filePath
      ];
      
      // Add configuration options if specified
      const config = vscode.workspace.getConfiguration('wfl.format');
      if (config.has('indentSize')) {
        args.push('--indent-size', config.get('indentSize', 4).toString());
      }
      
      if (config.has('maxLineLength')) {
        args.push('--max-line-length', config.get('maxLineLength', 80).toString());
      }
      
      // Execute the WFL CLI
      const process = cp.spawn(this.wflPath, args);
      
      let stdout = '';
      let stderr = '';
      
      process.stdout.on('data', (data: Buffer | string) => {
        stdout += data.toString();
      });
      
      process.stderr.on('data', (data: Buffer | string) => {
        stderr += data.toString();
      });
      
      process.on('close', (code: number | null) => {
        if (code !== 0) {
          reject(stderr || `Process exited with code ${code}`);
          return;
        }
        
        // Parse the diff output to get the formatted text
        try {
          const formatted = this.extractFormattedTextFromDiff(stdout, fs.readFileSync(filePath, 'utf8'));
          resolve(formatted);
        } catch (error) {
          reject(`Failed to parse WFL formatter output: ${error}`);
        }
      });
      
      process.on('error', (err: Error) => {
        reject(`Failed to start WFL formatter: ${err.message}`);
      });
    });
  }

  /**
   * Extract the formatted text from a unified diff output
   */
  private extractFormattedTextFromDiff(diffOutput: string, originalText: string): string {
    // If there's no diff output, the file was already formatted
    if (!diffOutput.trim()) {
      return originalText;
    }
    
    // Simple implementation - in a real extension, we would use a proper diff parser
    // For now, just extract the formatted text sections (lines starting with '+')
    // and skip the context and removal lines (lines starting with '-' or ' ')
    
    // This is a simplified approach that works for basic diffs
    // In a production extension, use a proper diff parser library
    const lines = diffOutput.split('\n');
    const formattedLines: string[] = [];
    
    // Skip the diff header lines
    let headerDone = false;
    
    for (const line of lines) {
      // Skip until we find a line starting with '+++'
      if (!headerDone) {
        if (line.startsWith('+++')) {
          headerDone = true;
        }
        continue;
      }
      
      // Skip diff metadata lines (starting with @@)
      if (line.startsWith('@@')) {
        continue;
      }
      
      // Include added lines (without the '+' prefix)
      if (line.startsWith('+')) {
        formattedLines.push(line.substring(1));
      }
      // Skip removed lines and context lines
    }
    
    return formattedLines.join('\n');
  }
}
