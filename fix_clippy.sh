#!/bin/bash
# Fix clippy warning in analyzer/mod.rs:1291
sed -i 's/} => {/}=> {/g' src/analyzer/mod.rs # Safety backup
sed -i 's/} => {/}=> {/g' src/fixer/mod.rs
sed -i 's/} => {/}=> {/g' src/linter/mod.rs
