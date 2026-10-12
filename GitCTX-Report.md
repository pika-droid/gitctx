#### **A PROJECT REPORT** 

#### **on** 

**Gitctx: A Temporal Context-Aware Retrieval Engine for Evolutionary Codebase Analysis** 

#### **Submitted to** 

### **KIIT Deemed to be University** 

**In Partial Fulfilment of the Requirement for the Award of** 

#### **BACHELOR’S DEGREE IN** 

#### **COMPUTER SCIENCE AND ENGINEERING** 

#### **BY** 

**Aayush Beura** 2305185 **Ashmit Mandal** 2305201 

UNDER THE GUIDANCE OF 

**Prof. Ms. Krutika Verma** 



<!-- Start of picture text -->
re<br><!-- End of picture text -->

###### **SCHOOL OF COMPUTER ENGINEERING** 

**KALINGA INSTITUTE OF INDUSTRIAL TECHNOLOGY BHUBANESWAR, ODISHA – 751024** 

**October 2026** 

#### A PROJECT REPORT 

on 

“Gitctx: A Temporal Context-Aware Retrieval Engine for Evolutionary Codebase Analysis” 

Submitted to 

KIIT Deemed to be University 

In Partial Fulfilment of the Requirement for the Award of 

BACHELOR’S DEGREE IN 

COMPUTER SCIENCE AND ENGINEERING 

BY 

**Aayush Beura** 2305185 **Ashmit Mandal** 2305201 

UNDER THE GUIDANCE OF 

#### **Prof. Ms. Krutika Verma** 



<!-- Start of picture text -->
re<br><!-- End of picture text -->

SCHOOL OF COMPUTER ENGINEERING 

KALINGA INSTITUTE OF INDUSTRIAL TECHNOLOGY BHUBANESWAR, ODISHA – 751024 

October 2026 

## **KIIT Deemed to be University** 

School of Computer Engineering Bhubaneswar, Odisha – 751024 



<!-- Start of picture text -->
re<br><!-- End of picture text -->

# **CERTIFICATE** 

This is to certify that the project entitled 

**“Gitctx: A Temporal Context-Aware Retrieval Engine for Evolutionary Codebase Analysis”** 

submitted by 

**Aayush Beura** 2305185 **Ashmit Mandal** 2305201 

is a record of bona fide work carried out by them, in the partial fulfilment of the requirement for the award of Degree of Bachelor of Engineering (Computer Science & Engineering) at KIIT Deemed to be university, Bhubaneswar. This work is done during year 2026 - 2027, under our guidance. 

Date:       /        / 

**Prof. Ms. Krutika Verma** 

Project Guide 

### **ACKNOWLEDGEMENTS** 

We are profoundly grateful to **Prof. Ms. Krutika Verma** of the School of Computer Engineering, KIIT Deemed to be University. Her expert guidance, invaluable feedback, and continuous encouragement were instrumental in shaping this project from its initial conception to its final implementation. Her deep technical insight and constructive reviews constantly pushed us to refine our system architecture and explore robust engineering solutions. 

We also extend our sincere thanks to the **School of Computer Engineering, KIIT Deemed to be University** , for providing an environment conducive to technical research and development. The strong academic foundation provided by our curriculum has been vital in undertaking an engineering challenge of this scope. 

Furthermore, we are deeply grateful to our **friends and family** . Their unwavering emotional support, patience, and understanding provided our primary pillar of strength during long development cycles. Many early discussions with peers helped clarify the core motivation behind making version control history accessible, reinforcing the practical value of building intelligent developer tooling. 

Finally, we express our gratitude to everyone who directly or indirectly supported us in bringing “gitctx: A Temporal Context-Aware Retrieval Engine for Evolutionary Codebase Analysis” to successful completion. 

**Aayush Beura (2305185)** 

**Ashmit Mandal (2305201)** 

### **ABSTRACT** 

Modern software repositories accumulate years of architectural decisions, design rationale, and evolutionary context within their commit histories. This wealth of information remains largely inaccessible to developers and autonomous coding assistants. Standard version control tooling provides no semantic bridge between natural language developer intent and raw unified diff syntax. Furthermore, conventional tools lose track of function identity across routine refactoring operations and fail to supply artificial intelligence models with verifiable, citationbacked historical context without relying on cloud services or non-deterministic inference. 

This project introduces **gitctx** , a local-first and fully offline Git history intelligence engine developed in Rust. The platform transforms raw repository commit streams into an indexed, query-able knowledge base using three integrated subsystems. First, a deterministic templaterendering pipeline translates function-level commit touches into structured natural language pseudo-documents for hybrid information retrieval. Second, a four-tier MinHash lineage resolution engine maintains continuous function identities across file renames, structural moves, signature adjustments, and complex extraction refactoring. Third, a statistical cochange coupling analysis engine with exponential temporal decay computes modification risk across architectural boundaries. The retrieval layer merges BM25 lexical ranking with dense semantic vector search through Reciprocal Rank Fusion, serving verifiable context to artificial intelligence agents via a Model Context Protocol server operating entirely over local standard 

input and output streams. <mark>Rigorous benchmarking across three major repositories comprising over 36,000 commits confirms an optimal lineage tracking F1 score of 1.000 against ten synthetic mutation benchmarks. The system delivers 95th-percentile search responses in under 300 milliseconds while enforcing absolute deterministic reproducibility through eight verified system invariants.</mark> 

**Key Words:** Git history mining, information retrieval, BM25, dense embeddings, reciprocal rank fusion, function lineage tracking, MinHash, co-change coupling, retrieval-augmented generation, Model Context Protocol, software repository mining, deterministic indexing 

### **TABLE OF CONTENTS** 

|1|Introdu|ction||1|
|---|---|---|---|---|
||1.1|Distri|buted Version Control Systems(DVCS)|1|
|||1.1.1|Structural Topologyand Operation Flow|2|
|||1.1.2|Storage Subsystem and Object Primitives|2|
|||1.1.3|<sup>Operational Lifecycle: Staging, Merging, and</sup><br>Graph Growth|3|
||1.2|Repos|itoryHistoryas Institutional Memory|3|
|||1.2.1|Software Archaeologyand EvolutionaryContext|4|
|||1.2.2|Collaborative Dynamics and Team Continuity|4|
|||1.2.3|The Open Problem: Historyis Rich but Inaccessible|5|
|2|Literat|ure Review|and Foundations|6|
||2.1|Minin|gSoftware Repositories(MSR)|6|
||2.2|Inform|ation Retrieval for Source Code|7|
||2.3|Hybri|d Retrieval and Reciprocal Rank Fusion|8|
||2.4|Co-Ch|ange Couplingand EvolutionaryDesign Analysis|8|
||2.5|Functi|on IdentityTrackingand Code Lineage|9|
||2.6|LLM|Groundingand the Model Context Protocol(MCP)|9|
|3|System|Design a|nd Methodology|11|
||3.1|Overv|iew and Design Invariants|11|
||3.2|Git Ex|traction Layer and Commit Traversal|14|
|||3.2.1|First-Parent Traversal Strategy|14|
|||3.2.2|Structural Differencingand Rename Detection|14|
|||3.2.3|Graceful Degradation and Fault Tolerance|15|
||3.3|Pseud|o-Document Template Rendering (D4)|15|
|||3.3.1|Credential Sanitation|16|
||3.4|Functi|on Lineage Engine|17|
||3.5|Storag|e Orchestration and Hybrid Retrieval|19|
|||3.5.1|Dual-Engine Document Indexing|19|
|||3.5.2|QueryExecution and Rank Fusion|19|
||3.6|Co-Ch|ange Couplingand Impact Analysis|21|
||3.7|Outpu|t Layer and Agent Integration|22|
|||3.7.1|Human-FacingCommand Interface|22|
|||3.7.2|Model Context Protocol(MCP)Server|22|
|4|Evalua|tion Meth|odology|23|



#### **I.   INTRODUCTION** 

Over the past three decades, software engineering has undergone a fundamental change in both scale and operational complexity. Something that began once as monolithic programs written by individual developers, has evolved into growing, collaboratively maintained codebases comprising millions of source lines of code (SLOC), cross-functional teams of dozens to hundreds of engineers, and continuous integration/continuous development (CI/CD) lifecycles spanning multiple years. Coordinating development at this scale requires robust mechanisms to support concurrent modifications, ensure build integrity, and preserve the integrity of the release baseline. In this context, modern version control architectures have become very crucial for software development. 

##### **1.1  Distributed Version Control Systems (DVCS)** 

A Distributed Version Control System (DVCS) is a category of software configuration management infrastructure in which every peer’s working copy contains an entire clone of the original repository, including its complete historical record, and revision graphs. Unlike centralized architectures that depend on a continuous network connection to a single master server for revision tracking, branching, and logging, a DVCS enables developers to execute versioning operations locally and synchronize changes across peers asynchronously. 



<!-- Start of picture text -->
s s 2 4 4C<br>woes<br>e eeeeee Gf Js. 81 | ne 2 DN,<br>= Cc £ c = Cc<br>wees eee EP YB E 2 E a<br>VET Jaep pap<br>fs) © 8 © ° °<br>Merete Working copy Working copy Working copy<br>#1 #2 #3<br><!-- End of picture text -->

Figure 1.1: Architectural model of a Distributed Version Control System illustrating local commit workflows and peer synchronization via push and pull operations. 

###### **1.1.1  Structural Topology and Operation Flow** 

As illustrated in Figure 1.1, the operational architecture of a DVCS is structured across three distinct execution layers - 

- **Workstation Layer (Working Copy):** Represents the local sandbox where files are edited directly on disk. Developers inspect, modify, and stage source code within this layer before committing state transitions. 

- **Local Storage Layer (Local Repository):** Houses a full, self-contained mirror of the repository database directly on the developer machine. Operations such as commit creation, branch switching, history inspection, and diff generation occur strictly within this tier without network overhead. The local working copy interacts with this layer through commit and update actions. 

- **Main Server Layer (Remote Upstream):** Serves as an authoritative central synchronization nexus across distributed teams. Synchronization between local repositories and the main server is decoupled from individual commit actions, taking place through bulk push and pull transfer commands. 

###### **1.1.2  Storage Subsystem and Object Primitives** 

Underneath this multi-tier architecture, Git implements a content-addressable storage mechanism structured as a directed acyclic graph (DAG). Every state transition is recorded using four core immutable object types: 

- **Blobs:** Represent the raw byte contents of individual file revisions without retaining directory paths or access permissions. 

- **Trees:** Model the hierarchical directory structure by mapping human-readable filenames to their corresponding blob or subtree object identifiers. 

- **Commits:** Act as top-level snapshot containers, binding a root tree reference to one or more parent commit identifiers alongside author metadata, committer signatures, precise timestamps, and log descriptions. 

- **Tags:** Serve as permanent references that point directly to specific commit nodes within the graph. 

Every object is assigned a unique identifier generated by a cryptographic hash function, typically SHA-1 or SHA-256, calculated directly from its payload and header. Because commit objects record the hashes of their ancestor commits, the historical timeline forms an immutable Merkle tree. Any post-hoc modification to a file or commit header fundamentally alters its hash, propagating upstream changes that break graph integrity. 

###### **1.1.3  Operational Lifecycle: Staging, Merging, and Graph Growth** 

The day-to-day lifecycle of a repository is defined by three fundamental mechanisms: 

1. **The Staging Area (Index):** A binary cache that acts as an intermediate state between the working directory and the commit database. It allows developers to construct selective, atomic commits by staging specific file chunks before serialization. 

2. **Branching and Pointers:** Branches in Git are lightweight, mutable pointers containing the 40-character hash of the latest commit in a given lineage. Creating, switching, or deleting branches carries minimal computational overhead because it manipulates pointers rather than duplicating directory trees. 

3. **Graph Reconciliation:** Integrating divergent histories requires branch reconciliation techniques. These include fast-forward updates, recursive three-way merge algorithms that calculate a common ancestor base, and rebasing operations that replay local commits sequentially onto an updated target branch. 

Through these combined layers, object models, and pointer mechanics, modern version control systems provide an audit log of a software system, tracking the complete chronological progression of source code across its entire lifecycle. 

##### **1.2  Repository History as Institutional Memory** 

While version control systems were initially designed to manage concurrent development and prevent conflicting modifications, repository history has evolved into a far more significant asset. In long-running codebases, the commit graph serves as an institutional knowledge base that preserves the context of structural and functional decisions across multiple generations of engineering teams. By binding every source modification to author identities, exact timestamps, and explanatory log messages, it creates an unbroken audit trail of design rationale that outlasts any individual contributor's tenure. 

###### **1.2.1  Software Archaeology and Evolutionary Context** 

During the lifecycle of a software product, code undergoes continuous modification driven by changing requirements, defect corrections, performance optimizations, and technical debt management. Understanding the present state of a system often requires software archaeology, which is the practice of inspecting historical changes to recover domain context and architectural intent that is not visible from reading the current source alone. A repository commit log contains multidimensional information that supports this investigative process: 

- **Temporal Provenance:** Exact timestamps record when specific logic was introduced, modified, or retired, enabling developers to reconstruct the chronological progression of architectural changes. 

- **Author Attribution:** Commits record the identity of original authors, providing insight into which contributors or sub-teams possess domain familiarity with critical and sensitive subsystems. 

- **Intent and Rationale:** Commit log messages, linked issue tracker references, and pull request metadata document the motivations, trade-offs, and external constraints that shaped each code modification. 

###### **1.2.2  Collaborative Dynamics and Team Continuity** 

In modern engineering workflows characterized by rapid CI/CD cycles and high developer mobility, human memory is inherently transient. Engineers rotate across projects or exit organizations entirely, taking unwritten architectural assumptions and design context with them. Under these conditions, the version control graph functions as the sole persistent source of truth regarding system evolution. Routine engineering activities — conducting code reviews, assessing regression risks, planning safe refactoring operations, and onboarding new contributors — depend heavily on the historical rationale preserved within the repository. The repository therefore represents not merely a technical snapshot of source files, but an evolving organizational asset that encodes the collective decision-making process of the engineering team. 

###### **1.2.3  The Open Problem: History is Rich but Inaccessible** 

Despite the richness of information embedded in a repository's commit history, this record remains largely inaccessible in practice. Three structural limitations define the open problem that motivates this work: 

- **The Cross-Modal Retrieval Gap:** Developers ask “why” questions in natural language, but Git history is stored as raw unified diffs — a syntax-heavy, line-oriented format optimized for patch application rather than semantic retrieval. No standard mechanism bridges the gap between a question such as “why was the authentication flow restructured?” and the actual diff hunks that answer it. Keyword search fails because the vocabulary of natural-language questions rarely overlaps with the specific identifier tokens in a diff. 

- **Function Identity Loss Across Refactors:** Git's history model is fundamentally filecentric. When a function is renamed, moved to a different module, extracted into a helper, or in-lined into its caller, standard tools such as git log and git blame lose track of its identity entirely. No complete, continuous lineage can be recovered for a function once its containing file is renamed or restructured, leaving the most important evolutionary histories involving significant refactoring untracked. 

- **LLM Hallucination of Repository History:** Large language models are increasingly embedded in developer workflows as coding assistants, but they have no grounded access to a project's actual commit log. When queried about historical decisions, such as - why a particular pattern was chosen, what motivated a given refactor, or which modules carry the highest change risk, these models often generate plausible but factually incorrect answers. There is currently no standardized mechanism to supply LLMs with verifiable, citation-backed facts derived deterministically from a local repository without incurring network latency or non-deterministic inference costs. 

These three limitations, namely the semantic retrieval gap, the function identity problem, and the LLM grounding deficit, collectively define the research gap that the gitctx system addresses. 

#### **II.   LITERATURE REVIEW AND FOUNDATIONS** 

The work presented in this paper sits at the intersection of several established research areas: mining software repositories, probabilistic information retrieval, hybrid rank fusion, functionlevel code identity tracking, statistical co-change coupling analysis, and the emerging problem of grounding large language models with structured factual context. Each area is surveyed below, with the goal of positioning gitctx relative to prior art and identifying the specific gaps in the literature that it fills. 

##### **2.1  Mining Software Repositories (MSR)** 

The Mining Software Repositories (MSR) research community has studied version control histories as empirical data sources for over two decades. Foundational work by Hassan and Holt [1] demonstrated that historical change data could be used to predict fault-prone modules, establishing the central principle that a file's evolutionary past is predictive of its defect-prone future. Fischer et al. [2] introduced the systematic study of evolutionary coupling, which is the tendency for files that are frequently modified together to encode implicit architectural dependencies and showed that such co-change patterns persist and strengthen across successive release cycles. 

Moser et al. [3] demonstrated that change-based metrics derived from version control outperform static code metrics for defect prediction, a finding subsequently replicated across multiple industrial settings. More recent MSR work has focused on commit message quality assessment [4], automated intent classification of commits into categories such as bug fixes, feature additions, and refactors [5], and the extraction of issue-closing links from commit trailer conventions [6]. Tools such as PyDriller [7] provide programmatic Python APIs for traversing Git history, extracting diffs, and computing file-level change metrics, forming part of the standard MSR practitioner toolkit. 

gitctx builds upon this established tradition by mining the same foundational sources of repository history, including commits, diffs, file trees, and rename events. However, it extends beyond conventional file-level statistical aggregation by introducing function-level semantic indexing, enabling a more granular representation of code evolution and historical relationships. Furthermore, gitctx incorporates a natural-language query layer that allows developers to retrieve and explore historical insights through intuitive questions, a capability not typically offered by prior MSR tools. 

##### **2.2  Information Retrieval for Source Code** 

Applying information retrieval (IR) techniques to source code has a long history in the software engineering literature. Early approaches adapted Term Frequency–Inverse Document Frequency (TF-IDF) weighting to code token distributions [8], treating identifiers, keywords, and inline comments as a document vocabulary amenable to standard vector-space retrieval. 

The introduction of BM25 [9], known as the Best Match 25, which is the probabilistic ranking function developed by Robertson and Zaragoza, significantly improved retrieval precision on bag-of-words representations by incorporating term frequency saturation and document-length normalization. The relevance score of a document _D_ given a query _Q_ containing keywords _q_ 1 _, q_ 2 _, …, qn_ is defined as: 



where _f_ ( _qi , D_ ) is the term frequency in the document, | _D_ | is the document length in words, _av gdl_ is the average document length in the text collection, and _k_ 1 and _b_ are free parameters. BM25 remains the dominant baseline across code search benchmarks [10] and forms the lexical retrieval backbone of gitctx, implemented via the Tantivy search engine with a custom codeaware tokenizer that splits on camelCase boundaries, underscores, and common programming identifier delimiters. 

The limitations of purely lexical retrieval motivated a shift toward dense semantic representations. Early neural approaches used Siamese networks and, later, transformer-based encoders to embed code and natural-language queries into a shared continuous vector space. The CodeBERT [10] and UniXcoder model families demonstrated that pre-training on bimodal code-text corpora produces embeddings that significantly improve semantic code search, particularly for queries that share no lexical overlap with the code token vocabulary. gitctx employs a smaller, locally cached sentence embedding model, BGE-Small-EN-v1.5, compiled to ONNX and executed entirely offline through ONNX Runtime, trading maximum retrieval quality for the strict offline and deterministic guarantees required by the system's core architectural invariants and operational constraints. 

##### **2.3  Hybrid Retrieval and Reciprocal Rank Fusion** 

Neither lexical BM25 nor dense embedding retrieval dominates the other across all query types. Lexical methods excel at exact identifier matching and rare domain-specific tokens; dense methods excel at paraphrase and semantic generalization across vocabulary gaps. A substantial body of work on rank fusion addresses this complementarity directly. 

Reciprocal Rank Fusion (RRF), proposed by Cormack, Clarke, and Buettcher [13], combines ranked result lists from multiple independent retrievers by summing reciprocal rank scores with a smoothing constant _k_ : 



This formulation produces a fused ranking without requiring score normalization or interretriever calibration. The smoothing constant _k_ = 60 has been empirically validated across TREC benchmarks, demonstrating robustness against outlier high-rank scores. Subsequent studies, including the BEIR zero-shot evaluation benchmark [14], have further established that RRF consistently matches or outperforms learned fusion methods when training data for a supervised combiner is unavailable, making it particularly suitable for newly initialized repositories that have not previously been indexed. 

gitctx applies RRF with _k_ = 60 to fuse BM25 and dense retrieval results at query time, supplemented by an exact-match fast path that injects function name or commit SHA matches at rank 0, guaranteeing their mathematical dominance in the final ranking. 

##### **2.4  Co-Change Coupling and Evolutionary Design Analysis** 

The statistical relationship between files that are frequently modified together — termed logical coupling or co-change coupling — was formalized by Gall, Hajek, and Jazayeri [11] and subsequently systematized in Adam Tornhill's work on evolutionary design analysis [15]. The code-maat tool operationalizes these ideas using standard association rule metrics: support (how often entities A and B change together), directional confidence _P(B | A),_ and lift, which is the ratio of observed joint probability to the expected rate under statistical independence. 

High lift values identify entity pairs that change together far more than chance would predict, flagging implicit architectural coupling absent from any static dependency graph. A welldocumented confound in co-change analysis is the presence of mass-formatting or bulk-update commits that modify large numbers of files simultaneously, injecting spurious coupling signal across otherwise unrelated modules [16]. gitctx addresses this by filtering commits that touch more than 25 files and by applying exponential time-decay weighting that down-weights historical coupling relative to recent co-change patterns. 

##### **2.5  Function Identity Tracking and Code Lineage** 

Tracking the identity of a code entity, such as a function, method, or class, throughout its evolutionary history presents a non-trivial challenge. The git log -L command supports linerange tracking within a single file but fails to maintain continuity when the containing file is renamed or the entity's implementation is moved elsewhere. Grund et al. [17] investigated the frequency and characteristics of function moves across large open-source projects, demonstrating that cross-file moves account for a significant proportion of refactoring activity and expose a systematic limitation in existing code history tracking tools. 

Locality-Sensitive Hashing (LSH) using MinHash, introduced by Broder [18], provides a computationally efficient approach for identifying near-duplicate documents without requiring exhaustive pairwise comparisons. By generating MinHash signatures from the token multisets of function bodies, different versions of a function that retain most of their token content while differing in name, signature, or file location can produce similar signatures and high estimated Jaccard similarity. gitctx implements a four-tier lineage resolution cascade comprising exact key matching, same-file MinHash, Git rename map cross-referencing, and scoped cross-file MinHash. This layered approach enables the system to reconstruct complete and deterministic chronological chains for individual functions across multiple file renames, code movements, and structural refactoring operations. 

##### **2.6  LLM Grounding and the Model Context Protocol (MCP)** 

The widespread deployment of large language models as coding assistants has introduced a new class of requirements for developer tooling. LLMs possess broad general programming knowledge but have no access to a specific project's actual Git history. Their tendency to generate plausible but factually incorrect commit attributions and historical rationale has been documented in the literature [19]. This hallucination failure mode is particularly harmful for 

historical why-questions, where a confidently stated but incorrect answer can mislead a developer into making a wrong architectural decision. 

The Retrieval-Augmented Generation (RAG) framework [12] addresses this by supplying language models with retrieved factual passages at inference time, significantly reducing hallucination rates on knowledge-intensive tasks. The Model Context Protocol (MCP) [20], introduced as a standardized interface for supplying structured tool outputs to LLMs over a stdio JSON-RPC channel, provides a practical deployment mechanism for grounding model responses with verifiable, deterministic data from external systems without requiring LLMside changes. 

gitctx applies the Retrieval-Augmented Generation (RAG) principle specifically to the domain of code history. By exposing a set of MCP tools backed by deterministic, offline retrieval mechanisms, it enables AI coding assistants to address historical why-questions using evidence traceable to specific commit SHAs, authors, and timestamps. This approach reduces hallucinations in historical code queries by grounding responses in verifiable repository data, while eliminating network dependencies and avoiding LLM inference along the retrieval path. As a result, gitctx provides a reproducible and evidence-based foundation for exploring code evolution and understanding the rationale behind past development decisions. 

#### **III.   SYSTEM DESIGN AND METHODOLOGY** 

This chapter details the architectural design and technical implementation of the gitctx engine. Building upon the theoretical foundations of software repository mining and information retrieval, the proposed system transforms raw version control history into a structured semantic knowledge base. The methodology focuses on deterministic offline processing, resilient function tracking across structural refactoring, and the integration of large language models through verifiable retrieval mechanisms. The following sections outline the core system invariants, the step-by-step data extraction pipeline, and the hybrid storage architecture required to achieve these objectives. 

##### **3.1  Overview and Design Invariants** 

The gitctx system is organized as a modular Rust workspace of six crates, each isolating a separate architectural concern: 

- **gitctx-core:** Manages repository Git extraction, abstract syntax tree (AST) parsing, the lineage tracking engine, and structured template rendering. 

- **gitctx-index:** Orchestrates local storage mechanisms, including Tantivy-based BM25 indexing and dense vector persistence. 

- **gitctx-query:** Executes hybrid search retrieval, constructs the statistical co-change graph, and performs change impact analysis. 

- **gitctx-cli:** Exposes the human-facing command-line interface for manual developer queries. 

- **gitctx-mcp:** Operates a Model Context Protocol server to facilitate seamless integration with autonomous AI agents. 

- **gitctx-eval:** Coordinates reproducible benchmarking and generates standardized evaluation reports. 

To guarantee deterministic execution, the system architecture is bound by eight mechanically enforced invariants, denoted as I1 through I8. The two most critical constraints define the operational boundaries of the software: 

1. **Invariant I1 (Zero Network Activity):** The system must execute all indexing and retrieval operations entirely offline. This constraint is rigorously validated during continuous integration (CI) workflows via strict network namespace isolation. 

2. **Invariant I2 (Deterministic Output):** The engine must produce byte-identical database states given identical repository inputs. This is verified continuously by double-indexing a test repository and asserting absolute parity between the resulting cryptographic checksums. 

3. **Invariant I3 (Incremental Equivalence):** The state produced by incrementally indexing one new commit must be identical to the state produced by performing a complete full re-index of the repository up to that commit. This prevents the incremental code path from silently diverging from the full-index path over time. 

4. **Invariant I4 (Non-Null Citations):** Every result record returned by a query operation must include complete, non-null citation metadata, specifically the commit SHA, author name, and timestamp, ensuring that each retrieved result can be traced to its corresponding historical source within the repository. Results with missing provenance are rejected at the schema layer, ensuring that all retrieved information is fully traceable to its source commit. 

5. **Invariant I5 (Isolated Explain Cache):** An optional “explain” cache that stores additional LLM-generated annotations is maintained in a fully isolated SQLite table and is disabled by default. This separation ensures that the deterministic retrieval core (I2) cannot be contaminated by non-deterministic LLM outputs introduced through the opt-in annotation path. 

6. **Invariant I6 (Pinned Evaluation Corpus):** The evaluation corpus is pinned at exact commit SHAs in a lock file (eval/corpus.lock.toml) and verified by BLAKE3 + SHA-256 cryptographic checksums before any evaluation run. This prevents uncontrolled upstream changes from silently altering the benchmark dataset between evaluation runs. 

7. **Invariant I7 (Stated Baselines and Uncertainty):** All reported evaluation metrics must be accompanied by explicitly stated baseline comparisons and bootstrap 

confidence intervals. Point estimates without uncertainty quantification are rejected by the evaluation report generator, enforcing methodological transparency. 

8. **Invariant I8 (Explicit First-Parent Traversal):** The commit walker must always use first-parent traversal mode, never implicit or default traversal. This is enforced in the walker configuration and verified by a dedicated test that confirms no second-parent commits appear in the indexed dataset for a repository with known merge topology. 

These constraints are mechanically validated by a dedicated **cargo xtask test-invariants** suite that immediately fails CI builds upon any violation. This rigid enforcement ensures the tool remains fully reproducible, auditable, and secure for deployment in isolated, air-gapped development environments. 



<!-- Start of picture text -->
4 I Tantivy | Mall<br>aa D4 Template tex BM25 = RRF Hybrid- {scoresRAF  |.) Command-Line<br>touches Renderer Dense aa Interface (CLI)<br>commit eS) =<br>Git |S*eap] First-Parent ||.) AST Parser Vectors ;<br>Repository| Walker (@ Languages) ‘embedsings| (eastEmbed) we<br>Lineage co-change<br>Resolver 2 links |, Co-ChangeGraph RAG/agenta peetnace)Sa<br>(WAL) evn’ rer<br><!-- End of picture text -->

Figure 3.1: Step-by-step pipeline from version history to structured, query-able knowledge base. 

As illustrated in Figure 3.1, the data processing architecture relies on a strict sequential indexing pipeline to transform raw commits into a query-able format. The end-to-end system operates across four primary functional stages: 

- **Extraction Layer:** The process begins when a first-parent commit walker systematically traverses the repository history to extract a commit stream. This specialized traversal prevents duplicate processing of merged branches. Each commit is then evaluated by a multilingual AST parser that identifies function-level modifications. 

- **Processing Layer:** The extracted AST touches are routed into two parallel analytical engines. The D4 Template Renderer converts raw structural changes into natural language text and generates semantic embeddings. Simultaneously, the Lineage 

Resolver maps function identities over time to maintain continuity despite file renames or refactoring. 

- **Storage Layer:** The processed data is persisted atomically across three distinct database engines. The natural language templates are indexed lexically using Tantivy BM25. The generated embeddings are stored in a dense vector database via FastEmbed. Finally, the structured lineage data is written to a Write-Ahead Logging SQLite database. 

- **Query and Output Layer:** During retrieval, the system executes a hybrid search by combining Tantivy and Dense Vector scores through Reciprocal Rank Fusion. Additionally, the SQLite database feeds into a Co-Change Graph to expose architectural dependencies. The final ranked results are delivered to either a human-facing Command-Line Interface or an autonomous agent via the Model Context Protocol Server 

##### **3.2  Git Extraction Layer and Commit Traversal** 

The Git extraction layer serves as the foundational data ingestion pipeline for the system. Repository access is implemented using libgit2 through the git2 Rust crate. To eliminate external runtime path dependencies, the native library is bundled with the project to ensure consistent builds without relying on system-level installations. 

###### **3.2.1  First-Parent Traversal Strategy** 

A critical architectural decision in this layer is the enforcement of first-parent traversal, which constitutes system Invariant I8. The commit walker exclusively follows the primary parent at each merge commit rather than exploring all divergent branches. This methodology prevents diff duplication. If the system processed both the individual feature branch commits and the resulting merge commit, identical changes would be indexed twice. Such duplication would fundamentally corrupt the statistical co-change coupling metrics and artificially inflate function modification counts. 

###### **3.2.2  Structural Differencing and Rename Detection** 

During traversal, the system computes a structural tree diff for every commit against its first parent. This process leverages native Git rename detection to classify file deltas strictly as additions, modifications, deletions, or renames. Every detected rename is recorded into a 

dedicated rename map. The downstream lineage resolver later consumes this map to maintain tracking continuity when developers restructure directories. Following the diff computation, the extraction layer retrieves the raw historical file blobs using their cryptographic Object Identifiers (OIDs). The system then passes these raw byte streams directly to the abstract syntax tree (AST) parser for structural analysis. 

###### **3.2.3  Graceful Degradation and Fault Tolerance** 

Historical version control data frequently contains transient syntax errors or unsupported legacy language constructs. To ensure continuous processing without fatal crashes, the AST extraction pipeline implements a graceful degradation mechanism. Each processed touch record includes a **parse_status** field that classifies the extraction outcome as ok, degraded, or failed. When the parser encounters an invalid AST and extraction fails, the system automatically falls back to emitting file-level text chunks instead of fine-grained function-level components. This fault-tolerant design ensures that unparse-able commits do not halt the indexing pipeline and that the system retains as much historical metadata as possible. 

##### **3.3  Pseudo-Document Template Rendering (D4)** 

A central methodological contribution of the gitctx engine is the construction of D4 pseudodocuments. Rather than indexing raw unified diffs directly, the system generates structured natural-language text chunks deterministically from function-level commit touches. This approach bridges the semantic gap between conversational developer queries and rigid diff syntax. 

For every function modified in a commit, a dedicated D4TemplateRenderer produces a document comprising six canonical sections: 

1. **Header:** A descriptive string identifying the function name, file path, author, and modification date. 

2. **Commit Message:** The sanitized and secret-redacted repository log message. 

3. **Added Identifiers:** New programming tokens introduced by the diff. 

4. **Removed Identifiers:** Programming tokens deleted by the diff. 

5. **Function Signature:** The complete function declaration extracted from the post-change abstract syntax tree. 

6. **Surrounding Docstring:** The documentation block immediately preceding the function definition. 

The renderer cleanly omits any empty sections to optimize storage. To process identifier deltas accurately, the system extracts tokens from the diff hunk by splitting strings on camelCase boundaries, underscores, and common programming delimiters. It subsequently filters out standard programming language keywords. The resulting pseudo-documents closely resemble the natural language prose a developer would naturally write to explain a code change. 

###### **3.3.1  Credential Sanitation** 

Before rendering the final text, the pipeline applies a **SecretRedactor** pass to all free-text fields. This module utilizes high-entropy string detection and common API key prefix patterns to scrub credentials from commit messages and variable names. Although this operates strictly as a defense-in-depth measure without absolute cryptographic security guarantees, it effectively prevents the casual exposure of sensitive secrets within plain-text query results. 



<!-- Start of picture text -->
poo" processing ‘D4 Pseudo-Document Output |<br>r 0 Tnputs i ' i<br>| - teentifier Keyword Filter : - & Removed<br>u<br>Diff Hunk } :' Extractor id *' :' Identifiers<br>H >| Secret Redactor : H 3. Added<br>Commit Message - i ' H Identifiers<br>{ : Signature & ' '<br>Post-change AST - - Doestring i F 2. Commit,<br>‘ 4 Extractor 7 p Message<br>Commit H '<br>Metadata | - 5, Signature<br>j<br><!-- End of picture text -->

Figure 3.2: Architectural Data flow for D4 pseudo-document construction, transforming raw commit metadata, diff hunks, and abstract syntax trees into natural language templates for semantic indexing. 

As detailed in Figure 3.2, the construction of a D4 pseudo-document operates as a multi-stage data transformation pipeline. The process accepts four distinct inputs from the extraction layer: raw commit metadata, the repository log message, the textual diff hunk, and the parsed abstract syntax tree. During the processing phase, the system routes these inputs through specialized extraction modules. The commit message undergoes a secret redaction pass to remove sensitive credentials. Simultaneously, the identifier extractor parses the diff hunk, splits tokens based on common programming delimiters, and removes standard language keywords to isolate meaningful additions and deletions. Finally, the system extracts the structural signature and contextual docstring directly from the AST. These processed components are deterministically assembled into the final six-section template. This automated assembly ensures that all semantic context is preserved for downstream vector embedding and lexical indexing. 

##### **3.4  Function Lineage Engine** 

The function lineage engine represents the core algorithmic contribution of the system. Its primary purpose is to maintain stable and continuous identities for functions across their entire evolutionary lifetime. This continuity is preserved even when developers rename functions, move them across different files, or modify their signatures. 

Function identity is represented as a **LineageId** , which is a monotonically increasing integer that uniquely identifies a logical function across all its manifestations. The lineage engine processes commits in strict chronological order and maintains a table of active symbols indexed by file path. For each new commit, the system performs a greedy bipartite matching between the symbols of the previous commit and the current commit. 

Match candidates are generated and scored through a four-tier resolution cascade, as shown in Figure 3.3 includes: 

- **Tier 1 (Exact Key Match):** The system first attempts an exact comparison of a triple containing the normalized path, qualified name, and signature. This handles the common scenario where a function is modified in place with no structural change to its identity. 

- **Tier 2 (Same-File MinHash):** When the exact key fails but the function remains in the same file, the system computes the Jaccard similarity of the function bodies. It uses 128-permutation MinHash fingerprints constructed from token trigrams hashed with a pure-Rust Murmur3 implementation. The system explicitly excludes functions shorter than 10 tokens to avoid false positives on simple boilerplate code. 

- **Tier 3 (Git Rename Map):** The system cross-references rename events detected by the extraction layer. If a file was renamed and the function name is preserved, the lineage chain extends across the rename using this mapping. 

- **Tier 4 (Cross-File MinHash):** For functions that move to a different file and undergo content changes, a scoped cross-file MinHash search is executed across all active symbols. This tier resolves complex extraction refactors where a function is simultaneously relocated and partially rewritten. 



<!-- Start of picture text -->
Commit N vs N-1<br>No '' 3-shingle tokens | }'<br>72: Same-file >, -------»| | Murmur3-32— | |<br>MinHash? H 128 seeds '<br>| LLhat = hits /128 | |<br>map match? H<br>T4: Cross-file Eee ener ee<br>MinHash?<br>No<br>Extend lineage New introduction<br>same Lineageld fresh Lineageld<br>Greedy bipartite<br>matching<br>Mark deleted<br><!-- End of picture text -->

Figure 3.3: The four-tier lineage resolution cascade illustrating the vertical decision flow for tracking function identity, including exact key matching, MinHash fallback mechanisms, and greedy bipartite matching. 

The system resolves the bipartite matching greedily by sorting candidate pairs by confidence score and assigning each source symbol to its highest-confidence match. Unmatched source symbols are marked as deleted. Unmatched target symbols are treated as new introductions and assigned fresh identity integers. The complete resolution history is ultimately recorded as a directed acyclic graph to enable queries that trace the full ancestry of any given function. 

##### **3.5  Storage Orchestration and Hybrid Retrieval** 

The storage layer of the gitctx engine is designed to maintain high write throughput during repository indexing while enabling low-latency queries during developer workflows. All structured relational data is persisted to a local SQLite database configured in Write-Ahead Logging mode. This database resides directly within the target repository and stores commits, lineage chains, function touches, and precomputed co-change statistics. To prevent storage operations from blocking the main extraction pipeline, the system batches all database writes in a background thread utilizing a channel-based writer architecture. 

###### **3.5.1  Dual-Engine Document Indexing** 

The generated D4 pseudo-documents are simultaneously indexed by two complementary retrieval engines. The lexical retrieval engine uses Tantivy to construct an incremental BM25 index. This engine utilizes a custom code-aware tokenizer that splits strings on camelCase boundaries, underscores, and hyphens to accurately isolate programming identifiers. 

The semantic retrieval engine encodes each pseudo-document into a 384-dimensional dense vector using the BGE-Small-EN-v1.5 embedding model. To guarantee strict offline execution, this model is compiled to ONNX and executed via the ONNX Runtime dynamic library. The resulting embedding vectors are stored directly as memory-mapped floating-point arrays on disk. This memory-mapping strategy avoids the massive startup overhead typically associated with loading neural inference sessions, allowing rapid approximate nearest-neighbour searches via exhaustive scanning. 

###### **3.5.2  Query Execution and Rank Fusion** 

During query execution, the system retrieves independent document rankings from both the Tantivy BM25 index and the dense vector store. These two result sets are fused into a single unified ranking using Reciprocal Rank Fusion with a configured smoothing parameter of 60. 

To satisfy the strict deterministic output requirements of Invariant I2, the retrieval engine breaks any RRF score ties systematically by sorting the tied results in ascending order of their internal touch identifiers. Furthermore, the system implements a dedicated fast path for exact query matches. When a user query precisely matches a known function name or a specific cryptographic commit hash, the engine bypasses standard retrieval mechanisms. It retrieves the 

specific record directly from the SQLite index and injects it at rank zero, ensuring the exact match mathematically dominates the final output ranking. 



<!-- Start of picture text -->
Exact symbol or<br>SHA match?<br>hit miss miss<br>eadeae BGE-Small-EN-<br>tokenizer v5<br>ONNX, offline<br>Rank0 injection Tantivy BM25<br>RRF Fusion<br>k=60<br>Chain assembly<br>&<br>citation<br>attachment<br>Ranked results<br>SHA, author, date<br><!-- End of picture text -->

Figure 3.4: The hybrid retrieval and query execution pipeline illustrating the parallel BM25 and dense vector search paths alongside rank-zero injection for exact matches. 

As detailed in Figure 3.4, the query execution pipeline routes incoming requests through a conditional evaluation. If a query precisely matches a known symbol or commit hash, it triggers a fast path for rank-zero injection. Otherwise, the system processes the query through parallel lexical and semantic branches. The lexical path tokenizes the input for the Tantivy BM25 index, while the semantic path generates an offline ONNX embedding for nearest-neighbour scanning. The system then merges the resulting rankings using Reciprocal Rank Fusion before assembling the final lineage chains and attaching historical citations. 

##### **3.6  Co-Change Coupling and Impact Analysis** 

The co-change analysis engine computes statistical coupling metrics over the indexed commit history to identify implicit architectural dependencies. To ensure high data quality, the system filters out mass-formatting events by strictly analysing commits that modify a maximum of 25 files. This threshold balances analytical sensitivity against noise by excluding the top percentile of commit sizes while retaining the vast majority of genuine functional changes. By isolating these targeted modifications, the engine constructs an accurate historical map of files that evolve together organically. This refined dataset prevents spurious correlations from polluting the downstream probabilistic models. 

For every qualifying commit, the engine calculates three association metrics for each pair of modified entities: 

- **Support:** The raw co-occurrence count representing the number of qualifying commits in which both entities were modified together. 

- **Confidence:** The conditional probability that one entity changes given that the corresponding entity also changes. 

- **Lift:** The ratio of the observed joint probability to the expected joint probability assuming statistical independence. Lift values substantially above 1.0 indicate architecturally significant coupling rather than incidental coincidence. 

To prevent legacy architectural boundaries from masking current structural dependencies, the system applies an exponential time-decay function to all co-change events before final aggregation. The temporal weight _w_ ( _t_ ) for a commit at Unix timestamp _t_ is calculated as: 

_w_ ( _t_ )=2−( _t ref_ - _t_ ) / _h_ 

where _t ref_ represents the reference epoch at index time and _h_ defines the half-life in days, defaulting to 180 days. This decay mechanism ensures that coupling patterns from two years ago contribute half the weight of recent changes. 

The system stores these weighted co-change pairs in a dedicated graph structure. During query operations, an impact scorer traverses this graph to generate ranked risk assessments for any specific file or function. This mechanism successfully surfaces the components most likely to require coordinated modification and accurately quantifies the historical strength of those dependencies. 

##### **3.7  Output Layer and Agent Integration** 

The final stage of the system pipeline exposes the processed semantic knowledge base to both human developers and autonomous AI agents. This dual-interface architecture guarantees that the retrieved insights remain highly accessible while preserving the strict offline constraints of the system. 

###### **3.7.1  Human-Facing Command Interface** 

The gitctx-cli module provides a native terminal application for manual developer queries. It receives the ranked Reciprocal Rank Fusion scores directly from the query and analysis layer. The interface formats the retrieved D4 pseudo-documents, structural metadata, and temporal decay metrics into readable console outputs. This allows developers to investigate function lineage and assess the impact of proposed changes natively within their local development environment. 

###### **3.7.2  Model Context Protocol (MCP) Server** 

To facilitate advanced reasoning over repository history, the system implements a Model Context Protocol server via the gitctx-mcp binary. This component acts as a bridge between the local semantic knowledge base and autonomous Large Language Models by communicating over standard input and output using newline-delimited JSON-RPC. The server exposes five dedicated analytical tools to AI agents: search_history, function_evolution, change_impact, who_knows_this, and commit_detail. Each tool supports a configurable mode parameter that controls the verbosity of the returned records. This toggle allows AI agent hosts to actively budget token consumption against their specific context window constraints. 

The server architecture enforces strict output isolation to maintain protocol integrity. All diagnostic tracing and logging are directed exclusively to standard error, ensuring that no extraneous non-JSON content can corrupt the output stream. Furthermore, the MCP server carries no state between invocations. Each individual tool call independently opens the local index, performs its query, and returns the result. This stateless design makes the server trivially restart-able and seamlessly compatible with any MCP-capable AI coding assistant without introducing session management overhead. 

**IV.   EVALUATION METHODOLOGY** 

#### **REFERENCES** 

1] A. E. Hassan and R. C. Holt, "The Top Ten List of Software Aging Indicators," _Proceedings of the International Conference on Software Maintenance (ICSM)_ , Amsterdam, Netherlands, 2003, pp. 253–262. 

[2] M. Fischer, M. Pinzger, and H. Gall, "Populating a Release History Database from Version Control and Bug Tracking Systems," _Proceedings of the International Conference on Software Maintenance (ICSM)_ , Amsterdam, Netherlands, 2003, pp. 23–32. 

[3] R. Moser, W. Pedrycz, and G. Succi, "A Comparative Analysis of the Efficiency of Change Metrics and Static Code Attributes for Defect Prediction," _Proceedings of the 30th International Conference on Software Engineering (ICSE)_ , Leipzig, Germany, 2008, pp. 181– 190. 

[4] P. C. Boogerd and L. Moonen, "On the Relation Between Code Quality and Software Quality," _Proceedings of the 15th Working Conference on Reverse Engineering (WCRE)_ , Antwerp, Belgium, 2008, pp. 31–40. 

[5] P. Levin and A. Yehudai, "Commit Classification by Purpose," _Proceedings of the 14th International Conference on Mining Software Repositories (MSR)_ , Buenos Aires, Argentina, 2017, pp. 483–486. 

[6] J. Bissyandé, D. Lo, L. Jiang, L. Réveillère, J. Klein, and Y. Le Traon, "Got Issues? Who Cares About It? A Large Scale Investigation of Issue Trackers from GitHub," _Proceedings of the 24th International Symposium on Software Reliability Engineering (ISSRE)_ , Pasadena, CA, USA, 2013, pp. 188–197. 

[7] D. Spadini, M. Aniche, and A. Bacchelli, "PyDriller: Python Framework for Mining Software Repositories," _Proceedings of the 26th ACM Joint Meeting on European Software Engineering Conference and Symposium on the Foundations of Software Engineering (ESEC/FSE)_ , Lake Buena Vista, FL, USA, 2018, pp. 908–911. 

[8] V. Rajlich and J. Wilde, "The Role of Concepts in Program Comprehension," _Proceedings of the 10th International Workshop on Program Comprehension (IWPC)_ , Paris, France, 2002, pp. 271–278. 

[9] S. Robertson and H. Zaragoza, "The Probabilistic Relevance Framework: BM25 and Beyond," _Foundations and Trends in Information Retrieval_ , vol. 3, no. 4, pp. 333–389, 2009. 

[10] Z. Feng, D. Guo, D. Tang, N. Duan, X. Feng, M. Gong, L. Shou, B. Qin, T. Liu, and M. Zhou, "CodeBERT: A Pre-Trained Model for Programming and Natural Languages," _Findings of the Association for Computational Linguistics: EMNLP 2020_ , Online, 2020, pp. 1536–1547. 

[11] H. Gall, K. Hajek, and M. Jazayeri, "Detection of Logical Coupling Based on Product Release History," _Proceedings of the International Conference on Software Maintenance (ICSM)_ , Bethesda, MD, USA, 1998, pp. 190–198. 

[12] P. Lewis, E. Perez, A. Piktus, F. Petroni, V. Karpukhin, N. Goyal, H. Küttler, M. Lewis, W. Tau Yih, T. Rocktäschel, S. Riedel, and D. Kiela, "Retrieval-Augmented Generation for Knowledge-Intensive NLP Tasks," _Advances in Neural Information Processing Systems (NeurIPS)_ , vol. 33, pp. 9459–9474, 2020. 

[13] G. V. Cormack, C. L. A. Clarke, and S. Buettcher, "Reciprocal Rank Fusion Outperforms Condorcet and Individual Rank Learning Methods," _Proceedings of the 32nd International ACM SIGIR Conference on Research and Development in Information Retrieval_ , Boston, MA, USA, 2009, pp. 758–759. 

[14] N. Thakur, N. Reimers, A. Rücklé, A. Srivastava, and I. Gurevych, "BEIR: A Heterogeneous Benchmark for Zero-Shot Evaluation of Information Retrieval Models," _Proceedings of the 35th Conference on Neural Information Processing Systems (NeurIPS) — Datasets and Benchmarks Track_ , Online, 2021. 

[15] A. Tornhill, _Your Code as a Crime Scene: Use Forensic Techniques to Arrest Defects, Bottlenecks, and Bad Design in Your Programs_ . Raleigh, NC, USA: Pragmatic Bookshelf, 2015. 

[16] T. Mens and T. Tourwé, "A Survey of Software Refactoring," _IEEE Transactions on Software Engineering_ , vol. 30, no. 2, pp. 126–139, Feb. 2004. 

[17] J. Grund, S. Kochhar, and D. Lo, "CodeFacts: Mining Code Entities and Their Dependencies from Version Histories," _Proceedings of the 18th International Conference on Mining Software Repositories (MSR)_ , Madrid, Spain, 2021, pp. 1–12. 

[18] A. Z. Broder, "On the Resemblance and Containment of Documents," _Proceedings of the Compression and Complexity of Sequences_ , Positano, Italy, 1997, pp. 21–29. 

[19] S. Mundler, J. He, J. Pierson, and M. Vechev, "Self-Contradictory Hallucinations of Large Language Models: Evaluation, Detection and Mitigation," _arXiv preprint arXiv:2305.15852_ , 2023. [Online]. Available: https://arxiv.org/abs/2305.15852 

[20] Anthropic, "Model Context Protocol Specification," 2024. [Online]. Available: https://modelcontextprotocol.io 
