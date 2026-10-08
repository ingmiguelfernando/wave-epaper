//! Paging model for the Bible reading view: which chapter and page are open,
//! and where ▲▼ take the reader. Pages come from the caller, so this stays
//! free of drawing and of SD access.

/// Where the reader is: an index into the book list, a chapter and a page.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Position {
    pub book: usize,
    pub chapter: u16,
    pub page: usize,
}

/// Outcome of a page turn: stay on the chapter, or move to another one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Turn {
    /// The page changed inside the chapter.
    Page,
    /// Forward past the last page: the next chapter's first page.
    NextChapter,
    /// Backward before the first page: the previous chapter's last page.
    PreviousChapter,
    /// No chapter to move to (first or last of the Bible); nothing changed.
    Edge,
}

/// Book and chapter counts, for crossing chapter and book boundaries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookShape {
    pub chapters: u16,
}

/// Move one page forward or backward. `pages` is the page count of the open
/// chapter; `books` lists every book's chapter count in canonical order.
pub fn turn(position: &mut Position, forward: bool, pages: usize, books: &[BookShape]) -> Turn {
    if forward {
        if position.page + 1 < pages {
            position.page += 1;
            return Turn::Page;
        }
        if position.chapter < books[position.book].chapters {
            position.chapter += 1;
            position.page = 0;
            return Turn::NextChapter;
        }
        if position.book + 1 < books.len() {
            position.book += 1;
            position.chapter = 1;
            position.page = 0;
            return Turn::NextChapter;
        }
        Turn::Edge
    } else {
        if position.page > 0 {
            position.page -= 1;
            return Turn::Page;
        }
        if position.chapter > 1 {
            position.chapter -= 1;
            // The caller re-pages the previous chapter and sets its last page.
            position.page = usize::MAX;
            return Turn::PreviousChapter;
        }
        if position.book > 0 {
            position.book -= 1;
            position.chapter = books[position.book].chapters;
            position.page = usize::MAX;
            return Turn::PreviousChapter;
        }
        Turn::Edge
    }
}

/// Split laid-out lines into pages of at most `lines_per_page` lines. An empty
/// chapter still has one page, so the reader always has something to show.
pub fn paginate(lines: usize, lines_per_page: usize) -> usize {
    if lines == 0 || lines_per_page == 0 {
        1
    } else {
        lines.div_ceil(lines_per_page)
    }
}

/// One drawn line of a chapter: a heading, or a verse line. A verse's first
/// line carries its number, drawn small and raised.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Line {
    Heading(String),
    /// `number` is set only on the verse's first line; `paragraph` starts a
    /// new paragraph above it.
    Verse {
        number: Option<String>,
        paragraph: bool,
        text: String,
    },
}

/// Lay a chapter's items out as lines of at most `width` pixels, measuring
/// with `measure`. Headings and verses wrap; a verse's number stays on its
/// first line only.
pub fn layout(
    items: &[crate::bible::ChapterItem],
    width: i32,
    first_width: i32,
    measure: impl Fn(&str) -> i32,
) -> Vec<Line> {
    let mut lines = Vec::new();
    for item in items {
        match item {
            crate::bible::ChapterItem::Heading(text) => {
                lines.push(Line::Heading(text.clone()));
            }
            crate::bible::ChapterItem::Verse {
                label,
                paragraph,
                text,
            } => {
                // The first line also holds the verse number and indent, so it
                // wraps to the narrower `first_width`; the rest use `width`.
                for (index, piece) in wrap_verse(text, first_width, width, &measure)
                    .into_iter()
                    .enumerate()
                {
                    lines.push(Line::Verse {
                        number: (index == 0).then(|| label.clone()),
                        paragraph: index == 0 && *paragraph,
                        text: piece,
                    });
                }
            }
        }
    }
    lines
}

/// Greedy word wrap of one verse: the first line fits `first_width`, every
/// later line fits `width`. An over-long word keeps its own line, as
/// `UiTextStyle::wrap` does.
fn wrap_verse(
    text: &str,
    first_width: i32,
    width: i32,
    measure: &impl Fn(&str) -> i32,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut limit = first_width;
    for word in text.split_whitespace() {
        let candidate = if line.is_empty() {
            word.to_owned()
        } else {
            format!("{line} {word}")
        };
        if line.is_empty() || measure(&candidate) <= limit {
            line = candidate;
        } else {
            lines.push(core::mem::replace(&mut line, word.to_owned()));
            limit = width;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn books() -> Vec<BookShape> {
        vec![BookShape { chapters: 2 }, BookShape { chapters: 3 }]
    }

    #[test]
    fn pages_move_inside_a_chapter_and_stop_at_its_edges_by_turning() {
        let mut at = Position {
            book: 0,
            chapter: 1,
            page: 0,
        };
        assert_eq!(turn(&mut at, true, 2, &books()), Turn::Page);
        assert_eq!(at.page, 1);
        assert_eq!(turn(&mut at, false, 2, &books()), Turn::Page);
        assert_eq!(at.page, 0);
    }

    #[test]
    fn forward_past_the_last_page_opens_the_next_chapter() {
        let mut at = Position {
            book: 0,
            chapter: 1,
            page: 1,
        };
        assert_eq!(turn(&mut at, true, 2, &books()), Turn::NextChapter);
        assert_eq!(
            at,
            Position {
                book: 0,
                chapter: 2,
                page: 0
            }
        );
    }

    #[test]
    fn forward_past_the_last_chapter_opens_the_next_book() {
        let mut at = Position {
            book: 0,
            chapter: 2,
            page: 0,
        };
        assert_eq!(turn(&mut at, true, 1, &books()), Turn::NextChapter);
        assert_eq!(
            at,
            Position {
                book: 1,
                chapter: 1,
                page: 0
            }
        );
    }

    #[test]
    fn backward_before_the_first_page_opens_the_previous_chapters_last_page() {
        let mut at = Position {
            book: 0,
            chapter: 2,
            page: 0,
        };
        assert_eq!(turn(&mut at, false, 2, &books()), Turn::PreviousChapter);
        assert_eq!(at.chapter, 1);
        // The caller clamps this to the chapter's real last page.
        assert_eq!(at.page, usize::MAX);
    }

    #[test]
    fn backward_from_the_first_chapter_of_a_book_crosses_to_the_last_chapter_before() {
        let mut at = Position {
            book: 1,
            chapter: 1,
            page: 0,
        };
        assert_eq!(turn(&mut at, false, 1, &books()), Turn::PreviousChapter);
        assert_eq!(
            at,
            Position {
                book: 0,
                chapter: 2,
                page: usize::MAX
            }
        );
    }

    #[test]
    fn the_first_and_last_chapter_of_the_bible_have_no_neighbour() {
        let mut first = Position {
            book: 0,
            chapter: 1,
            page: 0,
        };
        assert_eq!(turn(&mut first, false, 1, &books()), Turn::Edge);
        assert_eq!(
            first,
            Position {
                book: 0,
                chapter: 1,
                page: 0
            }
        );
        let mut last = Position {
            book: 1,
            chapter: 3,
            page: 0,
        };
        assert_eq!(turn(&mut last, true, 1, &books()), Turn::Edge);
        assert_eq!(
            last,
            Position {
                book: 1,
                chapter: 3,
                page: 0
            }
        );
    }

    #[test]
    fn paginate_rounds_up_and_keeps_one_page_for_empty_text() {
        assert_eq!(paginate(0, 20), 1);
        assert_eq!(paginate(20, 20), 1);
        assert_eq!(paginate(21, 20), 2);
        assert_eq!(paginate(40, 0), 1);
    }

    /// Every character is one pixel wide, so line breaks are easy to predict.
    fn by_chars(text: &str) -> i32 {
        text.chars().count() as i32
    }

    fn verse(label: &str, paragraph: bool, text: &str) -> crate::bible::ChapterItem {
        crate::bible::ChapterItem::Verse {
            label: label.into(),
            paragraph,
            text: text.into(),
        }
    }

    #[test]
    fn a_verse_number_stays_on_its_first_line_only() {
        let items = [verse("3", false, "uno dos tres cuatro")];
        let lines = layout(&items, 9, 9, by_chars);
        assert!(lines.len() > 1, "the verse wraps");
        assert!(matches!(&lines[0], Line::Verse { number: Some(n), .. } if n == "3"));
        assert!(lines[1..]
            .iter()
            .all(|line| matches!(line, Line::Verse { number: None, .. })));
    }

    #[test]
    fn a_heading_is_one_line_and_a_paragraph_flag_marks_only_the_first_line() {
        let items = [
            crate::bible::ChapterItem::Heading("Salmo".into()),
            verse("1", true, "uno dos tres cuatro"),
        ];
        let lines = layout(&items, 9, 9, by_chars);
        assert_eq!(lines[0], Line::Heading("Salmo".into()));
        let paragraphs = lines
            .iter()
            .filter(|line| {
                matches!(
                    line,
                    Line::Verse {
                        paragraph: true,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(paragraphs, 1, "only the first line starts the paragraph");
    }

    #[test]
    fn wrapping_keeps_every_word_in_order() {
        let text = "uno dos tres cuatro cinco seis";
        let items = [verse("2", false, text)];
        let rejoined: Vec<String> = layout(&items, 9, 9, by_chars)
            .into_iter()
            .filter_map(|line| match line {
                Line::Verse { text, .. } => Some(text),
                Line::Heading(_) => None,
            })
            .collect();
        assert_eq!(rejoined.join(" "), text);
    }

    #[test]
    fn a_long_word_keeps_its_own_line_without_being_split() {
        let items = [verse("1", false, "ab abcdefghijklmnop cd")];
        let texts: Vec<String> = layout(&items, 5, 5, by_chars)
            .into_iter()
            .filter_map(|line| match line {
                Line::Verse { text, .. } => Some(text),
                Line::Heading(_) => None,
            })
            .collect();
        assert!(texts.contains(&"abcdefghijklmnop".to_owned()));
    }
}
