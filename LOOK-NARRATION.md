# LOOK narration companion

Native EPUB read-along for Bookokrat 0.3.14, retaining PDF and SVG reading.
The upstream AGPL license is retained; packages include the license and source
commit reference.

| Key sequence | Action |
|---|---|
| Space, r | Read aloud; pause/resume |
| Space, R | Read from the current visual paragraph |
| Space, x | Stop |
| Space, v / V | Next voice / automatic default voice |
| Space, - / + | Slower / faster, 0.5–2.0× |
| q | Quit and stop this reader's speech |
| ? | Native Help, including configurable narration bindings |

Bookokrat highlights the narrated block's wrapped lines and follows it across
paragraphs and chapters. Manual chapter changes stop the old narration.
Visual bookmarks keep their native node addresses; they are not translated into
LOOK browser speech bookmarks.

LOOK supplies local voices through a bounded JSON-line worker. Voice and speed
choices share the local LOOK speech preference. Text stays on this computer.
The helper stops only its own speech processes. LOOK sets the current helper and
Python paths; standalone launches use `~/.local/share/look/book_speech.py`.
Missing support gives a HUD error. Native narration currently targets EPUB;
PDF reading remains available. No package or voice installs automatically.

Build: `cargo build --locked --release` (default PDF/SVG features).
Tests: `cargo test --locked --lib narration` and `cargo test --locked --lib keybindings`.
