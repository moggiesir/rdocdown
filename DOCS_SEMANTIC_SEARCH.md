# Design Doc: Semantic Search via Embeddings

## Problem
Currently, `rdocdown` uses brute-force keyword matching for searches. This requires users (and agents) to know the exact names or terms used in the documentation. If an agent is looking for a way to "store unique items" but doesn't know the term `HashSet`, a keyword search for "unique storage" might fail or return irrelevant results.

## Proposed Solution
Implement semantic search by generating vector embeddings for documentation items. This allows the search command to find relevant results based on meaning rather than just character matching.

### 1. Indexing Strategy
A new `index` command will be added to crawl documentation and generate embeddings.

- **Content Extraction**: Reuse `html_to_markdown` logic to convert rustdoc HTML into clean text.
- **Granularity**: Embeddings should be generated at the level of individual items (structs, enums, traits, functions) to allow precise deep-linking.
- **Model**: Use a local embedding model (e.g., `all-MiniLM-L6-v2` via `fastembed-rs`) for privacy and speed.

### 2. Storage
Store the embeddings in a local vector index file (e.g., `~/.cache/rdocdown/index.bin`). 
- **Metadata**: Each vector will be associated with a `RustPath` (e.g., `std::collections::HashSet`) and a source URL/path.
- **Incremental Updates**: The indexer should track file modification times to only re-index changed documentation.

### 3. Search Workflow
When a user runs `rdocdown search <query>`:
1.  **Check for Index**: If an index exists, generate an embedding for the `<query>`.
2.  **Vector Search**: Perform a cosine similarity search against the local index.
3.  **Rank & Display**: Return the top $N$ results, sorted by similarity score.
4.  **Fallback**: If no index is found or if similarity scores are low, fall back to the existing keyword search.

## Technical Details
- **New Dependency**: `fastembed` for local embedding generation.
- **Storage Format**: Simple binary format or a lightweight vector library like `instant-distance`.
- **Automatic Indexing**: Trigger indexing automatically when `rdocdown doc` generates new documentation for a local crate.
