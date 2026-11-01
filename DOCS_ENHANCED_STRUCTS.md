# Design Doc: Enhanced Struct Documentation (Methods & Trait Impls)

## Problem
Currently, when `rdocdown doc <struct_path>` is run, it returns the top-level documentation for the struct, but omits the documentation for its methods and trait implementations. This forces agents to guess method names or run multiple queries to find how to use the struct.

## Proposed Solution
Modify `rdocdown` to include the full documentation for a struct's inherent methods and trait implementations by default, unless specific flags are used.

### 1. Unified Documentation Extraction
The current `html_to_markdown` logic in `src/cmd/doc.rs` extracts the `main#main-content` or `.rustdoc` sections. This includes the top-level documentation, but the HTML structure for methods and traits is more complex.

- **Methods**: Extract sections with IDs like `implementations`, `method.<name>`, and their associated `docblock` children.
- **Trait Implementations**: Extract the `trait-implementations` section, including the documentation for each trait's methods if they are overridden.

### 2. Implementation Approach
- **Section Selection**: Instead of just grabbing the entire `main-content`, `rdocdown` will specifically identify and include:
    - The struct's definition and top-level doc.
    - The `Implementations` section (inherent methods).
    - The `Trait Implementations` section.
- **Formatting**: Use Markdown headers (e.g., `## Methods`, `## Trait Implementations`) to organize the output for better readability by both humans and agents.

### 3. Granularity & Control
To prevent the output from becoming too large for very complex structs (like `std::vec::Vec`), introduce flags:
- `--summary-only`: (New) Show only the struct definition and top-level doc.
- `--methods-only`: (New) Show only the inherent methods.
- `--traits-only`: (New) Show only trait implementations.
- `--signatures-only`: (Existing) Show only the function/method signatures without docstrings.

## Technical Details
- **HTML Parsing**: Use `scraper` to select specific sub-sections (`#implementations`, `#trait-implementations`) and ensure their `docblock` children are included in the final Markdown conversion.
- **Deduplication**: Ensure that if a member is specifically requested (e.g., `rdocdown doc std::vec::Vec::push`), only that specific method's documentation is returned, rather than the entire struct.
