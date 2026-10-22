# Native Wayland transport review evidence

Original protocol-level regression harness for the winit 0.31 adapter draft.
It runs a small isolated Wayland server and a subprocess client, not the user's
compositor. It creates only synthetic temporary file paths/data. No contributor
artwork or Adobe-generated fixtures are distributed.

From this directory on Linux (Rust 1.95, Wayland and libdbus development packages):

```sh
cargo test --test protocol native_file_drop_protocol -- --exact --nocapture
```

That parent test runs valid and overlapping offers, text clipboard, cancellation,
unsupported MIME/actions, empty/remote/NUL URI lists, oversized data and a stalled
writer; it checks response matching, hover cleanup and bounded completion.
The subprocess helper is ignored when invoked directly.

`native_file_drop_portal` is a separate ignored integration case. It requires a
real document portal. Run portal/Flatpak integration in a separate OS session or
VM, **not alongside a user's applications**. Earlier private-session-bus Flatpak
tests disturbed the live portal mount; private D-Bus/XDG_RUNTIME_DIR alone do not
provide sufficient isolation. The safe command above does not run this case.

Passing this harness establishes protocol and adapter behavior; it is not a
physical file-manager gesture test, a compositor interoperability matrix, a
sandbox permission grant or a release approval. The parent PR documents the
separately confirmed downstream Desktop and image-import results.

This evidence branch is deliberately separate from the upstream migration diff.
Dependency revisions match that candidate; upstream API/release agreements are
still required before the prototype dependencies can ship.
