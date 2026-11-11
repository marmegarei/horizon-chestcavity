//! Where each organ goes in a chest: one part of the cavity for each kind of organ, the places the human body has them
//! in. An organ put in goes to its own part; when that is full it takes a free slot among the neutral ones (the one the
//! human cavity leaves free and every slot past the third row), and when there are none it shares the part of another
//! organ, for it can go in any free slot.
use crate::cavity::{Cavity, SLOTS};

/// The parts of a chest: a kind of organ each, and the rest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Muscle,
    Rib,
    Appendix,
    Lung,
    Heart,
    Spleen,
    Kidney,
    Spine,
    Liver,
    Intestine,
    Stomach,
    /// Glands, tumors, cores, the vanilla pseudo-organs: no part of their own, they take the neutral slots.
    Other,
}

/// The slots of a part, in the order they are filled (the human cavity's own places). `Other` has the one the human
/// cavity leaves free; the neutral slots also are everything past the third row.
pub fn zone(part: Part) -> &'static [usize] {
    match part {
        Part::Muscle => &[0, 8, 9, 17, 18, 19, 25, 26],
        Part::Rib => &[1, 7, 10, 16],
        Part::Appendix => &[2],
        Part::Lung => &[3, 5],
        Part::Heart => &[4],
        Part::Spleen => &[11],
        Part::Kidney => &[12, 14],
        Part::Spine => &[13],
        Part::Liver => &[15],
        Part::Intestine => &[20, 21, 23, 24],
        Part::Stomach => &[22],
        Part::Other => &[6],
    }
}

/// The part an organ belongs to, by the end of its name (`chestcavity:dragon_heart` is a heart; `heart_of_gold` is not).
pub fn part_of(id: &str) -> Part {
    let name = id.rsplit(':').next().unwrap_or(id);
    const ENDS: [(&str, Part); 12] = [
        ("muscle", Part::Muscle),
        ("rib", Part::Rib),
        ("appendix", Part::Appendix),
        ("lung", Part::Lung),
        ("heart", Part::Heart),
        ("spleen", Part::Spleen),
        ("kidney", Part::Kidney),
        ("spine", Part::Spine),
        ("liver", Part::Liver),
        ("intestine", Part::Intestine),
        ("stomach", Part::Stomach),
        ("rumen", Part::Stomach),
    ];
    ENDS.iter().find(|(end, _)| name.ends_with(end)).map_or(Part::Other, |&(_, part)| part)
}

/// Whether slot `i` belongs to no kind of organ: the free one of the third row and everything past it.
fn neutral(i: usize) -> bool {
    zone(Part::Other).contains(&i) || i >= SLOTS
}

/// The slot an organ `id` goes in: the first free one of its part, else the first free neutral one, else any free one;
/// None when the cavity is full.
pub fn place(cavity: &Cavity, id: &str) -> Option<usize> {
    let free = |i: usize| cavity.slots.get(i).is_some_and(Option::is_none);
    let any = |pred: &dyn Fn(usize) -> bool| (0..cavity.slots.len()).find(|&i| pred(i) && free(i));
    zone(part_of(id)).iter().copied().find(|&i| free(i)).or_else(|| any(&neutral)).or_else(|| any(&|_| true))
}

/// Puts the organs that are out of their part back in it, whenever its part has a free slot: an organ of a kind into its
/// own zone, the others into the neutral slots. Nothing is swapped, so an organ that has no free place stays. Whether
/// anything moved.
pub fn organize(cavity: &mut Cavity) -> bool {
    let mut moved = false;
    // An organ only ever moves from a slot that is not its own to one that is, so this ends.
    loop {
        let mut pass = false;
        for i in 0..cavity.slots.len() {
            let Some(part) = cavity.slots[i].as_ref().map(|s| part_of(&s.id)) else { continue };
            let home = |j: usize| if part == Part::Other { neutral(j) } else { zone(part).contains(&j) };
            if home(i) {
                continue;
            }
            if let Some(j) = (0..cavity.slots.len()).find(|&j| home(j) && cavity.slots[j].is_none()) {
                cavity.slots[j] = cavity.slots[i].take();
                pass = true;
            }
        }
        if !pass {
            return moved;
        }
        moved = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cavity::{Slot, get, player_kind};

    fn at(cavity: &Cavity, i: usize) -> Option<&str> {
        cavity.slots[i].as_ref().map(|s| s.id.as_str())
    }

    #[test]
    fn the_human_bodys_own_organs_sit_in_their_own_parts() {
        for &(pos, id, _) in player_kind().slots {
            assert!(zone(part_of(id)).contains(&(pos as usize)), "{id} at {pos}");
        }
        assert_eq!(part_of("chestcavity:dragon_heart"), Part::Heart);
        assert_eq!(part_of("chestcavity:herbivore_rumen"), Part::Stomach);
        assert_eq!(part_of("chestcavity:heart_of_gold"), Part::Other);
        assert_eq!(part_of("minecraft:oak_door"), Part::Other);
        // No two parts share a slot, and the neutral one is nobody's.
        let mut seen = std::collections::HashSet::new();
        for part in [Part::Muscle, Part::Rib, Part::Appendix, Part::Lung, Part::Heart, Part::Spleen, Part::Kidney, Part::Spine, Part::Liver, Part::Intestine, Part::Stomach, Part::Other] {
            assert!(zone(part).iter().all(|&i| i < SLOTS && seen.insert(i)), "{part:?}");
        }
        assert_eq!(seen.len(), SLOTS, "every slot of the third row has a part");
    }

    #[test]
    fn an_organ_goes_to_its_part_then_to_a_neutral_slot_then_to_any_free_one() {
        let kind = player_kind();
        let mut cavity = Cavity::of(kind);
        cavity.slots[4] = None;
        assert_eq!(place(&cavity, "chestcavity:heart"), Some(4));
        assert_eq!(place(&cavity, "chestcavity:dragon_heart"), Some(4), "any heart goes to the heart's place");
        // A second heart has no place of its own: it takes the neutral slot.
        let mut full_heart = Cavity::of(kind);
        assert_eq!(place(&full_heart, "chestcavity:heart"), Some(6));
        // With that taken too, it takes whatever is free, even another part's.
        full_heart.slots[6] = Some(Slot::new("chestcavity:silk_gland", 1));
        full_heart.slots[1] = None;
        assert_eq!(place(&full_heart, "chestcavity:heart"), Some(1));
        // An organ with no part of its own goes to the neutral slot, then to the fourth row when the cavity has one.
        let mut grown = Cavity::of(kind).grown();
        assert_eq!(place(&grown, "chestcavity:silk_gland"), Some(6));
        grown.slots[6] = Some(Slot::new("chestcavity:silk_gland", 1));
        assert_eq!(place(&grown, "chestcavity:venom_gland"), Some(SLOTS));
        assert_eq!(place(&grown, "chestcavity:heart"), Some(SLOTS), "a heart too, with its place taken");
        // Nothing free, nothing to place.
        let mut full = Cavity::of(kind);
        full.slots[6] = Some(Slot::new("chestcavity:silk_gland", 1));
        assert_eq!(place(&full, "chestcavity:heart"), None);
    }

    #[test]
    fn organizing_puts_strays_home_without_swapping_and_changes_no_score() {
        let kind = player_kind();
        let whole = Cavity::of(kind);
        let mut cavity = whole.clone();
        // The heart sits in the neutral slot and its own place is empty: it goes home. A gland sits in the lung's place.
        cavity.slots[6] = cavity.slots[4].take();
        cavity.slots[3] = Some(Slot::new("chestcavity:silk_gland", 1));
        let scores = cavity.scores(kind);
        assert!(organize(&mut cavity));
        assert_eq!(at(&cavity, 4), Some("chestcavity:heart"));
        assert_eq!(at(&cavity, 6), Some("chestcavity:silk_gland"), "the gland took the neutral slot the heart left");
        assert!(cavity.slots[3].is_none(), "and left the lung's place free");
        assert_eq!(cavity.scores(kind), scores, "where an organ is changes nothing it does");
        assert!(!organize(&mut cavity), "a second time there is nothing to do");

        // With no free place for either, nothing is swapped.
        let mut stuck = whole.clone();
        stuck.slots[4] = Some(Slot::new("chestcavity:silk_gland", 1));
        stuck.slots[6] = Some(Slot::new("chestcavity:heart", 1));
        let before = stuck.clone();
        assert!(!organize(&mut stuck));
        assert_eq!(stuck, before);
        assert!(get(&stuck.scores(kind), "health") > 0.0);
    }
}
