# Hyphenation notices

The Reader hyphenates Spanish and English text with the
[`hypher`](https://github.com/typst/hypher) crate 0.1.8 (© Laurenz Mädje,
MIT OR Apache-2.0). Only its Spanish and English pattern automata are compiled
into the firmware (`default-features = false`, see `Cargo.toml`). They are
built from these hyph-utf8 pattern files:

| Language | Pattern file | Copyright | License |
| --- | --- | --- | --- |
| Spanish | `hyph-es.tex` 5.0 (2019-09-24) | © 1993, 1997, 2001-2019 Javier Bezos, CervanTeX | MIT/X11 |
| English | `hyph-en-us.tex` (2005-05-30) | © 1990, 2004, 2005 Gerard D.C. Kuiken | see below |

## Spanish patterns (MIT/X11)

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

## English patterns

Copyright (C) 1990, 2004, 2005 Gerard D.C. Kuiken.

Copying and distribution of this file, with or without modification, are
permitted in any medium without royalty provided the copyright notice and this
notice are preserved.
