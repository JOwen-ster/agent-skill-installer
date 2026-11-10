# Installation efficiency evaluation

What this repo does: Agent Skill Installer discovers skills in GitHub repositories and installs their files into the selected agent's skills directory. Both the skills.sh queue and the GitHub URL inputs use the same Rust GitHub discovery and installation functions.

Reviewed on November 10, 2026, against working tree HEAD `b990058`. This is an evaluation only; no implementation changes were made.

## Scope, assumptions, and existing fixes

The expected load is one desktop user installing one skill, several skills from one repository, or a mixed queue from several repositories. Large skill catalogs and skills with many small supporting files matter more than server-style concurrent users. The review follows discovery, selection, downloads, filesystem writes, progress updates, installed-skill refreshes, and history.

**Already implemented:** GitHub search and installation run in `tauri::async_runtime::spawn_blocking`; they do not perform their blocking network work directly in the frontend. The user confirmed that background workers fixed the queue freeze. GitHub name collisions already use the owner prefix and reject a second collision without replacing files. These are not missing features or proposed fixes here. Earlier freeze and rename TODO entries were removed during this review and are not used to establish a current defect. The current TODO suggestions about worker/download limits are consistent with the bounded scheduling recommendations below.

There is still measurable unnecessary CPU work in the queue's result processing, described below. That is an operation-count finding, not a claim that the fixed freeze still reproduces.

Repository-relative source locations below refer to the reviewed working tree. Other working-tree edits, including `TODO.md` and the pre-existing change to `src/lib/components/InstalledSkillsList.svelte`, were left untouched by this evaluation.

### How to read the ratings and estimates

| Rating | Meaning |
| --- | --- |
| 5/5 | A major source of serialized waiting or repeated work in ordinary multi-skill use. |
| 4/5 | Large avoidable cost for catalogs, overlapping selections, or larger candidate sets. |
| 3/5 | Material with many installed skills, accumulated history, large files, or failures. |
| 2/5 | Smaller or conditional cost; optimize after the main bottlenecks. |
| 1/5 | Negligible for this workload; not promoted to a finding. |

Ratings describe wasted work, not security severity or a measured speedup. Work estimates include a focused regression check and assume one developer familiar with the code. Small means a local helper or caller change, medium means several related functions or an IPC contract change, and large means restructuring discovery or installation scheduling. Estimates overlap and must not simply be added.

## Workflow map and current cost

| Workflow | Current path | Important behavior |
| --- | --- | --- |
| Queue preparation | `SkillsSearch.submitQueue` -> installed-skill scan -> `BulkQueueInstall` -> one `loadPanel` per queued item -> `searchGithub` -> `search_github_repo` | Same-repository items independently retrieve the same repository metadata and tree. All panel searches are launched immediately. |
| Queue installation | `installPanel` or `installAll` -> `installGithubSkills` -> repeated one-request Tauri calls -> Rust `install_github_skills` -> `install_github_candidate` | Skills are installed sequentially. Bulk selections are not deduplicated across panels. |
| GitHub URL preparation | `GithubInputRow.triggerSearch` -> `searchGithub` -> `search_github_repo` -> `SearchResults` -> `SkillSelect` | Search is explicitly triggered by Enter or the Search button. Each row owns its own result; there is no shared discovery cache. |
| GitHub URL installation | `SearchResults.installSelected` -> same API and Rust installation functions as the queue | Multiple selections use the same sequential implementation. Different rows also share the global install lock. |
| Completion | `oninstalled` -> `refreshData` and, if open, history refresh | Installed directories and descriptions are read again; source metadata comes from historical install records. |

Let `Q` be queued items, `R` distinct repository/ref combinations, `S` candidates in a repository, `B` repository blobs, `I` installed skills, and `F` files in one selected skill, including its `SKILL.md`.

- A repository search without an explicit branch performs two HTTP requests: repository metadata, then the recursive tree. An explicit branch skips the metadata request.
- A successful skill install normally performs `F + 1` raw-file requests: validation fetches `SKILL.md`, and the file loop fetches it again.
- Consequently, a fresh default-branch single-skill URL search and install uses `F + 3` HTTP requests. A skill containing only `SKILL.md` still uses four requests overall.
- A default-branch queue uses approximately `2Q + sum(F_i + 1)` requests before accounting for failures or overlapping jobs. Ten distinct skills from the same repository, each containing 20 files, currently use 230 requests: 20 discovery requests and 210 raw requests. Sharing discovery and reusing validated Markdown would reduce that example to 202 requests, before changing concurrency.
- Candidate construction performs roughly `S * B` path tests, with a new directory-prefix string allocated inside each non-root test.
- Requests and latency are separate issues: avoiding redundant work reduces request counts; bounded concurrency overlaps the remaining waits. Their expected gains are not additive percentages.

These are counts derived from the implementation, not live GitHub timings. Exact completion time also depends on connection reuse, bandwidth, disk speed, and remote responses.

## Should fix: installation and discovery

### 1. Every file in a skill waits for the previous download

**Rating:** 5/5. **Applies to:** single and multi-skill installs through both inputs.

**Evidence:** `src-tauri/src/github.rs:254-307`, especially the blocking fetch/write loop at lines 288-306. `fetch_raw_bytes` waits for a complete response before the next file begins.

**Problem and concrete case:** A skill with 100 small files performs 101 serial raw requests including validation. Even when bandwidth is mostly idle, each file pays its request wait before the next one can start. As an illustration only, at 100 ms per response this is about 10 seconds of file-loop waiting plus validation, before other costs. `spawn_blocking` protects responsiveness but does not overlap those downloads.

**Ideal proposed fix:** Retain validation first, then fetch the remaining files with a small bounded pool, sharing one HTTP client and writing to the skill's staging directory. Use one overall download limit across active skills rather than multiplying an unrestricted per-skill limit. Keep all-files-successful-before-rename behavior, path validation, and cleanup. Converting the existing helpers to async reqwest is one option; a bounded blocking worker approach can also work without rewriting the UI.

**Work / rewrite:** Medium, approximately 1-2 days for file-level concurrency alone. Changes center on the download helper, candidate installer, and error aggregation; the visible selection flow can stay. Check the concurrency ceiling, an out-of-order completion, and a failed file that must prevent publication.

**If skipped:** Both a single large skill and a large queue continue to scale with the sum of individual file request waits.

### 2. The global installation lock covers network waiting, and the frontend defeats backend batching

**Rating:** 5/5. **Applies to:** multiple skills, multiple URL rows, and overlapping queue/URL operations.

**Evidence:** `src/lib/api.ts:22-41` invokes the backend once per request and awaits it. `src-tauri/src/lib.rs:63-123` holds `INSTALL_LOCK` across database opening, every download, writes, and history recording. Dropped installs acquire the same lock at line 138.

**Problem and concrete case:** Three independent skills cannot download concurrently, even when they target different agent directories. A slow response can hold the lock for its network wait while unrelated installs sit behind it. Passing an array directly to the current Rust command saves IPC overhead but still executes its `for` loop sequentially. Wrapping frontend calls in `Promise.all` also fails to remove the backend lock bottleneck.

**Ideal proposed fix:** Submit one batch, keep the current progress behavior through a Tauri channel or equivalent per-job progress messages, and stage a bounded number of jobs concurrently. Preserve serialized destination-name allocation and publication. Use unique staging directories and either reserve destination names before downloads or recheck/resolve names under the lock immediately before publication. Narrowing the lock without addressing the deterministic staging paths would introduce races. File concurrency from finding 1 should share this batch's overall limit.

**Work / rewrite:** Large, approximately 2-4 days including progress and collision tests. Reaches `api.ts`, the Rust command and installer, `SearchResults`, `BulkQueueInstall`, and `tests/api.test.mjs`, whose current test explicitly expects one request per invocation. This is a coordinated install-path change, not an application rewrite.

**If skipped:** Multi-skill throughput remains serialized even after other request-count optimizations. Keep the existing lock until its safety role has a replacement.

### 3. Queue panels repeat discovery for the same repository and rebuild every panel on resubmission

**Rating:** 5/5. **Applies to:** multi-skill queue preparation; repeated repositories in URL rows also repeat discovery.

**Evidence:** `BulkQueueInstall.svelte:48-72,84-102` keys panels by repository URL plus skill slug, builds every panel anew, and starts one search per panel. `SkillsSearch.svelte:75-83` assigns a fresh submitted queue. `GithubInputRow.svelte:60-76` has only row-local search state; `github.rs:107-165` has no cache.

**Problem and concrete case:** Ten queued skills from one repository cause ten metadata requests, ten recursive tree requests, and ten copies of candidate processing. Adding a queue item and submitting again restarts unchanged panels as well. Searches for removed/replaced panels continue running. There is no bound on the number of panel searches launched by the loop, so a large queue creates a burst of duplicate workers and responses. Different completion times do not make the duplicate work useful.

**Ideal proposed fix:** Share an in-flight promise/result by normalized repository and ref across both input workflows, while retaining per-skill selection state. Group queued items before discovery and reuse unchanged panels when the submitted set changes. Bound discovery for distinct repositories. Use a submission generation to ignore obsolete results; cancellation where feasible saves the work as well. Cache lifetimes must be explicit: share within a preparation run first, and provide deliberate revalidation for a user-requested refresh. Preserve successful-search history semantics separately from fetching.

**Work / rewrite:** Medium, approximately 1-2 days. Primarily shared discovery state plus queue reconciliation; no download rewrite is required. Check same-repository requests started together, adding one new item, removal during search, and intentional refresh.

**If skipped:** A queue of `Q` items keeps paying `Q` full discoveries even when there are only `R` distinct repositories. This waste occurs before installation starts.

### 4. Queue completion merging performs cubic total work when all candidates are installed

**Rating:** 4/5. **Applies to:** larger queue panels and their progress updates.

**Evidence:** `BulkQueueInstall.svelte:130-136` maps all candidates for each completion, then runs `.find` over the old result array for almost every candidate. Both completion callbacks call it with a single new result at lines 149-154 and 181-185.

**Problem and concrete case:** One completion can cost `O(S^2)` comparisons; completing `S` selected candidates costs `O(S^3)` in total. An isolated probe executing the actual function counted 661,650 old-result entries visited for 100 candidates and 5,313,300 for 200 candidates. Doubling the candidate count caused approximately eight times the work. This work runs in frontend JavaScript despite downloads running in a background worker. It establishes unnecessary CPU cost, not a reproduced UI freeze.

**Ideal proposed fix:** Keep result positions or a map keyed by stable candidate identity, such as `skill_md_path`, and update only the completed candidate. If the aligned result array is retained, precompute identity-to-index and replace one entry in a shallow copy. Avoid rebuilding a map and searching the old array per row. `candidate_name` alone is ambiguous when different directories have the same basename; the result/progress contract should carry a stable identity or original request index.

**Work / rewrite:** Small to medium, approximately 0.5-1 day including identity propagation if needed. Reaches queue result state and possibly Rust/TypeScript result types. Check repeated basenames, preserved old results, and linear rather than nested search work.

**If skipped:** Large selections spend increasingly disproportionate CPU time handling completion messages.

### 5. Candidate discovery rescans every repository file for every skill

**Rating:** 4/5. **Applies to:** discovering catalogs through either input.

**Evidence:** `github.rs:179-206` filters `all_blob_paths` separately for each `SKILL.md`. The predicate creates `format!("{}/", dir_path)` inside the innermost loop.

**Problem and concrete case:** A tree with 20,000 blobs and 500 skills performs 10 million path checks. For non-root candidates it also builds a temporary prefix for each check. The cost is repeated for every same-repository panel under finding 3, and file-path strings are copied into candidate manifests, including overlapping descendants.

**Ideal proposed fix:** First move prefix creation outside the predicate for an immediate small win. Then sort/index blob paths once and find each directory's contiguous prefix range, or walk parent directories and assign blobs to matching skill roots. Aim for approximately `O(B log B + S log B + output paths)` instead of `O(SB)`, while preserving current inclusion rules. The present root-level candidate includes only top-level files; an efficiency change must not accidentally change that behavior.

**Work / rewrite:** Prefix hoist: small, under an hour. Indexed discovery: medium, approximately 0.5-1 day, confined mostly to candidate construction and fixtures. Test adjacent prefixes (`pdf` versus `pdf-tools`), nested skill roots, and root candidates.

**If skipped:** Large catalogs remain expensive to parse even after redundant HTTP discovery is eliminated.

### 6. Bulk selections can schedule the same source skill more than once

**Rating:** 4/5. **Applies to:** overlapping queue panels and repeated URL selections.

**Evidence:** `BulkQueueInstall.svelte:112-127,164-180` filters each panel independently and concatenates requests without a batch-level identity check. Installed state refreshes only after completion. `SearchResults.svelte:69-74` does not pass installed skills to `SkillSelect`, leaving its installed-list prop at the empty default.

**Problem and concrete case:** Queue two catalog items from one repository, then select the same extra candidate in both panels. Both requests enter the batch before any session refresh. The second can download the same source into an owner-prefixed folder, or fail if both permitted destination names exist. Likewise, another URL row can select an already installed candidate. Owner-prefix renaming already works correctly; it is not intended to eliminate duplicate source jobs.

**Ideal proposed fix:** Deduplicate a submitted batch by owner, repository, resolved ref, and skill path, then fan its result out to every panel that selected it. Pass the active installed list into the manual URL selection flow. Do not deduplicate by display name: identically named skills from different repositories or directories must remain distinct. Preserve deliberate repeat-install behavior if that is a supported action, rather than silently changing naming rules.

**Work / rewrite:** Medium, approximately 0.5-1.5 days. Reaches bulk job construction, progress-to-panel mapping, and the manual wrapper's props. Test overlapping panels and distinct sources sharing a name.

**If skipped:** Some multi-skill selections repeat full downloads and create redundant installations even with fast networking.

### 7. A URL naming a skill subdirectory still fetches the entire repository tree

**Rating:** 4/5. **Applies to:** single-skill URLs in large repositories; queue resolution when an exact path is available.

**Evidence:** `github.rs:24-47` parses owner, repository, and one branch segment but discards the trailing directory. Lines 140-164 always request the full recursive tree and reject a truncated result. The parser test at lines 389-392 verifies the branch, not preservation of the skill path.

**Problem and concrete case:** A URL ending in `/tree/main/skills/pdf` does just as much discovery as the repository root. Unrelated repository content is transferred and processed. A full-tree truncation can reject the request even if the requested skill directory itself is small. This limitation is explicitly acknowledged by the error at line 164.

**Ideal proposed fix:** Preserve an explicit directory in the parsed reference and resolve/fetch that subtree instead of enumerating unrelated content. Keep root-URL discovery for users requesting all skills. Resolve branch/ref boundaries carefully, including refs containing slashes, and use the resolved commit consistently. A skills.sh slug is not proof of a repository path; only use a targeted queue lookup when its path is known or reliably resolved, with normal discovery as fallback.

**Work / rewrite:** Medium to large, approximately 1.5-3 days. Reaches URL parsing, branch/subtree resolution, discovery filtering, and tests. Installation itself can still consume the existing candidate shape.

**If skipped:** Explicit single-skill URLs pay repository-wide costs, and large repositories remain unsupported when their root tree is truncated.

### 8. Availability and selection repeatedly scan result and installed arrays

**Rating:** 3/5. **Applies to:** queue panels, catalog results, and manual selection result lists.

**Evidence:** `SkillSelect.svelte:74-104,121-135,164-175` repeatedly calls `isAvailable`, `.find`, and the installed lookup during effects, derived counts, selection emission, and rendering. `BulkQueueInstall.svelte:76-78,108-127,281-286` computes requests repeatedly. `src/lib/installed.ts:3-35` constructs a URL and scans the installed array per lookup.

**Problem and concrete case:** One availability pass can cost `O(S * (S + I))` when result arrays and installed lists are large; multiple passes occur for a state update. A probe of the actual installed helper performed 500,000 installed-name reads and 500 URL constructions for 500 missing-candidate checks against 1,000 installed skills. Manual `SearchResults.pendingCandidates` also runs an array search per selected candidate at lines 30-32. These costs compound with duplicate panels.

**Ideal proposed fix:** Index results by stable identity and installed skills by normalized folder name, including the existing owner-prefix and source-repository matching rules. Normalize source metadata once when installed state changes. Derive available candidates and per-panel pending requests once per relevant state change, and reuse those values for counts, buttons, and selection. Retain the existing helper's treatment of missing metadata and similarly named repositories.

**Work / rewrite:** Medium, approximately 1-2 days across installed-state lookup, `SkillSelect`, and both selection wrappers. Extend `tests/installed.test.mjs` to preserve its existing matching guarantees and add operation-count checks.

**If skipped:** Large libraries and catalogs keep spending frontend CPU on the same membership questions.

### 9. Validated Markdown is downloaded a second time

**Rating:** 3/5; especially worthwhile because the fix is small. **Applies to:** every successful GitHub install.

**Evidence:** `github.rs:257-266` fetches and validates `SKILL.md`; lines 288-306 download every file in the candidate, including that same path from discovery.

**Problem and concrete case:** A Markdown-only skill makes two raw requests for one file. Every multi-skill batch pays one extra request per skill. The second fetch also means the bytes installed can differ from the bytes validated if the branch changes between requests.

**Ideal proposed fix:** Retain the validated bytes and write them into staging when processing the Markdown path; fetch only other files. Preserve UTF-8 and frontmatter validation and correct `files_written` accounting. Using a resolved commit for all files complements this fix but is not necessary to remove the duplicate request.

**Work / rewrite:** Small, approximately 2-4 hours in one installer function plus a mocked-fetch regression check. No UI or database changes.

**If skipped:** Every install retains an avoidable request and its latency; for a one-file skill, half the raw-file requests are redundant.

### 10. HTTP clients are rebuilt per discovery, per skill, and per catalog search

**Rating:** 3/5. **Applies to:** repeated operations and multi-skill installs through both inputs.

**Evidence:** `github.rs:109,255,369-374` creates a fresh blocking client for each search and skill. `skills_sh.rs:41-45` constructs a fresh async client for each catalog query.

**Problem and concrete case:** Ten skills from the same raw-content host use ten separate client lifetimes. The current installer can reuse connections within one skill, but discards that pool at the end of the skill. New clients repeat setup and cannot reuse the previous client's idle connections across jobs. Actual connection savings depend on the remote server and network.

**Ideal proposed fix:** Store reusable clients in application state and pass or clone their handles into workers. Keep the GitHub and catalog clients separate if their policies differ. The blocking client must have a suitable blocking-context lifecycle, or be replaced by the async client as part of finding 1. Reuse the installed reqwest dependency; no wrapper framework is needed.

**Work / rewrite:** Small to medium, approximately 0.5-1 day. Reaches command state and network helper signatures. Keep timeouts and user agents, and verify repeated calls use the same client lifetime.

**If skipped:** Repeated operations continue paying client and connection setup that could be shared.

### 11. Queue preflight repeatedly reads every installed skill and its full Markdown body

**Rating:** 3/5. **Applies to:** queue search/submission and completion refreshes for both inputs.

**Evidence:** `SkillsSearch.svelte:49-52,79-83` fetches installed skills during each catalog search and again at submission. `+page.svelte:24-39,51-54` scans after installation. `fs_ops.rs:64-108` checks every directory, reads each full `SKILL.md`, parses its YAML, and sorts the list.

**Problem and concrete case:** With 1,000 installed skills, a catalog search followed by queue submission rereads approximately 2,000 Markdown files even if nothing changed. Only a small part of each file is needed for description display, and duplicate-prevention preflight mainly needs names and source identities. A search's `Promise.all` waits for the installed scan before showing catalog results, so the disk work can become visible latency.

**Ideal proposed fix:** Separate a lightweight identity refresh from description hydration, or cache descriptions by file metadata and only reread changed files. Stream just the frontmatter when extracting descriptions. Share in-flight scans for the same agent, update state from successful install results where possible, and retain a live refresh at meaningful boundaries because users can edit skills outside this app. Resolve the stable skills-directory path once per agent rather than refetching it after every install.

**Work / rewrite:** Medium, approximately 1-2 days across filesystem listing, installed state, and refresh callers. Test external additions, deletions, edits, and an agent switch. Do not replace live state with an indefinitely stale cache.

**If skipped:** Preparing and finishing installs gets slower as the installed library grows, even for a single newly selected skill.

### 12. Expensive installed-list and history reads remain synchronous Tauri commands

**Rating:** 3/5. **Applies to:** preflight and post-install refreshes, rather than the already-threaded downloads.

**Evidence:** `src-tauri/src/lib.rs:220-251` declares `get_installed_skills` and `get_history` as ordinary synchronous commands. They perform filesystem and SQLite work inline. The locally installed `tauri-macros` 2.7.4 command wrapper's `body_blocking` calls synchronous handlers inline; there is no automatic `spawn_blocking` around these bodies.

**Problem and concrete case:** A library scan of thousands of files or a large history query occupies the command-handling context until it finishes. In contrast, search and installation explicitly hand their blocking work to workers. The impact on visible responsiveness was not reproduced in a running window, and this finding does not establish that the user's fixed queue freeze remains present.

**Ideal proposed fix:** Apply the existing async-command-plus-`spawn_blocking` pattern to these blocking reads. Also reduce their work under findings 11, 13, and 14; moving work alone does not make it finish faster. Avoid spawning a worker per file or adding a separate job framework.

**Work / rewrite:** Small, approximately 2-4 hours plus command error-propagation checks. Changes are confined to the two Rust command bodies/signatures; frontend invocation names can stay.

**If skipped:** Refresh cost remains synchronous even though the actual download phase is correctly offloaded.

### 13. Current source metadata is rebuilt by replaying historical installs

**Rating:** 3/5. **Applies to:** every installed-list refresh in both workflows.

**Evidence:** `db.rs:173-196` selects all successful GitHub records for an agent in ascending ID order, parses source URLs, and overwrites earlier map entries. `lib.rs:226-237` runs it after filesystem listing. The schema has no supporting index for this filter.

**Problem and concrete case:** After 50,000 historical installs, listing 100 current skills can process tens of thousands of old rows, including deleted skills and superseded sources. Time and allocations grow with installation history instead of the current library. There is no need for an additional full sort by ID if the primary-key scan supplies that order, but the traversal and row decoding still occur.

**Ideal proposed fix:** Query only the latest successful GitHub record for each relevant current name and agent, supported by an appropriate composite or partial index. Read live names first and avoid returning irrelevant source rows. For the smallest change, improve this query and add an index before creating a second source-of-truth table. Preserve the present latest-record and agent-isolation behavior.

**Work / rewrite:** Small to medium, approximately 0.5-1 day for query, index migration, and focused fixtures. Verify the query plan against large synthetic history and keep `install_sources_keep_agents_separate` passing.

**If skipped:** History accumulation gradually slows an operation whose visible output may remain small.

### 14. An open history panel loads and renders all history after every install action

**Rating:** 3/5. **Applies to:** completion refresh when History is open.

**Evidence:** `db.rs:131-170` has no agent filter or limit. `HistoryPanel.svelte:15-29` fetches both agents' histories and filters them in JavaScript. Lines 77 and 94 render every matching record; the CSS's 200 px maximum height only limits the viewport. `refresh()` already avoids fetching when the panel is closed.

**Problem and concrete case:** With 20,000 historical records, installing one new skill can retrieve the complete history again, serialize it over IPC, filter it, and maintain thousands of DOM rows. The other agent's records are transferred even when they cannot be displayed. Multiple URL-row completions can start overlapping full refreshes; there is no in-flight coalescing in `loadHistory`.

**Ideal proposed fix:** Filter by agent in SQL, return a bounded first page ordered by ID, and load older records on demand. Refresh only rows newer than the last seen ID or reload the current bounded page. Coalesce overlapping loads. If counts are required, query counts separately rather than transferring all rows. Preserve the existing no-fetch-while-closed behavior.

**Work / rewrite:** Medium, approximately 1-2 days across history commands/types, SQL queries/indexes, and the panel. Check pagination boundaries, switching agents, and a refresh during an existing request.

**If skipped:** A background panel increasingly dominates perceived completion time for otherwise small installs.

### 15. Nested skill roots cause overlapping files to be transferred independently

**Rating:** 3/5, conditional on repository structure. **Applies to:** multi-skill installs from nested skill trees.

**Evidence:** `github.rs:196-206` includes every descendant blob for a non-root candidate, including files belonging to nested skill candidates. Each selected candidate later downloads its entire independent manifest at lines 288-307.

**Problem and concrete case:** Suppose `skills/parent/SKILL.md` and `skills/parent/child/SKILL.md` both exist. Selecting both downloads the child's support files inside the parent installation and again for the child. This can be correct destination content but still duplicates network work. It is different from finding 6: these are distinct source skills with partially overlapping files.

**Ideal proposed fix:** Share downloaded source bytes or staged source files within a batch, keyed by repository, immutable commit, and file path/blob identity. Copy them into each required destination. Preserve each candidate's current content semantics; simply excluding nested directories could remove assets a parent skill expects. An archive strategy is only justified when selected content covers enough of the repository to outweigh downloading unrelated files.

**Work / rewrite:** Medium, approximately 1-2 additional days once batch scheduling exists. Reaches manifest identity and a batch-local download cache, not the selection UI. Test nested roots, same path on different commits, and independent destination files after installation.

**If skipped:** Selected nested skills can repeatedly transfer the same large descendants. Do not solve this with hardlinks unless shared writable file behavior is explicitly acceptable.

### 16. A late download failure discards useful work and leaves staging files behind

**Rating:** 3/5 on unreliable networks. **Applies to:** failed installs from either input.

**Evidence:** `github.rs:288-305` returns immediately through `?` on download, directory, or write failure. Cleanup at lines 311-314 covers rename failure, not earlier errors. A later attempt removes the old staging directory at lines 273-276 before downloading everything again. The fetch helper has no retry behavior.

**Problem and concrete case:** If file 99 of 100 fails, the earlier downloads remain in staging but are not reused. Re-searching and trying again deletes them and repeats all prior downloads. The partial folder also consumes disk and cleanup time. A failed candidate result is treated as completed by the current selection logic, so an immediate retry may require rebuilding the search/panel first; the report does not assume an automatic retry UI exists.

**Ideal proposed fix:** Ensure cleanup on every failed staging exit. Handle a small, bounded number of retryable transient file failures without restarting successful work, while returning permanent errors promptly. If batch-local source caching from finding 15 exists, reuse only successful downloads for the same immutable source identity. Persistent resume storage is unnecessary for the first fix and needs additional integrity rules if added later.

**Work / rewrite:** Small for cleanup, approximately 2-4 hours; medium for bounded transient retries and verified reuse, approximately 0.5-1.5 days. Check a late failure, retry exhaustion, a permanent HTTP failure, and preservation of existing installed folders.

**If skipped:** Failed large installs waste most of the previous transfer and disk work before they can succeed.

### 17. Every queued skill renders the repository's entire candidate list

**Rating:** 3/5, rising for large shared catalogs. **Applies to:** queue preparation and progress display.

**Evidence:** `BulkQueueInstall.svelte:235-276` renders a separate `SkillSelect` for every queued item, each receiving all repository candidates. `SkillSelect.svelte:164-198` creates DOM rows for every candidate. `CollapsibleSection.svelte` always renders children and merely hides its content div.

**Problem and concrete case:** Ten queued skills from a repository with 500 candidates create 5,000 candidate rows and checkbox references, even though the panels refer to the same catalog. Collapsing a section does not unmount these rows or stop their reactive computation. Sharing discovery promises alone will not eliminate these independently rendered views.

**Ideal proposed fix:** Group candidates by repository and maintain queued selections in one repository picker, preserving each catalog item's intended preselection. If the per-item panels must remain, keep their selection state outside lazily mounted candidate lists and show a compact summary until expanded. Add windowed rendering only if a single remaining list is still too large; avoid introducing a dependency before reducing duplicate lists.

**Work / rewrite:** Medium, approximately 1-2 days. Mostly queue view/state restructuring; backend behavior can stay. Test manual checkbox choices surviving collapse/expansion, distinct queued preselection, progress, and keyboard accessibility.

**If skipped:** Large same-repository queues keep paying multiplied DOM and reactive-state costs even after downloads are accelerated.

## Nice to have: smaller or conditional costs

### 18. File downloads buffer and copy each full response before writing

**Rating:** 2/5; more important before increasing download concurrency. **Applies to:** skills containing large supporting files.

**Evidence:** `github.rs:356-366` collects the complete response into bytes and then copies it with `.to_vec()`. Lines 290-305 hold that vector until `std::fs::write` completes.

**Problem and concrete case:** A 100 MB support file requires a full response buffer and a second 100 MB vector during conversion, rather than a bounded write buffer. Serial downloads currently limit this to one file at a time; eight concurrent large responses would multiply memory pressure and may undermine the intended throughput gain.

**Ideal proposed fix:** Stream non-Markdown response bodies into staging files using the existing client's reader or async stream API, with bounded buffers. Keep validation Markdown buffered, since validation requires its text. Removing the extra `.to_vec()` copy is a smaller interim change. Also avoid repeating directory-creation work for every file when the parent has already been created.

**Work / rewrite:** Small to medium, approximately 0.5-1 day in network/write helpers, overlapping with finding 1. Verify a large binary file, interrupted reads, and cleanup without changing bytes or counts.

**If skipped:** Large assets and added concurrency may trade latency gains for unnecessary peak memory and copies.

### 19. Every database open reruns initialization, and every record is committed separately

**Rating:** 2/5. **Applies to:** small-file batches, discovery bursts, and frequent refreshes.

**Evidence:** `db.rs:8-48` opens a connection, sets WAL mode, checks table creation, and attempts two already-applied `ALTER TABLE` migrations every time. Record functions at lines 53-103 execute individual inserts. The frontend's one-request invocations cause `lib.rs:69` to open again per skill; even a direct backend batch still inserts without an explicit transaction.

**Problem and concrete case:** A batch of 100 tiny skills repeats database initialization 100 times and makes 100 separate history commits, in addition to search and refresh opens. Duplicate-column errors from the migration attempts are routinely produced and ignored. Network latency will usually dominate today, so this is not the first performance change to make.

**Ideal proposed fix:** Run schema initialization/migrations once at startup using a schema version, and reuse a connection in a controlled blocking context or at least one connection per batch. Group completed history inserts into short transactions where the durability tradeoff is acceptable; do not hold a write transaction open during downloads. Keep the current best-effort history behavior and do not weaken SQLite durability settings just to make a benchmark faster.

**Work / rewrite:** Small to medium, approximately 0.5-1 day, partly covered by batching. Check migration of an old database, repeated startup, partial batch failure, and correct persisted outcomes. No database pool dependency is needed for this desktop workload.

**If skipped:** Once network waiting is reduced, repeated setup and commits become more noticeable for many tiny skills.

### 20. A destination conflict is detected only after downloading and validating Markdown

**Rating:** 2/5. **Applies to:** repeated or conflicting selections through either input.

**Evidence:** `github.rs:257-270` fetches Markdown before `resolve_github_install_name`. The resolver at lines 327-341 already correctly checks the plain name and then the owner-prefixed fallback.

**Problem and concrete case:** If both `pdf` and `owner-pdf` exist, an attempted install still performs a raw request and validation before reporting that no destination is available. This is more visible in manual URL results because installed skills are not currently supplied to that selector. The current rename behavior itself is implemented and is not the inefficiency.

**Ideal proposed fix:** Under the existing lock, run the cheap destination-name check before network work. If staging is later parallelized, treat that check as a reservation or recheck at commit time as described in finding 2. Retain owner-prefix behavior and never replace an existing destination to save time.

**Work / rewrite:** Small, approximately 1-2 hours with a fetch-count assertion for a guaranteed conflict. Touches one function; interacts with future lock changes.

**If skipped:** Guaranteed-to-fail installs retain one avoidable network round trip.

## Recommended order and expected reach

| Phase | Findings | Expected benefit | Approximate combined effort / rewrite |
| --- | --- | --- | --- |
| A. Remove repeat work | 3, 5's prefix hoist, 6, 9, 10, 20 | Fewer discovery/file requests; no duplicate source jobs; reuse HTTP pools. Benefits single and multi-skill workflows. | About 2-4 days. Shared discovery state and several contained installer/caller changes. |
| B. Reduce frontend work | 4, 8, 17 | Faster candidate selection and progress processing on large catalogs. Independent of the already-fixed background-thread issue. | About 2-4 days together. Queue and selection state changes; optional grouping of views. |
| C. Overlap safe network work | 1, 2, 18, cleanup from 16 | Largest likely improvement to successful install wall time, especially many small files. | About 3-5 days together. Coordinated Rust scheduling/progress change, not a full application rewrite. |
| D. Scale discovery and refresh | 5's index, 7, 11-14, 19 | Faster targeted URLs, large catalogs, larger installed libraries, and older histories. | About 4-7 days together. Discovery parsing/indexing plus filesystem/SQL and bounded history UI changes. |
| E. Optimize shared/failed transfers | 15 and remaining 16 | Avoid repeat bytes for nested skills and expensive restarts after transient failures. | About 1-3 additional days after C; depends on source identity and scheduling choices. |

The phases are a planning order, not a commitment to implement every finding. For a single Markdown-only skill, removing the second Markdown fetch is the clearest immediate win. For one skill with many files, bounded file concurrency matters most. For many skills from one catalog, shared discovery and deduplicated jobs remove work before bounded installation concurrency overlaps what remains. For mixed repositories, the batch scheduler and bounded discovery matter most.

A blanket switch to downloading whole repository archives is not the first recommendation: it can help dense selections, but can make a single small skill slower by downloading large unrelated content. Likewise, unrestricted frontend `Promise.all` does not bypass the current lock and is not a complete fix.

## Validation performed and evidence limits

- Read the Rust command layer, GitHub discovery/downloader, filesystem operations, database queries/schema, validator, and skills.sh adapter. Traced queue, URL-row, selection, progress, installed-list, history, and local-drop callers. Read application/build/dependency configuration and the existing test sources.
- Ran `npm test`: **10 tests passed**. These cover frontend invocation/progress behavior and installed-source matching, not real downloads or rendered queue performance.
- Ran `cargo test --offline --manifest-path src-tauri/Cargo.toml`: **4 tests passed**. These cover URL parsing, existing owner-prefix collision behavior, and agent-separated source metadata. Compilation emitted a nonfatal incremental-cache hardlink fallback warning; that is a local build issue, not an installation-runtime finding.
- Executed the current `mergeResults` function in isolation after TypeScript transpilation. Installed every candidate in order, starting from its usual null-filled result array. Instrumented array searches counted 5,130 old-result visits for 20 candidates, 661,650 for 100, and 5,313,300 for 200. These are operation counts from the source function, not browser frame timings.
- Executed the current `findInstalledGithubSkill` helper in isolation against 1,000 installed entries and 500 absent names. Instrumentation counted 500,000 installed-name reads and 500 repository URL constructions. Matching correctness remains covered by the passing existing tests.
- Inspected the locally installed Tauri 2.12.5 / command-macro 2.7.4 implementation to distinguish synchronous command handling from the explicit worker dispatch already present in search/install. No claim here depends on an assumed new Tauri scheduling feature.

### Checks to require when implementing improvements

Use isolated temporary skill directories and a local/mock HTTP source rather than altering real installed skills. Record discovery count, raw-file count, maximum active downloads, bytes transferred, peak memory, preparation time, install time, and refresh time separately.

| Scenario | What it isolates / required assertion |
| --- | --- |
| One Markdown-only skill through each input | Exactly one Markdown transfer after discovery; unchanged validation and install result. |
| One skill with 100 small support files | Bounded overlap improves wall time; file contents, atomic publication, and progress remain correct. |
| Ten distinct skills from one repository | One shared discovery for the preparation run; each selected source installed once. |
| Same candidate selected in two queue panels | One source job, both panels receive the outcome; no accidental second destination. |
| Mixed repositories and simultaneous URL/queue actions | Global download ceiling is respected; one slow job does not unnecessarily serialize unrelated downloads. |
| Same basename in different repositories/directories | Stable identities prevent result collisions; existing owner-prefix naming remains correct. |
| Explicit subdirectory URL in a large repository | No unrelated full-tree processing; slash-containing refs and root URLs still resolve correctly. |
| 200-1,000 candidates, with and without a large installed list | Completion and availability work scale as intended; checkbox selections and progress survive state updates. |
| 1,000 installed skills and 50,000 history records | Identity checks avoid full Markdown rereads; bounded history loads and latest-source queries stay bounded to relevant data. |
| Last-file failure, conflict, binary asset, and cancellation/replacement | No partial destination publication or overwritten skill; failures clean staging; obsolete discovery cannot overwrite current state. |

There are no current end-to-end performance tests covering these cases. No implementation changes or new permanent tests were added because this task requests an evaluation only.

## Verdict and limits

**Verdict:** Keep the existing background workers and safe collision handling. Remove repeated discovery and Markdown transfers, eliminate expensive frontend result searches, and then introduce bounded downloads with safe name allocation and streamed progress. Those changes address the shared bottlenecks in both install inputs without replacing the application.

**Lean:** Duplicate discovery, per-skill IPC calls, repeated array searches, and repeated schema setup can be removed. No new dependency is required for the core recommendations; a trustworthy net line-count reduction requires an actual implementation and is not estimated here.

**Not checked:** Live GitHub/skills.sh response times or current service quotas, release-build end-to-end timing, browser frame profiling, packet traces, real-user database contents, and OS-specific disk/antivirus effects. Installer/uninstaller lifecycle tests were read for context but not executed. No external network installs or modifications to real skill directories were performed.

**Risk / uncertainty:** Expected speed gains are inferred from confirmed request counts, loops, lock scope, and isolated operation probes. Their size needs measurement; concurrency and caching must preserve validation, external-change detection, source identity, destination collision safety, and atomic publication.
