//! Anomena compostos segons la nomenclatura del 1993 o, si hi escau, amb el seu nom propi
//!
//! Taula de prioritats
//! | Grup            | Simbòlic | Preferent   | Subsitutent     |
//! |------------------------------------------------------------|
//! | Àcid Carboxilic | -COOH    | -oic (àcid) | carboxi         |
//! | Èster           | -COOR    | -oat de $R  | ($R)oxicarbonil |
//! | Amida           | -CONH2   | -amida      | carbamoïl       |
//! | Nitril          | -CN      | -nitril     | ciano           |
//! | Aldehid         | -CHO     | -al         | formil          |
//! | Cetona          | -CO-     | -ona        | oxo             |
//! | Alcohol/Fenol   | -OH      | -ol         | hidroxi         |
//! | Amina           | -NH2     | -amina      | amino           |
//! | Èters           | R-O-R'   | (èter)      | ($R)oxi         |
//! | Halògens        | Cl,Br,F  | NaN         | ($R)            |

mod anomena;
pub use anomena::*;

use macroquad::prelude::*;
use macroquad::rand::*;
use std::collections::HashMap;
use std::rc::Rc;

pub type Id = u128;
pub const LINK_MARGIN_BETWEEN_RADICAL: f32 = 10.0;
pub const LINK_LINE_THICKNESS: f32 = 3.0;

pub struct UiState {
    pub uiblocks: Vec<UiBlock>,
    pub held: Option<Held>,
    pub is_help_up: bool,
    /// for undoing (DO NOT PUSH TO MANUALLY, USE `push_to_undo`)
    pub undo_list: Vec<UiAction>,
    /// for unundoing (DO NOT PUSH TO MANUALLY, USE `redo_last`)
    pub redo_list: Vec<UiAction>,
    /// (width, height)
    pub window_dims: (f32, f32),
    /// Text that shows the name of the molecule
    pub naming_text: Option<String>,
}

impl UiState {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            uiblocks: vec![],
            held: None,
            is_help_up: false,
            undo_list: vec![],
            redo_list: vec![],
            window_dims: (width, height),
            naming_text: None,
        }
    }
    pub fn push_to_undo(&mut self, a: UiAction) {
        self.undo_list.push(a);
        self.redo_list.clear();
    }
}

/// A building block of the UI; a node in the network
#[derive(Debug, Clone)]
pub struct UiBlock {
    pub pos: Vec2,
    pub radical: UiRadical,
    pub font: Rc<Font>,
    pub links: Vec<Id>,
    pub id: Id,
}

impl UiBlock {
    pub const PAD_V: f32 = 15.0;
    pub const PAD_H: f32 = 20.0;
    pub const LINK_CIRCLE_RADIUS: f32 = 7.0;
    pub const LINK_PAD: f32 = 10.0 + Self::LINK_CIRCLE_RADIUS;
    pub const FONT_SIZE: u16 = 30;
    pub const SPACING: f32 = 0.5; // I don't know what this does but raylib asks for it
    pub const ROUNDNESS: f32 = 0.75;
    pub const SEGMENTS: i32 = 20;
    pub const LINE_THICKNESS: f32 = 4.0;

    pub fn dims(&self) -> TextDimensions {
        let text = self.radical.to_string();
        measure_text(&text, Some(&*self.font), Self::FONT_SIZE, 1.0)
    }
    pub fn center(&self) -> Vec2 {
        let (width, height) = (self.dims().width, self.dims().height);
        let x = self.pos.x - Self::PAD_H + f32::midpoint(Self::PAD_H * 2.0, width);
        let y = self.pos.y - height / 2.0;
        Vec2 { x, y }
    }

    /// returns [Up Left Down Right]
    pub fn general_link_positions(&self) -> [Vec2; 4] {
        let (width, height) = (self.dims().width, self.dims().height);
        let Vec2 {
            x: mid_width,
            y: mid_height,
        } = self.center();

        let left = Vec2 {
            x: self.pos.x - Self::PAD_H - Self::LINK_PAD,
            y: mid_height,
        };
        let right = Vec2 {
            x: self.pos.x + Self::PAD_H + Self::LINK_PAD + width,
            y: mid_height,
        };
        let up = Vec2 {
            x: mid_width,
            y: self.pos.y - Self::PAD_V - Self::LINK_PAD - height,
        };
        let down = Vec2 {
            x: mid_width,
            y: self.pos.y + Self::PAD_V + Self::LINK_PAD,
        };

        [up, left, down, right]
    }
    pub fn link_positions(&self) -> Vec<Vec2> {
        let a_poss = self.general_link_positions();
        let avail = self
            .radical
            .valencia()
            .wrapping_sub(self.links.len() as u32);
        let l = match self.radical.valencia() {
            1 => vec![a_poss[3]],
            2 => vec![a_poss[1], a_poss[3]],
            3 => vec![a_poss[0], a_poss[1], a_poss[3]],
            4 => a_poss.to_vec(),
            l => {
                eprintln!("ERROR: molecula has {l} valencies, que no és ni 1 ni 2 ni 4");
                a_poss.to_vec()
            }
        };
        l.into_iter().take(avail as usize).collect()
    }

    pub fn count_links(blocks: &[UiBlock]) -> HashMap<(Id, Id), usize> {
        let mut links: HashMap<(Id, Id), usize> = HashMap::new();
        for block in blocks {
            for l_id in &block.links {
                let (a, b) = (block.id.min(*l_id), block.id.max(*l_id));
                links.entry((a, b)).and_modify(|c| *c += 1).or_insert(1);
            }
        }
        for m in links.values_mut() {
            *m /= 2;
        } // don't double count
        links
    }

    /// Includes padding
    pub fn bounding_rect(&self) -> Rect {
        Rect {
            x: self.pos.x - Self::PAD_H,
            y: self.pos.y - (self.dims().height + Self::PAD_V),
            w: self.dims().width + 2.0 * Self::PAD_H,
            h: self.dims().height + 2.0 * Self::PAD_V,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Held {
    /// `.0` is of the form Vec<(id, from)>
    // /// `origin` is where the mouse was originally when the holding down was initiated
    Radicals(Vec<(Id, Vec2)>),
    Link {
        radical: Id,
        from: Vec2,
    },
    RectangleCreation {
        from: Vec2,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UiRadical {
    F,
    Cl,
    Br,
    C,
    Amina,
    Eter,
    Fenol,
    Alcohol,
    Cetona,
    Aldehid,
    Nitril,
    Amida,
    Ester,
    Carboxil,
    //Amonia,
}

impl UiRadical {
    pub fn valencia(&self) -> u32 {
        use UiRadical as R;
        match self {
            R::C => 4,
            R::F | R::Cl | R::Br => 1,
            R::Amina => 1,
            R::Fenol => 1,
            R::Alcohol => 1,
            R::Cetona => 2,
            R::Aldehid => 1,
            R::Nitril => 1,
            R::Amida => 1,
            R::Ester => 2,
            R::Carboxil => 1,
            R::Eter => 2,
        }
    }
    pub fn contains_carbon(&self) -> bool {
        use UiRadical as R;
        match self {
            R::C
            | R::Fenol
            | R::Amida
            | R::Ester
            | R::Carboxil
            | R::Cetona
            | R::Aldehid
            | R::Nitril => true,
            R::F | R::Cl | R::Br | R::Amina | R::Alcohol | R::Eter => false,
        }
    }
    pub fn contains_nitrogen(&self) -> bool {
        use UiRadical as R;
        match self {
            R::Amina | R::Amida | R::Nitril => true,
            R::F
            | R::Cl
            | R::Br
            | R::Alcohol
            | R::Eter
            | R::C
            | R::Fenol
            | R::Ester
            | R::Carboxil
            | R::Cetona
            | R::Aldehid => false,
        }
    }
    pub fn contains_oxygen(&self) -> bool {
        use UiRadical as R;
        match self {
            R::Carboxil | R::Ester | R::Cetona | R::Aldehid | R::Alcohol | R::Fenol | R::Eter => {
                true
            }
            R::Amina | R::Amida | R::Nitril | R::F | R::Cl | R::Br | R::C => false,
        }
    }
}
impl std::fmt::Display for UiRadical {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let s = match self {
            Self::C => "C",
            Self::F => "F",
            Self::Cl => "Cl",
            Self::Br => "Br",
            Self::Amina => "NH2",
            Self::Fenol => "Fenol",
            Self::Alcohol => "OH",
            Self::Cetona => "CO",
            Self::Aldehid => "CHO",
            Self::Nitril => "CN",
            Self::Amida => "CONH2",
            Self::Ester => "COO",
            Self::Carboxil => "COOH",
            Self::Eter => "O",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone)]
pub enum UiAction {
    AddRadical(UiBlock),
    DeleteRadical(UiBlock),
    /// (id, from, to)
    MoveRadicals(Vec<(Id, Vec2, Vec2)>),
    AddLink(Id, Id),
    DeleteLink(Id, Id),
}

impl UiAction {
    pub fn opposite(&self) -> Self {
        match self {
            Self::AddRadical(what) => Self::DeleteRadical(what.clone()),
            Self::DeleteRadical(what) => Self::AddRadical(what.clone()),
            Self::MoveRadicals(data) => {
                Self::MoveRadicals(data.iter().map(|(i, f, t)| (*i, *t, *f)).collect())
            }
            Self::AddLink(a, b) => Self::DeleteLink(*b, *a),
            Self::DeleteLink(a, b) => Self::AddLink(*b, *a),
        }
    }
}

pub fn get_block_under_point(bs: &[UiBlock], cursor: Vec2) -> Option<&UiBlock> {
    bs.iter().find(|b| b.bounding_rect().contains(cursor))
}

pub fn remove_hanging_links(blocks: &mut Vec<UiBlock>, del_id: Id) {
    for b in blocks {
        b.links.retain(|&l| l != del_id)
    }
}

pub fn link_node_at_point(
    blocks: &[UiBlock],
    curr_mouse_pos: Vec2,
    threshold: f32,
) -> Option<(Id, Vec2)> {
    for b in blocks {
        for c in b.link_positions() {
            let circle = Circle {
                x: c.x,
                y: c.y,
                r: UiBlock::LINK_CIRCLE_RADIUS + threshold,
            };
            if circle.contains(&curr_mouse_pos) {
                return Some((b.id, c));
            }
        }
    }
    None
}

// TODO: The uiblocks should be sorted so these can be searched through with
// binary search, probably
pub fn get_block_unchecked(blocks: &[UiBlock], id: Id) -> &UiBlock {
    blocks
        .iter()
        .find(|b| b.id == id)
        .unwrap_or_else(|| panic!("Block unexpectedly disappeared: {id}"))
}
pub fn get_two_blocks_unchecked_mut(
    blocks: &mut [UiBlock],
    id_a: Id,
    id_b: Id,
) -> (&mut UiBlock, &mut UiBlock) {
    let a = blocks
        .iter()
        .position(|bl| bl.id == id_a)
        .unwrap_or_else(|| panic!("Block unexpectedly disappeared: {id_a}"));
    let b = blocks
        .iter()
        .position(|bl| bl.id == id_b)
        .unwrap_or_else(|| panic!("Block unexpectedly disappeared: {id_b}"));
    let [r1, r2] = blocks
        .get_disjoint_mut([a, b])
        .expect("Invalid indices to unchecked function");
    (r1, r2)
}
pub fn get_block_unchecked_mut(blocks: &mut [UiBlock], id: Id) -> &mut UiBlock {
    blocks
        .iter_mut()
        .find(|b| b.id == id)
        .unwrap_or_else(|| panic!("Block unexpectedly disappeared: {id}"))
}

pub fn get_points_for_link(b: &UiBlock, bp: &UiBlock) -> [Vec2; 2] {
    let TextDimensions {
        width: b_w,
        height: b_h,
        ..
    } = b.dims();
    let TextDimensions {
        width: bp_w,
        height: bp_h,
        ..
    } = bp.dims();
    let b_c = b.center();
    let bp_c = bp.center();
    let m = LINK_MARGIN_BETWEEN_RADICAL;

    //let angle = ;

    // What follows is incredibly scuffed
    // We divide the line into three cases (per rectangle): closest to short, closest
    // to long and closest to corner.

    // Assuming we're 'close' to top right corner
    // A is the 'bottom' point of the rounding circle
    // B is the 'left' point of the rounding circle
    // i.e. they're both the same distance away from the corner (m)
    //
    // +--------B--+
    // |           |
    // |           A
    // |           |
    // |           |
    // +-----------+
    let a = Vec2 {
        x: b_c.x + b_w / 2.0,
        y: b_c.y + b_h / 2.0 - m,
    };
    let b = Vec2 {
        x: b_c.x + b_w / 2.0 - m,
        y: b_c.y + b_h / 2.0,
    };
    let alpha = (a.y / a.x).atan();
    let beta = (b.y / b.x).atan();

    // TODO: replace this with actual logic
    [b_c, bp_c]
}

pub fn undo_last(st: &mut UiState) {
    let Some(action) = st.undo_list.pop() else {
        return;
    };
    eprintln!("dbg: Undoing last action: {action:x?}");

    undo_action(st, action.clone());
    st.redo_list.push(action.opposite());
}
pub fn redo_last(st: &mut UiState) {
    let Some(action) = st.redo_list.pop() else {
        return;
    };
    eprintln!("dbg: REdoing last action: {action:x?}");

    undo_action(st, action.clone());
    st.undo_list.push(action.opposite());
}

fn undo_action(st: &mut UiState, action: UiAction) {
    match action {
        UiAction::AddRadical(what) => {
            if let Some(index) = st.uiblocks.iter().position(|b| b.id == what.id) {
                st.uiblocks.remove(index);
                remove_hanging_links(&mut st.uiblocks, what.id);
            }
        }
        UiAction::MoveRadicals(data) => {
            for (id, from, to) in data {
                if let Some(block) = st.uiblocks.iter_mut().find(|b| b.id == id) {
                    block.pos = from;
                }
            }
        }
        UiAction::DeleteRadical(what) => st.uiblocks.push(what),
        UiAction::AddLink(a_id, b_id) => {
            let (a, b) = get_two_blocks_unchecked_mut(&mut st.uiblocks, a_id, b_id);
            if let Some(i) = a.links.iter().position(|&l| l == b_id) {
                a.links.remove(i);
            }
            if let Some(i) = b.links.iter().position(|&l| l == a_id) {
                b.links.remove(i);
            }
        }
        UiAction::DeleteLink(a_id, b_id) => {
            let (a, b) = get_two_blocks_unchecked_mut(&mut st.uiblocks, a_id, b_id);
            a.links.push(b_id);
            b.links.push(a_id);
        }
    }
}

pub fn delete_under_cursor(st: &mut UiState, curr_mouse_pos: Vec2) {
    if let Some(index) = st
        .uiblocks
        .iter()
        .position(|b| b.bounding_rect().contains(curr_mouse_pos))
    {
        let what = st.uiblocks.remove(index);
        remove_hanging_links(&mut st.uiblocks, what.id);
        st.push_to_undo(UiAction::DeleteRadical(what));
    } else {
        // remove link if it exists
        for ((a_id, b_id), m) in UiBlock::count_links(&st.uiblocks) {
            let a = get_block_unchecked(&st.uiblocks, a_id);
            let b = get_block_unchecked(&st.uiblocks, b_id);
            if cursor_on_link(curr_mouse_pos, a, b, m) {
                remove_link(&mut st.uiblocks, a_id, b_id);
                st.push_to_undo(UiAction::DeleteLink(a_id, b_id));
            }
        }
    }
}

/// Does nothing if the links don't exist
pub fn remove_link(blocks: &mut [UiBlock], a_id: Id, b_id: Id) {
    let (a, b) = get_two_blocks_unchecked_mut(blocks, a_id, b_id);
    if let (Some(i_a), Some(i_b)) = (
        a.links.iter().position(|&l| l == b_id),
        b.links.iter().position(|&l| l == a_id),
    ) {
        a.links.remove(i_a);
        b.links.remove(i_b);
    } else {
        eprintln!("Tried to remove a link between {a_id:x} and {b_id:x}, but there was none");
    }
}

pub fn cursor_on_link(mouse: Vec2, a: &UiBlock, b: &UiBlock, multiplicitat: usize) -> bool {
    let [c1, c2] = get_points_for_link(a, b);
    // Assuming the line flows from c1->c2

    let slope = (c2.y - c1.y) / (c2.x - c1.x);
    let max_dist = (multiplicitat as f32 * LINK_LINE_THICKNESS * 2.0 - 1.0) * 1.5; // give 50% margin
    let dist = {
        // line<->point distance
        let (a, b, c) = (slope, -1.0, c1.y - slope * c1.x);
        ((a * mouse.x + b * mouse.y + c) / (a * a + b * b).sqrt()).abs()
    };
    let is_in_between = c1.x.min(c2.x) <= mouse.x
        && mouse.x <= c1.x.max(c2.x)
        && c1.y.min(c2.y) <= mouse.y
        && mouse.y <= c1.y.max(c2.y);

    dist <= max_dist && is_in_between
}

pub fn generate_random_id() -> Id {
    // lmao
    // assumes Id = u128
    (rand() as u128) << 32 * 3
        | (rand() as u128) << 32 * 2
        | (rand() as u128) << 32 * 1
        | (rand() as u128) << 32 * 0
}
