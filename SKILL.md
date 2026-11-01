---
description: Retrieve Rust documentation formatted as Markdown
---

# rdocdown - Rust Documentation Markdown Viewer

When working on Rust codebases, you may need to look up documentation for standard library types, third-party crates, or local workspace crates. Instead of searching through source code using `grep` or navigating raw HTML files, you should use `rdocdown` to retrieve the documentation natively formatted as Markdown.

> [!info]
> Important: rdocdown only resolves crates that have been added as dependencies. After adding new dependencies, run `cargo doc` to update the index.

## Usage

You can use the `rdocdown` CLI tool to fetch documentation, search documentation, or view source code for any given Rust path.

If `rdocdown` is available in the current environment, you can run it as follows:

```bash
# If installed globally/system-wide or in ~/.cargo/bin:
/home/armiller/.cargo/bin/rdocdown doc std::vec::Vec
# OR
rdocdown doc std::vec::Vec

# If running directly from the project directory containing rdocdown:
cargo run --bin rdocdown -- doc std::vec::Vec
```

### Examples

**Standard Library:**
```bash
rdocdown doc std::string::String
rdocdown doc std::fs::File
```

**Third-Party Crates (e.g., loaded from `docs.rs` or local target dir):**
```bash
rdocdown doc serde::Deserialize
rdocdown doc clap::Parser
```

**Modules (to see top-level crate/module documentation):**
```bash
rdocdown doc std::collections
```

**Members (to see specific method, variant, or field documentation):**
```bash
rdocdown doc std::vec::Vec::len
rdocdown doc smithay::desktop::Window::surface_under
```

**Signatures Only (to view struct/enum definitions and method signatures without verbose descriptions):**
```bash
rdocdown doc -s std::vec::Vec
```

**Viewing Source Code:**
To fetch the actual source code of an item (extracted from the `[src]` link). This automatically extracts the specific lines for the requested member rather than the whole file:
```bash
rdocdown source std::vec::Vec::len
```

**Searching Local Documentation:**
To search the locally generated documentation for a specific term:
```bash
rdocdown search "search_query"
# Optionally search within a specific crate's documentation:
rdocdown search "search_query" -c serde
```

## How it works

Behind the scenes, the tool does the following to intelligently find the best documentation:
1. **Sysroot Lookup**: For standard library crates (`std`, `core`, `alloc`), it finds the local Rust system root via `rustc --print sysroot` and reads the prebuilt HTML documentation.
2. **Local Workspace**: For local packages and dependencies, it runs `cargo doc --document-private-items` and searches the `target/doc` directory.
3. **Docs.rs Fallback**: If it cannot find the documentation locally, it gracefully falls back to fetching it directly from `https://docs.rs`.

After locating the correct HTML page, it parses the main content block and converts it into standard Markdown for easy reading in terminal outputs.

## Note for Agents
- Use `rdocdown` instead of manually grepping `cargo` source registry caches when you need to understand the public API, trait implementations, or struct methods of a Rust type.
- For types with massive documentation pages, use the `-s` or `--signatures-only` flag with `rdocdown doc` to quickly scan available methods and trait implementations.
- `rdocdown source` intelligently extracts just the relevant lines of code for the requested item.
- `rdocdown search` helps you find types and visually highlights the crate name in the results.
- The tool implements an automatic freshness check by hashing `Cargo.lock`, making subsequent documentation lookups nearly instant.
