//! WFL JavaScript Runtime Library
//!
//! This module contains the JavaScript runtime library that provides
//! WFL-specific functionality in the transpiled output.

/// The WFL JavaScript runtime library for Node.js
pub const RUNTIME_NODE: &str = r#"// WFL Runtime Library for Node.js
// This runtime provides WFL-specific functionality

const fs = require('fs');
const path = require('path');
const { spawn, spawnSync } = require('child_process');

// WFL Runtime namespace
const WFL = {
  // Type checking utilities
  typeof: (value) => {
    if (value === null || value === undefined) return 'nothing';
    if (Array.isArray(value)) return 'list';
    if (value instanceof Date) return 'date';
    if (value instanceof WFL.Pattern) return 'pattern';
    if (value instanceof WFL.Container) return 'container';
    if (typeof value === 'object' && value._wfl_container) return 'container_instance';
    const t = typeof value;
    if (t === 'number') return 'number';
    if (t === 'string') return 'text';
    if (t === 'boolean') return 'boolean';
    if (t === 'function') return 'action';
    return 'object';
  },

  // Convert WFL value to string for display
  stringify: (value) => {
    if (value === null || value === undefined) return 'nothing';
    if (Array.isArray(value)) {
      return '[' + value.map(v => WFL.stringify(v)).join(', ') + ']';
    }
    if (typeof value === 'object') {
      if (value._wfl_container) {
        return `<${value._wfl_type} instance>`;
      }
      return JSON.stringify(value);
    }
    return String(value);
  },

  // Display function (print to console)
  display: (...args) => {
    console.log(args.map(WFL.stringify).join(' '));
  },

  // String operations
  text: {
    length: (s) => String(s).length,
    uppercase: (s) => String(s).toUpperCase(),
    lowercase: (s) => String(s).toLowerCase(),
    trim: (s) => String(s).trim(),
    substring: (s, start, end) => String(s).substring(start, end),
    indexOf: (s, search) => String(s).indexOf(search),
    replace: (s, search, replacement) => String(s).replace(search, replacement),
    replaceAll: (s, search, replacement) => String(s).split(search).join(replacement),
    split: (s, delimiter) => String(s).split(delimiter),
    startsWith: (s, prefix) => String(s).startsWith(prefix),
    endsWith: (s, suffix) => String(s).endsWith(suffix),
    contains: (s, search) => String(s).includes(search),
    charAt: (s, index) => String(s).charAt(index),
    concat: (...args) => args.map(String).join(''),
  },

  // Math operations
  math: {
    abs: Math.abs,
    round: Math.round,
    floor: Math.floor,
    ceil: Math.ceil,
    sqrt: Math.sqrt,
    pow: Math.pow,
    min: Math.min,
    max: Math.max,
    random: Math.random,
    randomInt: (min, max) => Math.floor(Math.random() * (max - min + 1)) + min,
    sin: Math.sin,
    cos: Math.cos,
    tan: Math.tan,
    log: Math.log,
    exp: Math.exp,
    PI: Math.PI,
    E: Math.E,
  },

  // List operations
  list: {
    create: (...items) => [...items],
    push: (list, item) => { list.push(item); return list; },
    pop: (list) => list.pop(),
    shift: (list) => list.shift(),
    unshift: (list, item) => { list.unshift(item); return list; },
    length: (list) => list.length,
    get: (list, index) => list[index],
    set: (list, index, value) => { list[index] = value; return list; },
    contains: (list, item) => list.includes(item),
    indexOf: (list, item) => list.indexOf(item),
    remove: (list, item) => {
      const idx = list.indexOf(item);
      if (idx > -1) list.splice(idx, 1);
      return list;
    },
    removeAt: (list, index) => { list.splice(index, 1); return list; },
    clear: (list) => { list.length = 0; return list; },
    slice: (list, start, end) => list.slice(start, end),
    concat: (...lists) => [].concat(...lists),
    reverse: (list) => [...list].reverse(),
    sort: (list, compareFn) => [...list].sort(compareFn),
    map: (list, fn) => list.map(fn),
    filter: (list, fn) => list.filter(fn),
    reduce: (list, fn, initial) => list.reduce(fn, initial),
    forEach: (list, fn) => list.forEach(fn),
    find: (list, fn) => list.find(fn),
    every: (list, fn) => list.every(fn),
    some: (list, fn) => list.some(fn),
    join: (list, separator) => list.join(separator || ''),
  },

  // Map/Object operations
  map: {
    create: (entries) => {
      const obj = {};
      if (entries) {
        for (const [k, v] of Object.entries(entries)) {
          obj[k] = v;
        }
      }
      return obj;
    },
    get: (map, key) => map[key],
    set: (map, key, value) => { map[key] = value; return map; },
    has: (map, key) => key in map,
    delete: (map, key) => { delete map[key]; return map; },
    keys: (map) => Object.keys(map),
    values: (map) => Object.values(map),
    entries: (map) => Object.entries(map),
    size: (map) => Object.keys(map).length,
  },

  // File operations (Node.js)
  file: {
    _validatePath: (filepath) => {
      const resolved = path.resolve(filepath);
      const normalized = path.normalize(resolved);
      // Prevent path traversal attacks by checking for suspicious patterns
      if (normalized.includes('..') || normalized !== resolved) {
        throw new Error(`Invalid file path: ${filepath}`);
      }
      return normalized;
    },
    read: (filepath) => {
      const validPath = WFL.file._validatePath(filepath);
      return fs.readFileSync(validPath, 'utf8');
    },
    write: (filepath, content) => {
      const validPath = WFL.file._validatePath(filepath);
      return fs.writeFileSync(validPath, content, 'utf8');
    },
    append: (filepath, content) => {
      const validPath = WFL.file._validatePath(filepath);
      return fs.appendFileSync(validPath, content, 'utf8');
    },
    exists: (filepath) => {
      const validPath = WFL.file._validatePath(filepath);
      return fs.existsSync(validPath);
    },
    delete: (filepath) => {
      const validPath = WFL.file._validatePath(filepath);
      return fs.unlinkSync(validPath);
    },
    copy: (src, dest) => {
      const validSrc = WFL.file._validatePath(src);
      const validDest = WFL.file._validatePath(dest);
      return fs.copyFileSync(validSrc, validDest);
    },
    move: (src, dest) => {
      const validSrc = WFL.file._validatePath(src);
      const validDest = WFL.file._validatePath(dest);
      return fs.renameSync(validSrc, validDest);
    },
    size: (filepath) => {
      const validPath = WFL.file._validatePath(filepath);
      return fs.statSync(validPath).size;
    },
    isFile: (filepath) => {
      const validPath = WFL.file._validatePath(filepath);
      return fs.existsSync(validPath) && fs.statSync(validPath).isFile();
    },
    isDirectory: (filepath) => {
      const validPath = WFL.file._validatePath(filepath);
      return fs.existsSync(validPath) && fs.statSync(validPath).isDirectory();
    },
  },

  // Directory operations (Node.js)
  directory: {
    create: (dirpath) => {
      const validPath = WFL.file._validatePath(dirpath);
      return fs.mkdirSync(validPath, { recursive: true });
    },
    delete: (dirpath) => {
      const validPath = WFL.file._validatePath(dirpath);
      return fs.rmSync(validPath, { recursive: true, force: true });
    },
    list: (dirpath) => {
      const validPath = WFL.file._validatePath(dirpath);
      return fs.readdirSync(validPath);
    },
    listRecursive: (dirpath, extensions) => {
      const validPath = WFL.file._validatePath(dirpath);
      const results = [];
      const walk = (dir) => {
        const files = fs.readdirSync(dir);
        for (const file of files) {
          const filepath = path.join(dir, file);
          const stat = fs.statSync(filepath);
          if (stat.isDirectory()) {
            walk(filepath);
          } else {
            if (!extensions || extensions.length === 0 ||
                extensions.some(ext => filepath.endsWith(ext))) {
              results.push(filepath);
            }
          }
        }
      };
      walk(validPath);
      return results;
    },
    exists: (dirpath) => {
      const validPath = WFL.file._validatePath(dirpath);
      return fs.existsSync(validPath) && fs.statSync(validPath).isDirectory();
    },
  },

  // Date/Time operations
  time: {
    now: () => Date.now(),
    today: () => {
      const d = new Date();
      return new Date(d.getFullYear(), d.getMonth(), d.getDate());
    },
    format: (date, format) => {
      const d = date instanceof Date ? date : new Date(date);
      const pad = (n) => String(n).padStart(2, '0');
      return format
        .replace('YYYY', d.getFullYear())
        .replace('MM', pad(d.getMonth() + 1))
        .replace('DD', pad(d.getDate()))
        .replace('HH', pad(d.getHours()))
        .replace('mm', pad(d.getMinutes()))
        .replace('ss', pad(d.getSeconds()));
    },
    parse: (str) => new Date(str),
    year: (date) => (date instanceof Date ? date : new Date(date)).getFullYear(),
    month: (date) => (date instanceof Date ? date : new Date(date)).getMonth() + 1,
    day: (date) => (date instanceof Date ? date : new Date(date)).getDate(),
    hours: (date) => (date instanceof Date ? date : new Date(date)).getHours(),
    minutes: (date) => (date instanceof Date ? date : new Date(date)).getMinutes(),
    seconds: (date) => (date instanceof Date ? date : new Date(date)).getSeconds(),
    milliseconds: (date) => (date instanceof Date ? date : new Date(date)).getMilliseconds(),
  },

  // Process operations (Node.js)
  process: {
    execute: (command, args) => {
      const result = spawnSync(command, args || [], { encoding: 'utf8' });
      if (result.error) throw result.error;
      if (result.status !== 0) {
        const error = new Error(`Command failed with exit code ${result.status}`);
        error.code = result.status;
        error.stderr = result.stderr;
        throw error;
      }
      return result.stdout;
    },
    spawn: (command, args) => {
      const proc = spawn(command, args || [], { stdio: 'pipe' });
      return {
        _process: proc,
        pid: proc.pid,
        stdout: '',
        stderr: '',
        exitCode: null,
        running: true,
      };
    },
    kill: (proc) => {
      if (proc._process) proc._process.kill();
      proc.running = false;
    },
    isRunning: (proc) => proc.running,
  },

  // Pattern matching
  Pattern: class Pattern {
    constructor(regex) {
      this.regex = regex instanceof RegExp ? regex : new RegExp(regex);
    }
    match(text) {
      return this.regex.test(text);
    }
    find(text) {
      const match = text.match(this.regex);
      return match ? match[0] : null;
    }
    findAll(text) {
      const globalRegex = new RegExp(this.regex.source, 'g');
      return text.match(globalRegex) || [];
    }
    replace(text, replacement) {
      return text.replace(this.regex, replacement);
    }
    replaceAll(text, replacement) {
      const globalRegex = new RegExp(this.regex.source, 'g');
      return text.replace(globalRegex, replacement);
    }
    split(text) {
      return text.split(this.regex);
    }
  },

  // Container (class) base
  Container: class Container {
    constructor() {
      this._wfl_container = true;
      this._wfl_type = this.constructor.name;
    }
  },

  // HTTP operations (Node.js)
  http: {
    get: async (url, timeout = 10000) => {
      const https = url.startsWith('https') ? require('https') : require('http');
      return new Promise((resolve, reject) => {
        const timeoutId = setTimeout(() => {
          reject(new Error(`Request timeout after ${timeout}ms`));
        }, timeout);

        const req = https.get(url, (res) => {
          clearTimeout(timeoutId);
          
          // Check HTTP status code
          if (res.statusCode < 200 || res.statusCode >= 400) {
            reject(new Error(`HTTP ${res.statusCode}: ${res.statusMessage}`));
            return;
          }

          let data = '';
          res.on('data', chunk => data += chunk);
          res.on('end', () => resolve(data));
          res.on('error', reject);
        });

        req.on('error', (err) => {
          clearTimeout(timeoutId);
          reject(err);
        });
        req.setTimeout(timeout, () => {
          req.destroy();
          reject(new Error(`Request timeout after ${timeout}ms`));
        });
      });
    },
    post: async (url, data, timeout = 10000) => {
      const https = url.startsWith('https') ? require('https') : require('http');
      const urlObj = new URL(url);
      const postData = typeof data === 'string' ? data : JSON.stringify(data);
      return new Promise((resolve, reject) => {
        const timeoutId = setTimeout(() => {
          reject(new Error(`Request timeout after ${timeout}ms`));
        }, timeout);

        const req = https.request({
          hostname: urlObj.hostname,
          port: urlObj.port,
          path: urlObj.pathname + urlObj.search,
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'Content-Length': Buffer.byteLength(postData),
          },
        }, (res) => {
          clearTimeout(timeoutId);
          
          // Check HTTP status code
          if (res.statusCode < 200 || res.statusCode >= 400) {
            let errorBody = '';
            res.on('data', chunk => errorBody += chunk);
            res.on('end', () => {
              reject(new Error(`HTTP ${res.statusCode}: ${res.statusMessage} - ${errorBody}`));
            });
            return;
          }

          let body = '';
          res.on('data', chunk => body += chunk);
          res.on('end', () => resolve(body));
          res.on('error', reject);
        });

        req.on('error', (err) => {
          clearTimeout(timeoutId);
          reject(err);
        });
        req.setTimeout(timeout, () => {
          req.destroy();
          reject(new Error(`Request timeout after ${timeout}ms`));
        });
        
        req.write(postData);
        req.end();
      });
    },
  },

  // Utility functions
  sleep: (ms) => new Promise(resolve => setTimeout(resolve, ms)),
  exit: (code) => process.exit(code || 0),
};

// Make WFL available globally in Node.js
if (typeof module !== 'undefined' && module.exports) {
  module.exports = WFL;
}
if (typeof global !== 'undefined') {
  global.WFL = WFL;
}
"#;

/// The WFL JavaScript runtime library for browsers
pub const RUNTIME_BROWSER: &str = r#"// WFL Runtime Library for Browsers
// This runtime provides WFL-specific functionality

const WFL = {
  // Type checking utilities
  typeof: (value) => {
    if (value === null || value === undefined) return 'nothing';
    if (Array.isArray(value)) return 'list';
    if (value instanceof Date) return 'date';
    if (value instanceof WFL.Pattern) return 'pattern';
    if (value instanceof WFL.Container) return 'container';
    if (typeof value === 'object' && value._wfl_container) return 'container_instance';
    const t = typeof value;
    if (t === 'number') return 'number';
    if (t === 'string') return 'text';
    if (t === 'boolean') return 'boolean';
    if (t === 'function') return 'action';
    return 'object';
  },

  // Convert WFL value to string for display
  stringify: (value) => {
    if (value === null || value === undefined) return 'nothing';
    if (Array.isArray(value)) {
      return '[' + value.map(v => WFL.stringify(v)).join(', ') + ']';
    }
    if (typeof value === 'object') {
      if (value._wfl_container) {
        return `<${value._wfl_type} instance>`;
      }
      return JSON.stringify(value);
    }
    return String(value);
  },

  // Display function (print to console)
  display: (...args) => {
    console.log(args.map(WFL.stringify).join(' '));
  },

  // String operations
  text: {
    length: (s) => String(s).length,
    uppercase: (s) => String(s).toUpperCase(),
    lowercase: (s) => String(s).toLowerCase(),
    trim: (s) => String(s).trim(),
    substring: (s, start, end) => String(s).substring(start, end),
    indexOf: (s, search) => String(s).indexOf(search),
    replace: (s, search, replacement) => String(s).replace(search, replacement),
    replaceAll: (s, search, replacement) => String(s).split(search).join(replacement),
    split: (s, delimiter) => String(s).split(delimiter),
    startsWith: (s, prefix) => String(s).startsWith(prefix),
    endsWith: (s, suffix) => String(s).endsWith(suffix),
    contains: (s, search) => String(s).includes(search),
    charAt: (s, index) => String(s).charAt(index),
    concat: (...args) => args.map(String).join(''),
  },

  // Math operations
  math: {
    abs: Math.abs,
    round: Math.round,
    floor: Math.floor,
    ceil: Math.ceil,
    sqrt: Math.sqrt,
    pow: Math.pow,
    min: Math.min,
    max: Math.max,
    random: Math.random,
    randomInt: (min, max) => Math.floor(Math.random() * (max - min + 1)) + min,
    sin: Math.sin,
    cos: Math.cos,
    tan: Math.tan,
    log: Math.log,
    exp: Math.exp,
    PI: Math.PI,
    E: Math.E,
  },

  // List operations
  list: {
    create: (...items) => [...items],
    push: (list, item) => { list.push(item); return list; },
    pop: (list) => list.pop(),
    shift: (list) => list.shift(),
    unshift: (list, item) => { list.unshift(item); return list; },
    length: (list) => list.length,
    get: (list, index) => list[index],
    set: (list, index, value) => { list[index] = value; return list; },
    contains: (list, item) => list.includes(item),
    indexOf: (list, item) => list.indexOf(item),
    remove: (list, item) => {
      const idx = list.indexOf(item);
      if (idx > -1) list.splice(idx, 1);
      return list;
    },
    removeAt: (list, index) => { list.splice(index, 1); return list; },
    clear: (list) => { list.length = 0; return list; },
    slice: (list, start, end) => list.slice(start, end),
    concat: (...lists) => [].concat(...lists),
    reverse: (list) => [...list].reverse(),
    sort: (list, compareFn) => [...list].sort(compareFn),
    map: (list, fn) => list.map(fn),
    filter: (list, fn) => list.filter(fn),
    reduce: (list, fn, initial) => list.reduce(fn, initial),
    forEach: (list, fn) => list.forEach(fn),
    find: (list, fn) => list.find(fn),
    every: (list, fn) => list.every(fn),
    some: (list, fn) => list.some(fn),
    join: (list, separator) => list.join(separator || ''),
  },

  // Map/Object operations
  map: {
    create: (entries) => {
      const obj = {};
      if (entries) {
        for (const [k, v] of Object.entries(entries)) {
          obj[k] = v;
        }
      }
      return obj;
    },
    get: (map, key) => map[key],
    set: (map, key, value) => { map[key] = value; return map; },
    has: (map, key) => key in map,
    delete: (map, key) => { delete map[key]; return map; },
    keys: (map) => Object.keys(map),
    values: (map) => Object.values(map),
    entries: (map) => Object.entries(map),
    size: (map) => Object.keys(map).length,
  },

  // Date/Time operations
  time: {
    now: () => Date.now(),
    today: () => {
      const d = new Date();
      return new Date(d.getFullYear(), d.getMonth(), d.getDate());
    },
    format: (date, format) => {
      const d = date instanceof Date ? date : new Date(date);
      const pad = (n) => String(n).padStart(2, '0');
      return format
        .replace('YYYY', d.getFullYear())
        .replace('MM', pad(d.getMonth() + 1))
        .replace('DD', pad(d.getDate()))
        .replace('HH', pad(d.getHours()))
        .replace('mm', pad(d.getMinutes()))
        .replace('ss', pad(d.getSeconds()));
    },
    parse: (str) => new Date(str),
    year: (date) => (date instanceof Date ? date : new Date(date)).getFullYear(),
    month: (date) => (date instanceof Date ? date : new Date(date)).getMonth() + 1,
    day: (date) => (date instanceof Date ? date : new Date(date)).getDate(),
    hours: (date) => (date instanceof Date ? date : new Date(date)).getHours(),
    minutes: (date) => (date instanceof Date ? date : new Date(date)).getMinutes(),
    seconds: (date) => (date instanceof Date ? date : new Date(date)).getSeconds(),
    milliseconds: (date) => (date instanceof Date ? date : new Date(date)).getMilliseconds(),
  },

  // Pattern matching
  Pattern: class Pattern {
    constructor(regex) {
      this.regex = regex instanceof RegExp ? regex : new RegExp(regex);
    }
    match(text) {
      return this.regex.test(text);
    }
    find(text) {
      const match = text.match(this.regex);
      return match ? match[0] : null;
    }
    findAll(text) {
      const globalRegex = new RegExp(this.regex.source, 'g');
      return text.match(globalRegex) || [];
    }
    replace(text, replacement) {
      return text.replace(this.regex, replacement);
    }
    replaceAll(text, replacement) {
      const globalRegex = new RegExp(this.regex.source, 'g');
      return text.replace(globalRegex, replacement);
    }
    split(text) {
      return text.split(this.regex);
    }
  },

  // Container (class) base
  Container: class Container {
    constructor() {
      this._wfl_container = true;
      this._wfl_type = this.constructor.name;
    }
  },

  // HTTP operations (Browser - using fetch)
  http: {
    get: async (url, timeout = 10000) => {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), timeout);
      
      try {
        const response = await fetch(url, {
          signal: controller.signal
        });
        clearTimeout(timeoutId);
        
        if (!response.ok) {
          const errorText = await response.text();
          throw new Error(`HTTP ${response.status}: ${response.statusText} - ${errorText}`);
        }
        
        return response.text();
      } catch (error) {
        clearTimeout(timeoutId);
        if (error.name === 'AbortError') {
          throw new Error(`Request timeout after ${timeout}ms`);
        }
        throw error;
      }
    },
    post: async (url, data, timeout = 10000) => {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), timeout);
      
      try {
        const response = await fetch(url, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: typeof data === 'string' ? data : JSON.stringify(data),
          signal: controller.signal
        });
        clearTimeout(timeoutId);
        
        if (!response.ok) {
          const errorText = await response.text();
          throw new Error(`HTTP ${response.status}: ${response.statusText} - ${errorText}`);
        }
        
        return response.text();
      } catch (error) {
        clearTimeout(timeoutId);
        if (error.name === 'AbortError') {
          throw new Error(`Request timeout after ${timeout}ms`);
        }
        throw error;
      }
    },
  },

  // Utility functions
  sleep: (ms) => new Promise(resolve => setTimeout(resolve, ms)),
  exit: (code) => { throw new Error(`Program exited with code ${code || 0}`); },
};

// Make WFL available globally
window.WFL = WFL;
"#;

/// Returns the runtime library for the specified target
pub fn get_runtime(target: super::TranspilerTarget) -> &'static str {
    match target {
        super::TranspilerTarget::Node => RUNTIME_NODE,
        super::TranspilerTarget::Browser => RUNTIME_BROWSER,
        super::TranspilerTarget::Universal => RUNTIME_NODE, // Default to Node.js version
    }
}
