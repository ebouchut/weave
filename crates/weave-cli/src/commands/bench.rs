use std::time::Instant;

use weave_core::entity_merge;

/// Run merge benchmarks comparing weave's entity-level merge against
/// git's line-level merge (simulated via diffy).
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("weave merge benchmark");
    println!("=====================\n");

    let scenarios = vec![
        Scenario {
            name: "Different functions modified",
            description: "Two agents modify different functions in the same file",
            file_path: "app.ts",
            base: r#"import { config } from './config';

export function processData(input: string): string {
    return input.trim();
}

export function validateInput(input: string): boolean {
    return input.length > 0;
}

export function formatOutput(data: string): string {
    return `Result: ${data}`;
}
"#,
            ours: r#"import { config } from './config';

export function processData(input: string): string {
    const cleaned = input.trim();
    console.log("Processing:", cleaned);
    return cleaned.toUpperCase();
}

export function validateInput(input: string): boolean {
    return input.length > 0;
}

export function formatOutput(data: string): string {
    return `Result: ${data}`;
}
"#,
            theirs: r#"import { config } from './config';

export function processData(input: string): string {
    return input.trim();
}

export function validateInput(input: string): boolean {
    if (!input) return false;
    return input.length > 0 && input.length < 1000;
}

export function formatOutput(data: string): string {
    return `Result: ${data}`;
}
"#,
        },
        Scenario {
            name: "Different class methods modified",
            description: "Two agents modify different methods in the same class",
            file_path: "service.ts",
            base: r#"export class UserService {
    getUser(id: string): User {
        return this.db.find(id);
    }

    createUser(data: UserData): User {
        return this.db.create(data);
    }

    deleteUser(id: string): void {
        this.db.delete(id);
    }

    listUsers(): User[] {
        return this.db.findAll();
    }
}
"#,
            ours: r#"export class UserService {
    getUser(id: string): User {
        const cached = this.cache.get(id);
        if (cached) return cached;
        const user = this.db.find(id);
        this.cache.set(id, user);
        return user;
    }

    createUser(data: UserData): User {
        return this.db.create(data);
    }

    deleteUser(id: string): void {
        this.db.delete(id);
    }

    listUsers(): User[] {
        return this.db.findAll();
    }
}
"#,
            theirs: r#"export class UserService {
    getUser(id: string): User {
        return this.db.find(id);
    }

    createUser(data: UserData): User {
        if (!data.email) throw new Error("email required");
        if (!data.name) throw new Error("name required");
        const user = this.db.create(data);
        this.events.emit("user.created", user);
        return user;
    }

    deleteUser(id: string): void {
        this.db.delete(id);
    }

    listUsers(): User[] {
        return this.db.findAll();
    }
}
"#,
        },
        Scenario {
            name: "Both add different imports",
            description: "Two agents add different imports — commutative merge",
            file_path: "imports.ts",
            base: r#"import { foo } from './foo';
import { bar } from './bar';

export function main() {
    return foo() + bar();
}
"#,
            ours: r#"import { foo } from './foo';
import { bar } from './bar';
import { baz } from './baz';

export function main() {
    return foo() + bar();
}
"#,
            theirs: r#"import { foo } from './foo';
import { bar } from './bar';
import { qux } from './qux';

export function main() {
    return foo() + bar();
}
"#,
        },
        Scenario {
            name: "Class: getUser + createUser (4 methods)",
            description: "Bigger class — both agents edit different methods among 4",
            file_path: "big-service.ts",
            base: r#"export class DataService {
    fetch(id: string): Data {
        return this.api.get(id);
    }

    transform(data: Data): Output {
        return { value: data.raw };
    }

    validate(input: Input): boolean {
        return input.value != null;
    }

    save(output: Output): void {
        this.db.insert(output);
    }
}
"#,
            ours: r#"export class DataService {
    fetch(id: string): Data {
        const cached = this.cache.get(id);
        if (cached) return cached;
        const result = this.api.get(id);
        this.cache.set(id, result);
        return result;
    }

    transform(data: Data): Output {
        return { value: data.raw };
    }

    validate(input: Input): boolean {
        return input.value != null;
    }

    save(output: Output): void {
        this.db.insert(output);
    }
}
"#,
            theirs: r#"export class DataService {
    fetch(id: string): Data {
        return this.api.get(id);
    }

    transform(data: Data): Output {
        const cleaned = this.sanitize(data.raw);
        return { value: cleaned, timestamp: Date.now() };
    }

    validate(input: Input): boolean {
        return input.value != null;
    }

    save(output: Output): void {
        this.db.insert(output);
    }
}
"#,
        },
        Scenario {
            name: "One adds function, other modifies existing",
            description: "Agent A adds a new function, Agent B modifies an existing one",
            file_path: "utils.ts",
            base: r#"export function helper() {
    return "help";
}
"#,
            ours: r#"export function helper() {
    return "help";
}

export function newFeature() {
    return "new feature by agent A";
}
"#,
            theirs: r#"export function helper() {
    console.log("helper called");
    return "improved help";
}
"#,
        },
        Scenario {
            name: "Adjacent function changes (stress test)",
            description: "Two agents modify adjacent functions — tests merge precision",
            file_path: "adjacent.ts",
            base: r#"export function alpha() {
    return "a";
}
export function beta() {
    return "b";
}
"#,
            ours: r#"export function alpha() {
    return "A";
}
export function beta() {
    return "b";
}
"#,
            theirs: r#"export function alpha() {
    return "a";
}
export function beta() {
    return "B";
}
"#,
        },
        Scenario {
            name: "Python: different methods in same class",
            description: "Two agents modify different methods in a Python class",
            file_path: "service.py",
            base: r#"class DataProcessor:
    def load(self, path):
        with open(path) as f:
            return f.read()

    def transform(self, data):
        return data.strip()

    def save(self, data, path):
        with open(path, 'w') as f:
            f.write(data)
"#,
            ours: r#"class DataProcessor:
    def load(self, path):
        import json
        with open(path) as f:
            return json.load(f)

    def transform(self, data):
        return data.strip()

    def save(self, data, path):
        with open(path, 'w') as f:
            f.write(data)
"#,
            theirs: r#"class DataProcessor:
    def load(self, path):
        with open(path) as f:
            return f.read()

    def transform(self, data):
        cleaned = data.strip()
        return cleaned.lower()

    def save(self, data, path):
        with open(path, 'w') as f:
            f.write(data)
"#,
        },
        Scenario {
            name: "Python: adjacent methods (harder)",
            description: "Two agents modify adjacent methods in Python class — diffy often fails",
            file_path: "service.py",
            base: r#"class Service:
    def create(self, data):
        return self.db.insert(data)

    def read(self, id):
        return self.db.find(id)

    def update(self, id, data):
        self.db.update(id, data)

    def delete(self, id):
        self.db.remove(id)
"#,
            ours: r#"class Service:
    def create(self, data):
        if not data:
            raise ValueError("empty")
        result = self.db.insert(data)
        self.log.info(f"Created {result.id}")
        return result

    def read(self, id):
        return self.db.find(id)

    def update(self, id, data):
        self.db.update(id, data)

    def delete(self, id):
        self.db.remove(id)
"#,
            theirs: r#"class Service:
    def create(self, data):
        return self.db.insert(data)

    def read(self, id):
        cached = self.cache.get(id)
        if cached:
            return cached
        result = self.db.find(id)
        self.cache.set(id, result)
        return result

    def update(self, id, data):
        self.db.update(id, data)

    def delete(self, id):
        self.db.remove(id)
"#,
        },
        Scenario {
            name: "TS: both add exports at end",
            description: "Both agents add different named exports — a very common pattern",
            file_path: "exports.ts",
            base: r#"export function alpha(): string {
    return "alpha";
}

export function beta(): string {
    return "beta";
}
"#,
            ours: r#"export function alpha(): string {
    return "alpha";
}

export function beta(): string {
    return "beta";
}

export function gamma(): string {
    return "gamma - from agent A";
}
"#,
            theirs: r#"export function alpha(): string {
    return "alpha";
}

export function beta(): string {
    return "beta";
}

export function delta(): string {
    return "delta - from agent B";
}
"#,
        },
        Scenario {
            name: "Reformat vs modify (whitespace-aware)",
            description: "One agent reformats, other makes real change — whitespace detection",
            file_path: "format.ts",
            base: r#"export function process(data: string): string {
    return data.trim();
}

export function validate(input: string): boolean {
    return input.length > 0;
}
"#,
            ours: r#"export function process(data: string): string {
      return data.trim();
}

export function validate(input: string): boolean {
      return input.length > 0;
}
"#,
            theirs: r#"export function process(data: string): string {
    const cleaned = data.trim();
    return cleaned.toUpperCase();
}

export function validate(input: string): boolean {
    return input.length > 0;
}
"#,
        },
        // --- NEW: scenarios targeting known git false-conflict patterns ---
        Scenario {
            name: "Both add new functions at end of file",
            description: "Both agents append different functions — git conflicts on insertion point",
            file_path: "append.ts",
            base: r#"export function existing() {
    return "exists";
}
"#,
            ours: r#"export function existing() {
    return "exists";
}

export function featureA() {
    return "added by agent A";
}
"#,
            theirs: r#"export function existing() {
    return "exists";
}

export function featureB() {
    return "added by agent B";
}
"#,
        },
        Scenario {
            name: "Both add methods to class at end",
            description: "Both agents add different methods to end of class — git conflicts",
            file_path: "class-append.ts",
            base: r#"export class Router {
    get(path: string) {
        return this.routes.get(path);
    }
}
"#,
            ours: r#"export class Router {
    get(path: string) {
        return this.routes.get(path);
    }

    post(path: string, handler: Handler) {
        this.routes.set(path, handler);
    }
}
"#,
            theirs: r#"export class Router {
    get(path: string) {
        return this.routes.get(path);
    }

    delete(path: string) {
        this.routes.delete(path);
    }
}
"#,
        },
        Scenario {
            name: "Rust: both add different use statements",
            description: "Both agents add different use imports — commutative merge",
            file_path: "lib.rs",
            base: r#"use std::io;
use std::fs;

pub fn process() {
    println!("processing");
}
"#,
            ours: r#"use std::io;
use std::fs;
use std::path::PathBuf;

pub fn process() {
    println!("processing");
}
"#,
            theirs: r#"use std::io;
use std::fs;
use std::collections::HashMap;

pub fn process() {
    println!("processing");
}
"#,
        },
        Scenario {
            name: "Python: both add different imports",
            description: "Both agents add different Python imports — commutative merge",
            file_path: "app.py",
            base: r#"import os
import sys

def main():
    print("hello")
"#,
            ours: r#"import os
import sys
import json

def main():
    print("hello")
"#,
            theirs: r#"import os
import sys
import pathlib

def main():
    print("hello")
"#,
        },
        Scenario {
            name: "Class: modify method + add new method",
            description: "Agent A modifies a method, Agent B adds a new one — tests mixed changes",
            file_path: "mixed.ts",
            base: r#"export class Cache {
    get(key: string): string | null {
        return this.store[key] || null;
    }

    set(key: string, value: string): void {
        this.store[key] = value;
    }
}
"#,
            ours: r#"export class Cache {
    get(key: string): string | null {
        const val = this.store[key];
        if (!val) return null;
        this.hits++;
        return val;
    }

    set(key: string, value: string): void {
        this.store[key] = value;
    }
}
"#,
            theirs: r#"export class Cache {
    get(key: string): string | null {
        return this.store[key] || null;
    }

    set(key: string, value: string): void {
        this.store[key] = value;
    }

    delete(key: string): boolean {
        if (this.store[key]) {
            delete this.store[key];
            return true;
        }
        return false;
    }
}
"#,
        },
        Scenario {
            name: "Both add functions between existing ones",
            description: "Both insert different functions in the middle — git conflicts on position",
            file_path: "insert-middle.ts",
            base: r#"export function first() {
    return 1;
}

export function last() {
    return 99;
}
"#,
            ours: r#"export function first() {
    return 1;
}

export function middleA() {
    return "from agent A";
}

export function last() {
    return 99;
}
"#,
            theirs: r#"export function first() {
    return 1;
}

export function middleB() {
    return "from agent B";
}

export function last() {
    return 99;
}
"#,
        },
    ];

    let mut total_weave_clean = 0;
    let mut total_git_clean = 0;
    let total_scenarios = scenarios.len();

    for scenario in &scenarios {
        print!("  {:<50}", scenario.name);

        // Run weave merge
        let start = Instant::now();
        let weave_result = entity_merge(
            scenario.base,
            scenario.ours,
            scenario.theirs,
            scenario.file_path,
        );
        let weave_time = start.elapsed();

        // Run git-style merge (line-level via diffy)
        let start = Instant::now();
        let git_result = diffy::merge(scenario.base, scenario.ours, scenario.theirs);
        let git_time = start.elapsed();

        let weave_clean = weave_result.is_clean();
        let git_clean = git_result.is_ok();

        if weave_clean {
            total_weave_clean += 1;
        }
        if git_clean {
            total_git_clean += 1;
        }

        let status = match (weave_clean, git_clean) {
            (true, false) => "WEAVE WINS",
            (true, true) => "both clean",
            (false, true) => "git wins",
            (false, false) => "both conflict",
        };

        println!(
            "weave: {:>5}us ({:<9} {}) | git: {:>5}us ({}) | {}",
            weave_time.as_micros(),
            if weave_clean { "clean" } else { "CONFLICT" },
            weave_result.stats.confidence(),
            git_time.as_micros(),
            if git_clean { "clean" } else { "CONFLICT" },
            status,
        );
    }

    println!("\n--- Summary ---");
    println!(
        "weave: {}/{} clean merges ({:.0}%)",
        total_weave_clean,
        total_scenarios,
        total_weave_clean as f64 / total_scenarios as f64 * 100.0,
    );
    println!(
        "git:   {}/{} clean merges ({:.0}%)",
        total_git_clean,
        total_scenarios,
        total_git_clean as f64 / total_scenarios as f64 * 100.0,
    );

    let improvement = total_weave_clean as i32 - total_git_clean as i32;
    if improvement > 0 {
        println!(
            "\nweave resolved {} additional merge(s) that git could not.",
            improvement,
        );
        println!(
            "False conflict reduction: {:.0}%",
            if total_scenarios > total_git_clean {
                improvement as f64 / (total_scenarios - total_git_clean) as f64 * 100.0
            } else {
                0.0
            },
        );
    }

    Ok(())
}

struct Scenario {
    name: &'static str,
    #[allow(dead_code)]
    description: &'static str,
    file_path: &'static str,
    base: &'static str,
    ours: &'static str,
    theirs: &'static str,
}
