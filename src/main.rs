use macroquad::color::colors::*;
use macroquad::prelude::*;
use macroquad::rand::*;
use organomenador::*;
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

use UiBlock as B;

// TODO: when hovering over a tag, make its connected graph glow/get highlighted
// TODO: Maybe, bulk select to move?
// TODO: add a readme

fn window_conf() -> Conf {
    Conf {
        window_title: "organomenador".to_owned(),
        fullscreen: false,
        sample_count: 4,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
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

    let mut st = UiState::new(screen_width(), screen_height());

    let mut curr_mouse_pos = mouse_position().into();
    let mut mouse_delta: Vec2;

    loop {
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
                for (_id, from, to) in data {
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
            // We just clicked leftclick
            (None, true) => {
                if let Some(index) = st
                    .uiblocks
                    .iter()
                    .position(|b| b.bounding_rect().contains(curr_mouse_pos))
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

            // Nothing happening
            (None, false) => {}

            // We're dragging some radicals
            (Some(Held::Radicals(data)), true) => {
                for (id, _from) in data {
                    get_block_unchecked_mut(&mut st.uiblocks, *id).pos += mouse_delta;
                }
            }

            // We've _stopped_ dragging some radicals
            (Some(Held::Radicals(data)), false) => {
                let mut new_data = vec![];
                for (id, from) in data {
                    let to = get_block_unchecked(&st.uiblocks, *id).pos;
                    new_data.push((*id, *from, to));
                }
                st.push_to_undo(UiAction::MoveRadicals(new_data));
                st.held = None;
            }

            // We're holding a link
            (Some(Held::Link { .. }), true) => {}

            // We've stopped holding a link
            (Some(Held::Link { radical: s_id, .. }), false) => {
                if let Some((dest_id, ..)) =
                    link_node_at_point(&st.uiblocks, curr_mouse_pos, LINK_CIRCLE_CLICKING_THRESHOLD)
                {
                    if *s_id != dest_id {
                        let (source, dest) =
                            get_two_blocks_unchecked_mut(&mut st.uiblocks, *s_id, dest_id);
                        source.links.push(dest_id);
                        dest.links.push(*s_id);
                        st.push_to_undo(UiAction::AddLink(*s_id, dest_id));
                    }
                }
                st.held = None;
            }

            // We're holding a selection
            (Some(Held::RectangleCreation { .. }), true) => {}

            // We've stoped holding a selection
            (Some(Held::RectangleCreation { .. }), false) => st.held = None,
        }
        if is_mouse_button_down(MouseButton::Right) {
            // TODO: if over a radical, interpret that as initiating holding a link
            match st.held {
                Some(Held::Radicals(data)) => {
                    _ = data.iter().for_each(|(id, from)| {
                        get_block_unchecked_mut(&mut st.uiblocks, *id).pos = *from
                    })
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
            draw_uiblock(block, &apl387);
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
            let TextDimensions {
                width: t_width,
                height: t_height,
                ..
            } = measure_multiline_text(&help_text, Some(&*apl387), HELP_TEXT_FONTSIZE, 1.0, None);

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
            draw_naming_text(text, &apl387, &st);
        }
        next_frame().await;
    }
}

fn draw_naming_text(text: &String, apl387: &Rc<Font>, st: &UiState) {
    let text_dims = measure_text(&text, Some(&**apl387), B::FONT_SIZE, 1.0);
    draw_text_ex(
        text,
        st.window_dims.0 as f32 / 2.0 - text_dims.width / 2.0,
        st.window_dims.1 as f32 - text_dims.height,
        TextParams {
            font: Some(&**apl387),
            font_size: B::FONT_SIZE,
            font_scale: 1.0,
            font_scale_aspect: 1.0,
            rotation: 0.0,
            color: BLACK,
        },
    );
}

fn draw_uiblock(block: &UiBlock, font: &Rc<Font>) {
    let text = &block.radical.to_string();
    let font = Some(&**font);

    let r = block.bounding_rect();

    draw_rectangle(r.x, r.y, r.w, r.h, WHITE);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, B::LINE_THICKNESS, BLACK);
    draw_text_ex(
        text,
        block.pos.x,
        block.pos.y,
        TextParams {
            font,
            font_size: B::FONT_SIZE,
            color: BLACK,
            ..Default::default()
        },
    );

    for center in block.link_positions() {
        draw_circle(center.x, center.y, B::LINK_CIRCLE_RADIUS, SKYBLUE);
    }
}
