//! Photos: the gallery grid and the full-screen viewer.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{DrawTarget, Drawable, Pixel, Point, Primitive, Size},
    primitives::{
        Circle, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, StrokeAlignment, Triangle,
    },
};

use crate::{
    app::{
        state::AppState,
        typography::{Text, UiTextStyle},
        widgets::{
            bottom_bar::{draw_bottom_bar, KeyCap},
            header::draw_header,
            list_row::{draw_list_row, ListRow, LIST_ROW_HEIGHT},
            status_row::{draw_status_row, StatusRow},
        },
    },
    orientation::OrientedFrameBuffer,
    photos::{
        image::{Thumbnail, THUMB_HEIGHT, THUMB_WIDTH},
        ui::{PhotoAction, PhotoStatus},
    },
};

const GRID_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "move"),
    (KeyCap::Select, "view"),
    (KeyCap::Boot, "star"),
];

const ACTION_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "choose"),
    (KeyCap::Select, "apply"),
    (KeyCap::Boot, "hold: close"),
];

const DELETE_HINTS: [(KeyCap, &str); 2] =
    [(KeyCap::Select, "delete"), (KeyCap::Boot, "hold: cancel")];

const COLUMNS: [i32; 3] = [12, 168, 324];
const ROWS: [i32; 2] = [134, 390];
const THUMB_SIZE: Size = Size::new(THUMB_WIDTH as u32, THUMB_HEIGHT as u32);
/// Top of the action and delete panels in the viewer.
const PANEL_TOP: i32 = 430;

pub fn render_photos(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let photos = &state.photos;
    let count = match photos.photos.len() {
        1 => "1 photo".to_string(),
        count => format!("{count} photos"),
    };
    let starred = format!("{} starred", photos.starred_count());
    let page = format!("{}/{}", photos.page() + 1, photos.page_count());

    draw_header(display, state.display, "PHOTOS", "/RUSTMIX/PHOTOS ON THE SD CARD")?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: &count,
            middle: &starred,
            right: &page,
        },
    )?;
    if let Some(note) = &photos.note {
        draw_empty(display, state, note)?;
    } else {
        for (slot, index) in photos.visible().enumerate() {
            draw_cell(display, state, slot, index)?;
        }
        if let Some(info) = photos.info_label() {
            let body = state.display.body_style();
            let info = body.fit(&info, 456);
            Text::new(&info, Point::new(12, 690), body).draw(display)?;
        }
    }
    draw_bottom_bar(display, state.display, &GRID_HINTS)?;
    Ok(())
}

fn draw_empty(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    note: &str,
) -> Result<(), Infallible> {
    let heading = state.display.heading_style();
    let body = state.display.body_style();
    Text::new(note, Point::new(22, 200), heading).draw(display)?;
    let help = "Copy JPEG photos into a PHOTOS folder at the top of the SD card. \
                Each one is prepared once, then shows here.";
    let mut baseline = 250;
    for line in body.wrap(help, 436) {
        Text::new(&line, Point::new(22, baseline), body).draw(display)?;
        baseline += i32::from(body.line_height()) + 4;
    }
    Ok(())
}

fn draw_cell(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    slot: usize,
    index: usize,
) -> Result<(), Infallible> {
    let photos = &state.photos;
    let photo = &photos.photos[index];
    let detail = state.display.detail_style();
    let top_left = Point::new(COLUMNS[slot % 3], ROWS[slot / 3]);
    match photos.thumbnail(photo.key()) {
        Some(thumbnail) => draw_thumbnail(display, top_left, thumbnail)?,
        None => {
            let label = match photos.status[index] {
                PhotoStatus::Failed(_) => "Can't open",
                _ => "Preparing",
            };
            Rectangle::new(top_left, THUMB_SIZE)
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                .draw(display)?;
            let left = top_left.x + (THUMB_WIDTH as i32 - detail.text_width(label)) / 2;
            Text::new(label, Point::new(left, top_left.y + 112), detail).draw(display)?;
        }
    }
    if photos.starred.contains(&photo.name) {
        draw_badge(display, top_left + Point::new(THUMB_WIDTH as i32 - 20, 20))?;
    }
    if index == photos.selected {
        let frame = PrimitiveStyleBuilder::new()
            .stroke_color(BinaryColor::On)
            .stroke_width(4)
            .stroke_alignment(StrokeAlignment::Outside)
            .build();
        Rectangle::new(top_left, THUMB_SIZE)
            .into_styled(frame)
            .draw(display)?;
    }
    let stem = photo
        .name
        .rsplit_once('.')
        .map_or(photo.name.as_str(), |(stem, _)| stem);
    let name = detail.fit(stem, THUMB_WIDTH as i32);
    let baseline = top_left.y + THUMB_HEIGHT as i32 + 20;
    Text::new(&name, Point::new(top_left.x, baseline), detail).draw(display)?;
    Ok(())
}

fn draw_thumbnail(
    display: &mut OrientedFrameBuffer<'_>,
    top_left: Point,
    thumbnail: &Thumbnail,
) -> Result<(), Infallible> {
    let ink = (0..THUMB_HEIGHT)
        .flat_map(|y| (0..THUMB_WIDTH).map(move |x| (x, y)))
        .filter(|&(x, y)| thumbnail.is_black(x, y))
        .map(|(x, y)| Pixel(top_left + Point::new(x as i32, y as i32), BinaryColor::On));
    display.draw_iter(ink)
}

/// A white star on a black disc, marking photos in the sleep set.
fn draw_badge(display: &mut OrientedFrameBuffer<'_>, center: Point) -> Result<(), Infallible> {
    Circle::with_center(center, 30)
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(display)?;
    draw_star(display, center, 11, BinaryColor::Off)
}

/// A five-pointed star; the fonts have no ★ glyph.
fn draw_star(
    display: &mut OrientedFrameBuffer<'_>,
    center: Point,
    radius: i32,
    color: BinaryColor,
) -> Result<(), Infallible> {
    let corner = |index: i32, len: f32| {
        let angle = (index as f32 * 36.0 - 90.0).to_radians();
        center + Point::new((angle.cos() * len) as i32, (angle.sin() * len) as i32)
    };
    let fill = PrimitiveStyle::with_fill(color);
    let inner = radius as f32 * 0.4;
    for tip in (0..10).step_by(2) {
        let left = corner(tip - 1, inner);
        let right = corner(tip + 1, inner);
        Triangle::new(corner(tip, radius as f32), left, right)
            .into_styled(fill)
            .draw(display)?;
        Triangle::new(center, left, right)
            .into_styled(fill)
            .draw(display)?;
    }
    Ok(())
}

pub fn render_photo_viewer(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let photos = &state.photos;
    let Some(photo) = photos.selected_photo() else {
        return Ok(());
    };
    let body = state.display.body_style();
    match &photos.viewer.frame {
        Some(frame) => display.copy_native_frame(frame),
        None => {
            let message = match &photos.status[photos.selected] {
                PhotoStatus::Failed(reason) => format!("{} {reason}", photo.name),
                _ => "Preparing this photo...".to_string(),
            };
            let mut baseline = 400;
            for line in body.wrap(&message, 436) {
                Text::new(&line, Point::new(22, baseline), body).draw(display)?;
                baseline += i32::from(body.line_height()) + 4;
            }
        }
    }

    // A white bar on top keeps the position readable over any photo.
    Rectangle::new(Point::zero(), Size::new(480, 44))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))
        .draw(display)?;
    Rectangle::new(Point::new(0, 44), Size::new(480, 2))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(display)?;
    let position = format!(
        "{} / {} \u{b7} {}",
        photos.selected + 1,
        photos.photos.len(),
        photo.name
    );
    let position = body.fit(&position, 410);
    Text::new(&position, Point::new(12, 30), body).draw(display)?;
    if photos.starred.contains(&photo.name) {
        draw_star(display, Point::new(456, 22), 13, BinaryColor::On)?;
    }

    if photos.viewer.confirm_delete {
        draw_delete_confirmation(display, state, &photo.name)?;
    } else if let Some(highlighted) = photos.viewer.action {
        draw_actions(display, state, highlighted)?;
    }
    Ok(())
}

fn draw_panel(display: &mut OrientedFrameBuffer<'_>, top: i32) -> Result<(), Infallible> {
    Rectangle::new(Point::new(0, top), Size::new(480, (800 - top) as u32))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))
        .draw(display)?;
    Rectangle::new(Point::new(0, top), Size::new(480, 3))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(display)?;
    Ok(())
}

fn draw_actions(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    highlighted: usize,
) -> Result<(), Infallible> {
    let photos = &state.photos;
    let starred = photos
        .selected_photo()
        .is_some_and(|photo| photos.starred.contains(&photo.name));
    draw_panel(display, PANEL_TOP)?;
    let detail = state.display.detail_style();
    Text::new("PHOTO", Point::new(22, PANEL_TOP + 30), detail).draw(display)?;
    if let Some(info) = photos.info_label() {
        let info = detail.fit(&info, 436);
        Text::new(&info, Point::new(22, PANEL_TOP + 56), detail).draw(display)?;
    }
    for (index, action) in PhotoAction::ALL.into_iter().enumerate() {
        let row = ListRow {
            title: action.label(starred),
            subtitle: "",
            value: "",
            selected: index == highlighted,
        };
        let top = PANEL_TOP + 70 + index as i32 * LIST_ROW_HEIGHT;
        draw_list_row(display, state.display, top, row)?;
    }
    draw_bottom_bar(display, state.display, &ACTION_HINTS)?;
    Ok(())
}

fn draw_delete_confirmation(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    name: &str,
) -> Result<(), Infallible> {
    let top = 560;
    draw_panel(display, top)?;
    let heading: UiTextStyle = state.display.heading_style();
    let body = state.display.body_style();
    let question = heading.fit(&format!("Delete {name}?"), 436);
    Text::new(&question, Point::new(22, top + 50), heading).draw(display)?;
    let note = "The file is removed from the SD card.";
    Text::new(note, Point::new(22, top + 90), body).draw(display)?;
    draw_bottom_bar(display, state.display, &DELETE_HINTS)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use embedded_graphics::prelude::Point;

    use crate::{
        app::{render_current_screen, AppState, ScreenRoute},
        buttons::ButtonEvent,
        framebuffer::FrameBuffer,
        photos::{
            test_photos::grey_jpeg,
            ui::PhotosUiState,
            worker::{prepare, PhotoJob},
        },
    };

    #[test]
    fn grid_and_viewer_render_prepared_and_pending_photos() {
        let root = std::env::temp_dir().join(format!("wave-photo-screens-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let photos = root.join("PHOTOS");
        fs::create_dir_all(&photos).unwrap();
        for name in ["A.jpg", "B.jpg"] {
            fs::write(photos.join(name), grey_jpeg(96, 64, None)).unwrap();
        }
        let mut state = AppState::default();
        state.photos = PhotosUiState::with_roots(&photos, root.join("CACHE"));
        state.photos.refresh();
        let job: PhotoJob = state.photos.cache_jobs().remove(0);
        let result = prepare(&photos, state.photos.cache_directory(), &job);
        state.photos.on_job_result(&result);
        state.photos.toggle_star();
        state.router.navigate_to(ScreenRoute::Photos);

        let mut frame = FrameBuffer::new_white();
        render_current_screen(&mut frame, &state).unwrap();
        // The selected cell's outer frame (logical x 9, y 200) is drawn.
        assert_eq!(frame.is_black(Point::new(200, 479 - 9)), Some(true));

        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::PhotoViewer);
        state.apply(ButtonEvent::Select);
        render_current_screen(&mut frame, &state).unwrap();
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::PhotoViewer);
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::Photos);
        let _ = fs::remove_dir_all(root);
    }
}
