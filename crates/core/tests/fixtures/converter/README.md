# Conversion reference matrix

The sample contains user messages, assistant answers, thinking, tool output,
system context, compaction, a synthetic error, Unicode, and fixed timestamps.

`reference_matrix.json` was generated using the unmodified Python imports,
exports, and providers at `b5894dae5feaf13947c2d49a34f91b088cde60ec` in temporary
data directories. It includes each of the five source adapters and all 20
directed conversions, both with and without tools (45 reference results).

Only generated UUIDs/tool IDs and timestamp representation were normalized.
The Rust test compares every exported role, subtype, label, text, tool, status,
code and timestamp with Python, then opens the imported conversation and asks
for editor previews for every editable item.

This tests compatibility with the original converter, including its intentional
provider-specific representations. It does not assert that unrelated AI clients
have identical internal execution state.
