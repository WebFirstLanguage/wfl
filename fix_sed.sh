#!/bin/bash
sed -i 's/                } => {/                } if \!is_snake_case(name) => {/g' src/linter/mod.rs
sed -i '/if !is_snake_case(name) {/,/^[[:space:]]*}/!b;//!d;/if !is_snake_case(name) {/d' src/linter/mod.rs
