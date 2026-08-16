**gitctx: Complete Step-by-Step Implementation Plan with Pitfalls, Mitigations, and Verification** 

This implementation plan defines the construction of **gitctx** , a high-fidelity indexing engine designed to provide **Large Language Models** (LLMs) with verifiable, structured facts from **Git** history. Unlike standard **RAG** (Retrieval-Augmented Generation) systems that index raw text, `gitctx` treats code history as a temporal graph of functional changes. 

# **Project Invariants** 

To ensure scientific validity and operational reliability, the following invariants are enforced mechanically via CI and runtime checks: 

|ID|Invariant|Enforcement Mechanism|
|---|---|---|
|**I1**|No network/LLM calls during<br>indexingor retrieval.|`unshare -n`in CI; mock HTTP<br>layer in tests.|
|**I2**|Deterministic indexing (Repo<br>+ Confg = Byte-identical<br>state).|Double-index comparison<br>test (Part 4.2).|
|**I3**|Incremental index≡Full<br>index.<br>|State equivalence test on<br>commit ranges (Part 4.3).|
|**I4**|Every fact carries a verifable<br>citation.|SQL`NOT NULL`constraints on<br>`commit_sha`and`path`.|
|**I5**|No LLM-derived text in<br>embeddinginputs.|Physical table isolation for<br>`explain_cache`.|
|**I6**|Evaluation corpus is pinned<br>bycommit SHA.|`eval/corpus.lock.toml`<br>validation.|
|**I7**|Reported numbers include<br>baseline and uncertainty.|Mandatory template<br>columns in`cargo xtask eval`.|
|**I8**|Traversal semantics are<br>explicit.|`index_state.traversal`<br>metadata feld.|



# **− 1. Phase 1: Foundation and Corpus Selection** 

# **1.1 Repository Scaffolding** 

- **Build:** A Rust workspace containing `gitctx-core` (extraction), `gitctx-index` (storage), `gitctxquery` (retrieval), and `gitctx-eval` . 

- **Pitfalls:** Dependency bloat and slow compile times for the eval harness. 

- **Solutions:** Use a shared `core` crate; avoid `tokio` in the indexer if possible to keep it a simple CLI tool. 

- **Verification:** `cargo test` passes in under 10 s for the base workspace. 

# **1.2 Synthetic Fixture Generation** 

- **Build:** A `tests/fixtures/mkrepo.rs` utility to programmatically generate tiny Git repositories. 

- **Pitfalls:** Real-world Git edge cases (evil merges, renames, submodules) are hard to find in 

- small repos. 

Ablation Study: Retriever vs. Document Template Performance 

|D1|0.681|||0.681|0.90|
|---|---|---|---|---|---|
|D2<br>Ww<br>ov<br>=]&Qi<br>oe<br>or<br>D3<br>5<br>£<br>=<br>VY°<br>a<br>D4|0.681<br>0.654<br>0.687|0.662<br>0.711|Be|0.692<br>0.686<br>st Performance<br>0.942<br>0.708|0.85<br>8<br>a<br>0.80 =<br>@<br>°<br>0.75|
|D5||0.678|0.708|0.723|0.70|
||BM25|MiniLM|BGE-M3<br>Retrievers|RRF<br>Code-Model||



- **Build:** A **libgit2** walker using `first-parent` logic. For each commit, parse changed blobs using **tree-sitter** . 

- **Pitfalls:** Syntactically broken code in historical commits. 

- **Solutions:** Implement a “degraded chunk” fallback that treats the whole file as one chunk if 

- parsing fails. 

- **Verification:** Log `parse_success_rate` ; target > 98% for mature repos. 

# **3.2 The Lineage Algorithm** 

- **Build:** An ordered matching engine to track functions across time. 

- **Algorithm Steps:** 

1. Exact match (Path + Qualified Name). 

2. **MinHash** Move detection (Qualified Name + fingerprint). 

3. Rename detection (Path + MinHash). 

4. Fuzzy match (MinHash threshold > 0.7). 

- **Verification:** Synthetic mutation testing. Apply 10 transformations (M1-M10) to a file and verify the engine tracks the identity. 

```
// Rust Sketch: Lineage Fingerprinting
```

```
pubstructFunctionFingerprint {
pub qualified_name: String,
pub minhash: Vec<u64>,
pub signature_hash: u64,
}
```

```
fncalculate_minhash(tokens: &[String]) -> Vec<u64> {
```

   - `// Implementation of K-permuted MinHash for Jaccard similarity` 

- `// ...` 

- `}` 

# **4. Phase 2: Indexing and Data Modeling** 

# **4.1 SQLite Schema (DDL)** The database is the source of truth for all temporal facts. 

```
-- Core Schema for gitctx
CREATETABLEindex_state (
    schema_version INTEGERPRIMARY KEY,
    config_hash TEXTNOTNULL,
    embedding_model_sha256 TEXTNOTNULL,
    traversal_mode TEXTDEFAULT'first-parent'
```

```
);
```

```
CREATETABLEfunction_lineage (
    lineage_id INTEGERPRIMARY KEY,
    first_seen_commit TEXTNOTNULL,
    last_seen_commit TEXTNOTNULL,
    canonical_name TEXTNOTNULL
);
```

```
CREATETABLEfunction_touches (
    lineage_id INTEGERREFERENCES function_lineage(lineage_id),
    commit_hash TEXTNOTNULL,
pathTEXTNOTNULL,
    start_line INTEGER,
```

```
    end_line INTEGER,
    match_confidence REAL,
PRIMARY KEY (lineage_id, commit_hash)
);
```

```
CREATETABLEcall_edges (
    caller_lineage_id INTEGER,
    callee_lineage_id INTEGER,
    valid_from_commit TEXT,
PRIMARY KEY (caller_lineage_id, callee_lineage_id, valid_from_commit)
);
CREATETABLEexplain_cache (
    fact_hash TEXTPRIMARY KEY,
    llm_explanation TEXTNOTNULL
);
```

# **4.2 Vector Store and RRF Fusion** 

- **Build:** A memory-mapped 𝑓32 vector store for cosine similarity. 

- **Fusion:** Combine **Tantivy** (BM25) scores with vector scores using Reciprocal Rank Fusion. 

- **Verification:** Determinism test. Index the same repo twice; the `tantivy` segments and SQLite WAL-cleared files must be bit-identical. 

# **5. Phase 3: Agent Surface (CLI & MCP)** 

# **5.1 MCP Tool Implementation** 

- **Build:** A **Model Context Protocol** server exposing tools: `get_function_history` , `find_related_changes` , `query_why` . 

- **Pitfalls:** LLMs exceeding token limits with large diffs. 

- **Solutions:** Implement a `compact` mode that returns only commit summaries and line ranges, and a `full` mode for specific deep-dives. 

- **Verification:** MCP inspector tool validates the JSON-RPC schema. 

- **5.2 Output Envelope** Every response must include metadata to ensure the agent knows the context of the facts. 

```
{
```

```
"gitctx_version": "0.1.0",
"config_hash": "a1b2c3d4...",
```

```
"results": [...],
```

```
"warnings": ["Parse error in commit 8f2e1: fallback to raw diff used"]
}
```

# **6. Evaluation and Risk Management** 

# **6.1 Risk Register** 



<!-- Start of picture text -->
Verification Loop Framework<br>1. Synthetic Fixtures<br>Data Generation & Setup<br></> Fal<br>Re-evaluate / Retry<br>Pass<br>2. Lineage Oracle<br>Traceability & Provenance<br>Fail Revision / Debug<br>Pass<br>3. Ablation Grid<br>Systematic Perturbation<br>E> Fal<br>Pass<br>4. Ground Truth Harvest<br>Consensus & Metric Extraction<br>Ei Fail<br>5. Final Eval Report<br>Verification Complete<br><!-- End of picture text -->

- **Days 6-15:** Phase 0 (Python Ablation). **Gate:** If RRF doesn’t beat BM25 by 15%, pivot to pure keyword search. 

- **Days 16-35:** Phase 1 & 2 (Rust Core & Indexing). 

- **Days 36-45:** Phase 3 & 4 (MCP & Final Eval). **Cut Order:** Drop `doc-RAG` first, then drop `callgraph` depth, then drop `blame-attribution` . 

This plan ensures that every component of `gitctx` is built on a foundation of measurable accuracy, providing a reliable “memory” for AI-assisted software engineering. 

