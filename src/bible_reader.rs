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
}
