use organomenar::*;
use raylib::prelude::*;
use std::fmt::Write;
use std::rc::Rc;

const APL387_BYTES: &[u8] = include_bytes!("../APL387.ttf");
const HELP_TEXT_FONTSIZE: i32 = 25;
const LINK_CIRCLE_CLICKING_THRESHOLD: f32 = 20.0;
const KEYMAP: [(KeyboardKey, UiRadical); 15] = [
    (KeyboardKey::KEY_C, UiRadical::C),
    (KeyboardKey::KEY_Q, UiRadical::C),
    (KeyboardKey::KEY_W, UiRadical::Carboxil),
    (KeyboardKey::KEY_E, UiRadical::Ester),
    (KeyboardKey::KEY_R, UiRadical::Amida),
    (KeyboardKey::KEY_T, UiRadical::Nitril),
    (KeyboardKey::KEY_Y, UiRadical::Aldehid),
    (KeyboardKey::KEY_U, UiRadical::Cetona),
    (KeyboardKey::KEY_I, UiRadical::Alcohol),
    (KeyboardKey::KEY_O, UiRadical::Fenol),
    (KeyboardKey::KEY_P, UiRadical::Amina),
    (KeyboardKey::KEY_A, UiRadical::Eter),
    (KeyboardKey::KEY_S, UiRadical::Br),
    (KeyboardKey::KEY_D, UiRadical::Cl),
    (KeyboardKey::KEY_F, UiRadical::F),
];

// TODO: when hovering over a tag, make its connected graph glow/get highlighted
// TODO: Maybe, bulk select to move?

fn main() {
    unsafe { raylib::ffi::SetConfigFlags(ConfigFlags::FLAG_MSAA_4X_HINT as u32) };
    unsafe { raylib::ffi::SetConfigFlags(ConfigFlags::FLAG_WINDOW_RESIZABLE as u32) };

    // SETUP
    let (mut rl, thread) = raylib::init().size(1000, 700).title("Organomenar").build();

    let apl387 = Rc::new(
        rl.load_font_from_memory(&thread, ".ttf", APL387_BYTES, UiBlock::FONT_SIZE, None)
            .expect("Cannot fail, font is loaded in at compile time"),
    );
    let apl387_help = rl
        .load_font_from_memory(&thread, ".ttf", APL387_BYTES, HELP_TEXT_FONTSIZE, None)
        .expect("Cannot fail, font is loaded in at compile time");

    rl.set_target_fps(120);
    let help_text = {
        let mut t = String::new();
        for (k, r) in KEYMAP {
            write!(
                t,
                "{} -> Afegeix {}\n",
                format!("{k:?}").replace("KEY_", ""),
                r
            );
        }
        t.push_str("X -> Elimina\n");
        t.push_str("N -> Anomena mol. sota cursor\n");
        t.push_str("Z -> Undo/Desfer\n");
        // TODO: Find out why all non-ascii becomes question marks??? I
        // geniunely don't understand why even draw_text_codepoints doesn't
        // render them properly
        t.push_str("(Els enllaços buits se consideren H)");
        t
    };

    let mut st = UiState {
        uiblocks: vec![],
        held: None,
        is_help_up: false,
        undo_list: vec![],
        redo_list: vec![],
        window_dims: (rl.get_render_width(), rl.get_render_height()),
        naming_text: None,
    };

    let mut curr_mouse_pos = rl.get_mouse_position();
    let mut mouse_delta: Vector2;
    while !rl.window_should_close() {
        use UiBlock as B;

        mouse_delta = rl.get_mouse_position() - curr_mouse_pos;
        curr_mouse_pos = rl.get_mouse_position();

        // ==== Adjust positions when rescaling ====
        let rescale_factor = Vector2 {
            x: rl.get_render_width() as f32 / st.window_dims.0 as f32,
            y: rl.get_render_height() as f32 / st.window_dims.1 as f32,
        };
        st.window_dims = (rl.get_render_width(), rl.get_render_height());
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
        match (
            &st.held,
            rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT),
        ) {
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
        if rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_RIGHT) {
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
        for (_, radical) in KEYMAP.iter().filter(|(k, _)| rl.is_key_pressed(*k)) {
            let r = 10;
            let rand_delta = Vector2 {
                x: rand::random_range(-r..=r) as f32,
                y: rand::random_range(-r..=r) as f32,
            };
            let b = UiBlock {
                pos: curr_mouse_pos + rand_delta,
                radical: *radical,
                font: apl387.clone(),
                id: rand::random(),
                links: vec![],
            };
            st.push_to_undo(UiAction::AddRadical(b.clone()));
            st.uiblocks.push(b);
        }
        if rl.is_key_pressed(KeyboardKey::KEY_Z) && !rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) {
            undo_last(&mut st)
        }
        if rl.is_key_pressed(KeyboardKey::KEY_Z) && rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) {
            redo_last(&mut st)
        }
        if rl.is_key_pressed(KeyboardKey::KEY_X) {
            delete_under_cursor(&mut st, curr_mouse_pos)
        }
        if rl.is_key_pressed(KeyboardKey::KEY_H) {
            st.is_help_up = !st.is_help_up
        }
        if rl.is_key_pressed(KeyboardKey::KEY_N) {
            if let Some(b) = get_block_under_point(&st.uiblocks, curr_mouse_pos) {
                st.naming_text = Some(anomena(&st.uiblocks, b));
            } else {
                st.naming_text = Some("No he trobat res sota el cursor :c".to_string());
            }
        } else if rl.get_key_pressed().is_some()
            || rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT)
        {
            st.naming_text = None
        }

        //  ===== Drawing and such =====
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::WHITE);

        // links first
        for ((a_id, b_id), m) in UiBlock::count_links(&st.uiblocks) {
            let b = get_block_unchecked(&st.uiblocks, a_id);
            let bp = get_block_unchecked(&st.uiblocks, b_id);
            let [a, b] = get_points_for_link(b, bp);

            // This is cheating! For drawing double/triple bonds >:3
            for (i, t) in (1..=(2 * m - 1)).rev().step_by(2).enumerate() {
                let color = if i % 2 != 0 {
                    Color::WHITE
                } else {
                    Color::BLACK
                };
                d.draw_line_ex(a, b, LINK_LINE_THICKNESS * (t as f32), color);
            }
        }

        // radicals last
        for block in &st.uiblocks {
            d.draw_rectangle_rounded(
                Rectangle {
                    width: block.dims().x + 2.0 * B::PAD_H,
                    height: block.dims().y + 2.0 * B::PAD_V,
                    x: block.pos.x - B::PAD_H,
                    y: block.pos.y - B::PAD_V,
                },
                B::ROUNDNESS,
                B::SEGMENTS,
                Color::WHITE,
            );
            d.draw_rectangle_rounded_lines_ex(
                Rectangle {
                    width: block.dims().x + 2.0 * B::PAD_H,
                    height: block.dims().y + 2.0 * B::PAD_V,
                    x: block.pos.x - B::PAD_H,
                    y: block.pos.y - B::PAD_V,
                },
                B::ROUNDNESS,
                B::SEGMENTS,
                B::LINE_THICKNESS,
                Color::BLACK,
            );

            d.draw_text_ex(
                &*apl387,
                &block.radical.to_string(),
                block.pos,
                B::FONT_SIZE as f32,
                B::SPACING,
                Color::BLACK,
            );

            for center in block.link_positions() {
                d.draw_circle_v(center, B::LINK_CIRCLE_RADIUS, Color::ROYALBLUE);
            }
        }

        if st.naming_text.is_none() {
            d.draw_text_ex(
                &*apl387,
                "H: Ajuda",
                Vector2 {
                    x: 5.0,
                    y: (st.window_dims.1 - HELP_TEXT_FONTSIZE) as f32,
                },
                HELP_TEXT_FONTSIZE as f32,
                B::SPACING,
                Color::BLACK,
            );
        }

        if st.is_help_up {
            let dims = apl387_help.measure_text(&help_text, HELP_TEXT_FONTSIZE as f32, B::SPACING);
            d.draw_rectangle(
                0,
                0,
                dims.x as i32,
                dims.y as i32,
                Color::new(255u8, 255u8, 255u8, 100u8),
            );
            d.draw_text_ex(
                &apl387_help,
                &help_text,
                Vector2 { x: 5.0, y: 5.0 },
                HELP_TEXT_FONTSIZE as f32,
                B::SPACING,
                Color::BLACK,
            );
        }
        if let Some(Held::Link { from, .. }) = st.held {
            d.draw_line_ex(from, curr_mouse_pos, LINK_LINE_THICKNESS, Color::BLACK);
        } else if let Some(Held::RectangleCreation { from }) = st.held {
            let to = curr_mouse_pos;
            d.draw_rectangle_rounded(
                Rectangle {
                    x: from.x.min(to.x),
                    y: from.y.min(to.y),
                    width: (from.x - to.x).abs(),
                    height: (from.y - to.y).abs(),
                },
                B::ROUNDNESS,
                B::SEGMENTS,
                Color {
                    r: 0,
                    g: 0,
                    b: 180,
                    a: 100,
                },
            );
        }
        if let Some(text) = &st.naming_text {
            let text_dims = (*apl387).measure_text(text, B::FONT_SIZE as f32, B::SPACING);
            d.draw_text_ex(
                &*apl387,
                text,
                Vector2 {
                    x: (st.window_dims.0 as f32 / 2.0 - text_dims.x / 2.0),
                    y: (st.window_dims.1 as f32 - text_dims.y),
                },
                B::FONT_SIZE as f32,
                B::SPACING,
                Color::BLACK,
            );
        }
    }
}
