//! Rule of `absent_reactivation`, shared by Windows (absent_reactivation.rs) and macOS
//! (absent_reactivation_mac.rs): every machine of a POL game must decide alike. See
//! absent_reactivation.rs for the marker and the fix.

/// Whether a `PlayerTurnComplete` for `player`, received from `sender` (`None`: not received through
/// GameCore's receive function), keeps the player's turn ended.
pub fn keeps_ended(sender: Option<i32>, player: i32, pol: bool, network: bool) -> bool {
    let valid = |p: i32| (0..64).contains(&p);
    pol && network && valid(player) && sender.is_some_and(|s| valid(s) && s != player)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_another_players_message_in_a_pol_network_game() {
        assert!(keeps_ended(Some(1), 2, true, true), "the host ends an absent player's turn");
        assert!(!keeps_ended(Some(2), 2, true, true), "the player's own end of turn");
        assert!(!keeps_ended(None, 2, true, true), "not received through GameCore's receive function");
        assert!(!keeps_ended(Some(1), 2, false, true), "not every player has BGE");
        assert!(!keeps_ended(Some(1), 2, true, false), "local game (solo, hotseat) or PlayByCloud");
        assert!(!keeps_ended(Some(-1), 2, true, true) && !keeps_ended(Some(64), 2, true, true), "no such sender");
        assert!(!keeps_ended(Some(1), -1, true, true) && !keeps_ended(Some(1), 64, true, true), "no such player");
    }
}
