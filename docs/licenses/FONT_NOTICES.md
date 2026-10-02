# Generated bitmap-font notices

Wave embeds 1-bpp bitmap atlases rasterized by `scripts/fonts/generate.py`
(run through the `fonts` GitHub workflow). Each atlas covers printable ASCII,
Latin-1 and common typographic punctuation (see `src/charset.rs`). The
repository contains only the generated Rust arrays; raw font files are not
distributed.

| Atlas file | Source font |
| --- | --- |
| `src/app/typography/assets.rs` | Inter (Medium, SemiBold); Atkinson Hyperlegible (Regular, Bold) |
| `src/app/reader_atkinson_next_assets.rs` | Atkinson Hyperlegible Next (Medium) |
| `src/app/reader_literata_assets.rs` | Literata (Medium) |
| `src/app/reader_serif_assets.rs` | DejaVu Serif 2.37 |

Inter, Atkinson Hyperlegible, Atkinson Hyperlegible Next and Literata are taken
from the Google Fonts repository at a pinned commit and are used under the SIL
Open Font License 1.1, retained in `docs/licenses/OFL-1.1.txt`.

DejaVu changes are public domain; the base Bitstream Vera permission notice is
retained in `docs/licenses/DEJAVU-SERIF-NOTICE.txt`.
