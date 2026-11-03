# Structured bulk operations and previews

Use `apply_batch` for structured MCP mutations. To validate a candidate without
committing it, add `"dryRun": true` to the same request:

```json
{
  "projectId": "p_example",
  "expectedRevision": 7,
  "idempotencyKey": "rename-title-v1",
  "dryRun": true,
  "operations": [
    {"op":"update_node","id":"title","props":{"text":"New title","fontSize":32}}
  ]
}
```

A successful preview returns `projectId`, `dryRun:true`, `baseRevision`, the
would-be `changedIds`, and `diagnostics:[]`. Invalid candidates return the same
validation error as a commit. Diagnostics are structural; preview does not render
or measure text. It validates request bounds, existing key collisions, revision,
the ordered candidate, references, and the final model under the project writer
lock. It does not increment revision, write a commit/idempotency record, publish
events, touch assets, or enqueue prerender. Existing matching keys do not short
circuit a preview: the current revision and candidate are still checked.

Commit by sending the identical request with `dryRun` omitted or false. A preview
does not reserve the key. Another writer can commit between preview and commit;
handle `REVISION_CONFLICT` using `changedSince` and rebase with a fresh key.

For an existing operation file, use the MCP file tool:

```json
{
  "projectId": "p_example",
  "expectedRevision": 7,
  "idempotencyKey": "rename-title-v1",
  "path": "rename.json",
  "dryRun": true
}
```

`rename.json` is a pure JSON array containing exactly the operations above. The
relative path starts inside `<data-root>/<project-slug>/inbox/`, never the process
working directory. Nested inbox directories are supported. An absolute path is
accepted only when it names that canonical inbox; paths through aliases such as
macOS `/var` instead of `/private/var` are rejected. Prefer relative filenames.
New projects have an empty inbox. For older projects, a caller with filesystem
access must create it locally; the file tool never creates it or uploads a file.

Only regular files are read. Traversal, symlink files/directories, paths outside
the project's inbox, directories and special files fail with `PATH_REJECTED`.
A missing input returns `FILE_NOT_FOUND`. Errors never echo a private absolute
path. Descriptor-relative no-follow opens, a regular-file preflight, nonblocking
open and a post-open regular-file check protect the read. Secure file input is
supported on Unix (the project targets macOS); other platforms return
`UNSUPPORTED_PLATFORM` without reading the file.

Each input contains 1–500 operations. The file is capped at 2 MiB before reading;
the bounded stream is checked again to cover file growth. The normalized complete
request has the existing 2 MiB batch limit too, including request metadata.
Operations use the exact published `apply_batch` schema. JavaScript, generators,
or executable expressions are never evaluated by this tool.

There is no automatic chunking or partial commit. Split larger designs into
explicit batches, each with its own expected revision and key. Every successful
batch increments revision once. Inline and file mutations use the same canonical
request hash and transaction path: whitespace, object key order and filename do
not affect replay. A changed operation array with an existing key returns
`IDEMPOTENCY_KEY_REUSED`; file changes cannot accidentally replay an old commit.
After a lost response, keep the key, revision and operation content unchanged.
