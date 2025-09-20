import * as path from 'path';
import Mocha from 'mocha';
import * as glob from 'glob';

export function run(): Promise<void> {
  // Create the mocha test
  const mocha = new Mocha({
    ui: 'bdd',
    color: true,
    timeout: 30000 // Increase timeout for LSP tests
  });

  const testsRoot = path.resolve(__dirname, '..');

  return new Promise<void>((resolve, reject) => {
    glob.sync('**/**.test.js', { cwd: testsRoot }).forEach(file => {
      mocha.addFile(path.resolve(testsRoot, file));
    });

    try {
      // Run the mocha test
      mocha.run((failures: number) => {
        if (failures > 0) {
          reject(new Error(`${failures} tests failed.`));
        } else {
          resolve();
        }
      });
    } catch (err) {
      console.error(err);
      reject(err);
    }
  });
}