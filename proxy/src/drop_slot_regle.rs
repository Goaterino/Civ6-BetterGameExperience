//! Rule of `drop_slot`, shared by Windows (drop_slot.rs) and macOS (drop_slot_mac.rs): every
//! machine of a POL game must decide alike. See drop_slot.rs for the fix itself.

/// SLOT_STATUS values the fix reads and writes (the same on Windows and macOS).
pub const SLOT_COMPUTER: i32 = 1;
pub const SLOT_TAKEN: i32 = 3;

/// What to do with a connection change.
#[derive(Debug, PartialEq, Eq)]
pub enum Change {
    /// The game's handler, untouched.
    Game,
    /// The game's handler, with the slot kept human.
    KeepHuman,
    /// Nothing: the second message of a departure already handled.
    Skip,
}

pub fn decide(pol: bool, connecting: bool, kicked: bool, away: bool) -> Change {
    match (pol, connecting, kicked, away) {
        (false, ..) | (_, true, ..) | (_, _, true, _) => Change::Game,
        (true, false, false, false) => Change::KeepHuman,
        (true, false, false, true) => Change::Skip,
    }
}

pub fn bit(player: u32) -> u64 {
    if player < 64 {
        1 << player
    } else {
        0
    }
}

/// The status the slot gets: "taken" in place of "computer" during a kept departure.
pub fn kept(leaving: bool, status: i32) -> i32 {
    if leaving && status == SLOT_COMPUTER {
        SLOT_TAKEN
    } else {
        status
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_plain_departure_keeps_the_slot() {
        assert_eq!(decide(true, false, false, false), Change::KeepHuman, "a player drops");
        assert_eq!(decide(true, false, false, true), Change::Skip, "the second message of that departure");
        assert_eq!(decide(true, false, true, false), Change::Game, "a kick hands the player to the AI");
        assert_eq!(decide(true, false, true, true), Change::Game, "kicked while away");
        assert_eq!(decide(true, true, false, true), Change::Game, "the player is back");
        for away in [false, true] {
            for kicked in [false, true] {
                for connecting in [false, true] {
                    assert_eq!(decide(false, connecting, kicked, away), Change::Game, "not every player has BGE");
                }
            }
        }
    }

    #[test]
    fn only_computer_becomes_taken() {
        assert_eq!(kept(true, SLOT_COMPUTER), SLOT_TAKEN);
        assert_eq!(kept(false, SLOT_COMPUTER), SLOT_COMPUTER, "outside a kept departure");
        for status in [0, 2, 3, 4, 5] {
            assert_eq!(kept(true, status), status);
        }
        assert_eq!(bit(64), 0);
        assert_eq!(bit(63), 1 << 63);
    }
}
