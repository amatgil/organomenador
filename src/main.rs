use macroquad::color::colors::*;
use macroquad::prelude::*;
use macroquad::rand::*;
use organomenar::*;
use std::fmt::Write;
use std::rc::Rc;

const APL387_BYTES: &[u8] = include_bytes!("../APL387.ttf");
const HELP_TEXT_FONTSIZE: u16 = 20;
const LINK_CIRCLE_CLICKING_THRESHOLD: f32 = 20.0;
const KEYMAP: [(KeyCode, UiRadical); 15] = [
    (KeyCode::C, UiRadical::C),
    (KeyCode::Q, UiRadical::C),
    (KeyCode::W, UiRadical::Carboxil),
    (KeyCode::E, UiRadical::Ester),
    (KeyCode::R, UiRadical::Amida),
    (KeyCode::T, UiRadical::Nitril),
    (KeyCode::Y, UiRadical::Aldehid),
    (KeyCode::U, UiRadical::Cetona),
    (KeyCode::I, UiRadical::Alcohol),
    (KeyCode::O, UiRadical::Fenol),
    (KeyCode::P, UiRadical::Amina),
    (KeyCode::A, UiRadical::Eter),
    (KeyCode::S, UiRadical::Br),
    (KeyCode::D, UiRadical::Cl),
    (KeyCode::F, UiRadical::F),
];

// TODO: when hovering over a tag, make its connected graph glow/get highlighted
// TODO: Maybe, bulk select to move?
// TODO: add a readme

#[macroquad::main("organomenador")]
async fn main() {
    // SETUP
    let apl387 =
        Rc::new(load_ttf_font_from_bytes(APL387_BYTES).expect("Font is static, cannot panic"));

    // TODO: Set maximum fps
    let help_text = {
        let mut t = String::new();
        for (k, r) in KEYMAP {
            _ = write!(
                t,
                "{} -> Afegeix {}\n",
                format!("{k:?}").replace("KEY_", ""),
                r
            );
        }
        t.push_str("X -> Elimina\n");
        t.push_str("N -> Anomena mol. sota cursor\n");
        t.push_str("Z -> Undo/Desfer\n");
        t.push_str("(Els enllaços buits se consideren H)");
        t
    };

    let mut st = UiState {
        uiblocks: vec![],
        held: None,
        is_help_up: false,
        undo_list: vec![],
        redo_list: vec![],
        window_dims: (screen_width(), screen_height()),
        naming_text: None,
    };

    let mut curr_mouse_pos = Vec2 {
        x: mouse_position().0,
        y: mouse_position().1,
    };
    let mut mouse_delta: Vec2;

    loop {
        use UiBlock as B;

        mouse_delta = Vec2::from(mouse_position()) - curr_mouse_pos;
        curr_mouse_pos = mouse_position().into();

        // ==== Adjust positions when rescaling ====
        let rescale_factor = Vec2 {
            x: screen_width() as f32 / st.window_dims.0 as f32,
            y: screen_height() as f32 / st.window_dims.1 as f32,
        };
        st.window_dims = (screen_width(), screen_height());
        for block in &mut st.uiblocks {
            block.pos *= rescale_factor
        }
        for action in &mut st.undo_list {
            if let UiAction::MoveRadicals(data) = action {
                for (id, from, to) in data {
                    *from *= rescale_factor;
                    *to *= rescale_factor;
                }
            }
        }
        if let Some(Held::Radicals(data)) = &mut st.held {
            for (_, f) in data {
                *f *= rescale_factor
            }
        } else if let Some(Held::Link { from, .. }) = &mut st.held {
            *from *= rescale_factor
        }

        // ===== Handle clicking =====
        match (&st.held, is_mouse_button_down(MouseButton::Left)) {
            (None, true) => {
                if let Some(index) = st
                    .uiblocks
                    .iter()
                    .position(|b| is_point_in_block(curr_mouse_pos, b))
                {
                    st.held = Some(Held::Radicals(vec![(
                        st.uiblocks[index].id,
                        st.uiblocks[index].pos,
                    )]));
                } else if let Some((radical, from)) =
                    link_node_at_point(&st.uiblocks, curr_mouse_pos, LINK_CIRCLE_CLICKING_THRESHOLD)
                {
                    st.held = Some(Held::Link { radical, from });
                } else {
                    st.held = Some(Held::RectangleCreation {
                        from: curr_mouse_pos,
                    });
                }
            }
            (None, false) => {}
            (Some(Held::Radicals(data)), true) => {
                for (id, from) in data {
                    if let Some(b) = st.uiblocks.iter_mut().find(|b| b.id == *id) {
                        b.pos += mouse_delta;
                    };
                }
            }
            (Some(Held::Radicals(data)), false) => {
                let mut new_data = vec![];
                for (id, from) in data {
                    let to = get_block_unchecked(&st.uiblocks, *id).pos;
                    new_data.push((*id, *from, to));
                }
                st.push_to_undo(UiAction::MoveRadicals(new_data));
                st.held = None;
            }
            (Some(Held::Link { .. }), true) => {}
            (
                Some(Held::Link {
                    radical: source_id, ..
                }),
                false,
            ) => {
                if let Some((dest_id, ..)) =
                    link_node_at_point(&st.uiblocks, curr_mouse_pos, LINK_CIRCLE_CLICKING_THRESHOLD)
                {
                    let (source, dest) =
                        get_two_blocks_unchecked_mut(&mut st.uiblocks, *source_id, dest_id);
                    source.links.push(dest_id);
                    dest.links.push(*source_id);
                    st.push_to_undo(UiAction::AddLink(*source_id, dest_id));
                }
                st.held = None;
            }
            (Some(Held::RectangleCreation { .. }), true) => {}
            (Some(Held::RectangleCreation { .. }), false) => st.held = None,
        }
        if is_mouse_button_down(MouseButton::Right) {
            match st.held {
                Some(Held::Radicals(data)) => {
                    _ = data
                        .iter()
                        .for_each(|(id, f)| get_block_unchecked_mut(&mut st.uiblocks, *id).pos = *f)
                }
                Some(Held::Link { .. } | Held::RectangleCreation { .. }) | None => {}
            }
            st.held = None;
        }

        // ===== Handle keypresses =====
        for (_, radical) in KEYMAP.iter().filter(|(k, _)| is_key_pressed(*k)) {
            let r = 10;
            let rand_delta = Vec2 {
                x: gen_range(-r, r) as f32,
                y: gen_range(-r, r) as f32,
            };
            let b = UiBlock {
                pos: curr_mouse_pos + rand_delta,
                radical: *radical,
                font: apl387.clone(),
                id: generate_random_id(),
                links: vec![],
            };
            st.push_to_undo(UiAction::AddRadical(b.clone()));
            st.uiblocks.push(b);
        }
        if is_key_pressed(KeyCode::Z) && !is_key_down(KeyCode::LeftShift) {
            undo_last(&mut st)
        }
        if is_key_pressed(KeyCode::Z) && is_key_down(KeyCode::LeftShift) {
            redo_last(&mut st)
        }
        if is_key_pressed(KeyCode::X) {
            delete_under_cursor(&mut st, curr_mouse_pos)
        }
        if is_key_pressed(KeyCode::H) {
            st.is_help_up = !st.is_help_up
        }
        if is_key_pressed(KeyCode::N) {
            if let Some(b) = get_block_under_point(&st.uiblocks, curr_mouse_pos) {
                st.naming_text = Some(anomena(&st.uiblocks, b));
            } else {
                st.naming_text = Some("No he trobat res sota el cursor :c".to_string());
            }
        } else if get_last_key_pressed().is_some() || is_mouse_button_down(MouseButton::Left) {
            st.naming_text = None
        }

        //  ===== Drawing and such =====
        clear_background(WHITE);
        // links first
        for ((a_id, b_id), m) in UiBlock::count_links(&st.uiblocks) {
            let b = get_block_unchecked(&st.uiblocks, a_id);
            let bp = get_block_unchecked(&st.uiblocks, b_id);
            let [a, b] = get_points_for_link(b, bp);

            // This is cheating! For drawing double/triple bonds >:3
            for (i, t) in (1..=(2 * m - 1)).rev().step_by(2).enumerate() {
                let color = if i % 2 != 0 { WHITE } else { BLACK };
                draw_line(a.x, a.y, b.x, b.y, LINK_LINE_THICKNESS * (t as f32), color);
            }
        }

        // radicals last
        for block in &st.uiblocks {
            // TODO: make these rounded
            //draw_rectangle_rounded(
            //    Rectangle {
            //        w: block.dims().width + 2.0 * B::PAD_H,
            //        h: block.dims().height + 2.0 * B::PAD_V,
            //        x: block.pos.x - B::PAD_H,
            //        y: block.pos.y - B::PAD_V,
            //    },
            //    B::ROUNDNESS,
            //    B::SEGMENTS,
            //    WHITE,
            //);
            draw_rectangle(
                block.pos.x - B::PAD_H,
                block.pos.y - B::PAD_V,
                block.dims().width + 2.0 * B::PAD_H,
                block.dims().height + 2.0 * B::PAD_V,
                WHITE,
            );
            draw_rectangle_lines(
                block.pos.x - B::PAD_H,
                block.pos.y - B::PAD_V,
                block.dims().width + 2.0 * B::PAD_H,
                block.dims().height + 2.0 * B::PAD_V,
                B::LINE_THICKNESS,
                BLACK,
            );

            draw_text_ex(
                &block.radical.to_string(),
                block.pos.x,
                block.pos.y,
                TextParams {
                    font: Some(&*apl387),
                    font_size: B::FONT_SIZE,
                    color: BLACK,
                    ..Default::default()
                },
            );

            for center in block.link_positions() {
                draw_circle(center.x, center.y, B::LINK_CIRCLE_RADIUS, SKYBLUE);
            }
        }

        if st.naming_text.is_none() {
            draw_text_ex(
                "H: Ajuda",
                10.0,
                st.window_dims.1 - HELP_TEXT_FONTSIZE as f32 / 2.0,
                TextParams {
                    font: Some(&*apl387),
                    font_size: HELP_TEXT_FONTSIZE,
                    color: BLACK,
                    ..Default::default()
                },
            );
        }

        if st.is_help_up {
            let baseline = HELP_TEXT_FONTSIZE as f32;
            let (t_width, t_height) =
                measure_multiline_text(&help_text, Some(&*apl387), HELP_TEXT_FONTSIZE, todo!());

            draw_rectangle(
                0.0,
                0.0,
                t_width,
                t_height,
                Color {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 0.5,
                },
            );
            draw_multiline_text_ex(
                &help_text,
                10.0,
                baseline + 5.0,
                None,
                TextParams {
                    font: Some(&*apl387),
                    font_size: HELP_TEXT_FONTSIZE,
                    color: BLACK,
                    ..Default::default()
                },
            );
        }
        if let Some(Held::Link { from, .. }) = st.held {
            draw_line(
                from.x,
                from.y,
                curr_mouse_pos.x,
                curr_mouse_pos.y,
                LINK_LINE_THICKNESS,
                BLACK,
            );
        } else if let Some(Held::RectangleCreation { from }) = st.held {
            let to = curr_mouse_pos;
            // TODO: roundedness?
            draw_rectangle(
                from.x.min(to.x),
                from.y.min(to.y),
                (from.x - to.x).abs(),
                (from.y - to.y).abs(),
                Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.5,
                    a: 0.5,
                },
            );
        }
        if let Some(text) = &st.naming_text {
            let text_dims = measure_text(&text, Some(&*apl387), B::FONT_SIZE, 1.0);
            draw_text_ex(
                text,
                st.window_dims.0 as f32 / 2.0 - text_dims.width / 2.0,
                st.window_dims.1 as f32 - text_dims.height,
                TextParams {
                    font: Some(&*apl387),
                    font_size: B::FONT_SIZE,
                    font_scale: 1.0,
                    font_scale_aspect: 1.0,
                    rotation: 0.0,
                    color: BLACK,
                },
            );
        }
        next_frame().await;
    }
}

// Mandatory font because i'm only gonna use this with my font
fn measure_multiline_text(
    text: &str,
    font: &Font,
    font_size: u16,
    // Spacing between lines
    spacing: f32,
) -> (f32, f32) {
    let font_line_distance = match font.font.horizontal_line_metrics(1.0) {
        Some(metrics) => metrics.new_line_size,
        None => font_size,
    };

    let lines: Vec<&str> = text.split('\n').collect();

    // Height of a single line
    let m = measure_text("Ay", Some(font), font_size, 1.0);
    let line_height = m.height;

    let total_height = lines.len() as f32 * line_height;

    // Width = widest line
    let max_width = lines
        .iter()
        .map(|line| measure_text(line, Some(font), font_size, 1.0).width)
        .fold(0.0, f32::max);

    (max_width, total_height)
}
