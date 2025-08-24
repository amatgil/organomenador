use crate::*;

const PREFIX_SEGONS_MIDA: [&str; 11] = [
    "?", "met", "et", "prop", "but", "pent", "hexa", "hept", "oct", "non", "dec",
];

// TODO: Press N or something to get the name of the molecule you're hovering over (i.e. only name
//       its connected graph). Print "nothing under cursor" instead if there's nothing there
//
// Return value may also be an error message to be displayed directly (too lazy to make a proper
// error enum and impl Display on it)
pub fn anomena(input: &[UiBlock], source: &UiBlock) -> String {
    let index = input
        .iter()
        .position(|b| b.id == source.id)
        .expect("block existed and then didn't in the same frame");
    let Ok(molecula) = find_connex(input, index) else {
        return "ERR: La molecula conté un cicle :c".to_string();
    };
    let links = UiBlock::count_links(&molecula);

    if !molecula.iter().any(|b| b.radical.contains_carbon()) {
        return "ERR: La molecula (sota el cursor) ha de contindre carboni".to_string();
    }
    return "Encara noooo".to_string();
    return todo!();

    // 1. S'ha de triar la funció principal a partir de l'ordre de prioritat.
    // 2. S'ha de triar la cadena principal aplicant les normes, en l'ordre en què figuren a la llista, fins trobar-ne una que decideixi, en cas de dues o més cadenes iguals:
    //    a) aquella que conté el grup principal
    //    b) aquella que té més grups principals
    //    c) aquella més insaturada (amb més dobles i triples enllaços en conjunt)
    //    d) aquella més llarga
    //    e) aquella que té més dobles enllaços
    //    f) la que presenti els localitzadors més baixos i en l'ordre de prioritat que s'indica per:
    //       1) grups principals
    //       2) enllaços múltiples en conjunt
    //       3) dobles enllaços
    //    g) la que tengui el major número possible de substituents i, en cas d'igualtat, la que els assigni els números més baixos
    // 3. S'ha de numerar la cadena principal, des d'un extrem i assignant els números més baixos, en
    // ordre d'importància, a:
    //    a) al(s) grup(s) principal(s)
    //    b) a dobles i triples enllaços en conjunt. En cas d'igualtat, als dobles enllaços.
    //    c) als substituents; en igualtat de condicions tenen preferència segons l'ordre alfabètic.
    // 4. Es forma el nom, començant pels substituents en ordre alfabètic o de complexitat; a
    // continuació la cadena principal acabada amb la terminació del grup principal

    // TODO: ASSUMEIXO HIDROCARBUR PER ARA

    // 1. Trio funció principal
    let principal = {
        let v = molecula.iter().map(|b| b.radical).collect::<Vec<_>>();
        v.sort();
        v.first()
            .expect("cannot be empty, we're given at least one block of it")
    };
}

fn get_adjacent(g: &[UiBlock], index: usize) -> Vec<usize> {
    let adj_ids = &g[index].links;
    adj_ids
        .iter()
        .map(|id| g.iter().position(|b| b.id == *id).unwrap())
        .collect()
}

/// Ok(nodes) | Err(There's a cycle)
fn find_connex(g: &[UiBlock], index: usize) -> Result<Vec<UiBlock>, ()> {
    fn go(g: &[UiBlock], seen: &mut [bool], i: usize, prev: usize) -> bool {
        // bool is cycle
        if seen[i] {
            return true;
        }
        seen[i] = true;
        for adj_idx in get_adjacent(g, i).into_iter().filter(|&x| x != prev) {
            if go(g, seen, adj_idx, i) {
                return true;
            }
        }
        false
    }

    let mut seen = vec![false; g.len()];
    if go(g, &mut seen, index, index) {
        Err(())
    } else {
        Ok(g.iter()
            .cloned()
            .enumerate()
            .filter_map(|(i, n)| seen[i].then_some(n))
            .collect())
    }
}
