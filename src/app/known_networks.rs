//! Network › Known networks: the saved names as a list. Select asks to
//! forget the selected one and a second Select confirms, so a stray press
//! deletes nothing. The main loop takes the request and replaces
//! `AppState::saved_networks`.

use crate::buttons::ButtonEvent;

/// What the Known networks screen asks of the main loop.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetworkRequest {
    /// Remove this saved network.
    Forget(String),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct KnownNetworksUi {
    selected: usize,
    /// The name a Select asked to forget, waiting for the second Select.
    confirming: Option<String>,
    request: Option<NetworkRequest>,
}

impl KnownNetworksUi {
    /// The selected row, kept inside `count` names (the list can shrink
    /// after a forget).
    #[must_use]
    pub fn selected(&self, count: usize) -> usize {
        self.selected.min(count.saturating_sub(1))
    }

    /// The name waiting for confirmation, if any.
    #[must_use]
    pub fn confirming(&self) -> Option<&str> {
        self.confirming.as_deref()
    }

    /// ▲▼ move the selection (around the ends); ● asks to forget the
    /// selected name, and ● again forgets it. ▲▼ while asking keep it.
    pub fn apply(&mut self, event: ButtonEvent, names: &[String]) {
        if names.is_empty() {
            self.selected = 0;
            self.confirming = None;
            return;
        }
        self.selected = self.selected(names.len());
        if let Some(name) = self.confirming.take() {
            if event == ButtonEvent::Select && names.contains(&name) {
                self.request = Some(NetworkRequest::Forget(name));
            }
            return;
        }
        match event {
            ButtonEvent::Up => {
                self.selected = self.selected.checked_sub(1).unwrap_or(names.len() - 1);
            }
            ButtonEvent::Down => self.selected = (self.selected + 1) % names.len(),
            ButtonEvent::Select => self.confirming = Some(names[self.selected].clone()),
        }
    }

    /// Hold BOOT: drops a pending question. True when there was one, so the
    /// screen stays.
    pub fn cancel(&mut self) -> bool {
        self.confirming.take().is_some()
    }

    /// The request for the main loop, once.
    pub fn take_request(&mut self) -> Option<NetworkRequest> {
        self.request.take()
    }
}

#[cfg(test)]
mod tests {
    use super::{KnownNetworksUi, NetworkRequest};
    use crate::buttons::ButtonEvent::{Down, Select, Up};

    fn names(count: usize) -> Vec<String> {
        (1..=count).map(|n| format!("net{n}")).collect()
    }

    #[test]
    fn up_and_down_move_around_the_ends() {
        let list = names(3);
        let mut ui = KnownNetworksUi::default();
        ui.apply(Up, &list);
        assert_eq!(ui.selected(3), 2, "up from the first goes to the last");
        ui.apply(Down, &list);
        assert_eq!(ui.selected(3), 0, "down from the last goes to the first");
        ui.apply(Down, &list);
        assert_eq!(ui.selected(3), 1);
        assert_eq!(ui.take_request(), None, "moving asks nothing");
    }

    #[test]
    fn select_asks_and_a_second_select_forgets_the_selected_name() {
        let list = names(3);
        let mut ui = KnownNetworksUi::default();
        ui.apply(Down, &list);
        ui.apply(Select, &list);
        assert_eq!(ui.confirming(), Some("net2"));
        assert_eq!(ui.take_request(), None, "one press deletes nothing");
        ui.apply(Select, &list);
        assert_eq!(ui.confirming(), None);
        assert_eq!(
            ui.take_request(),
            Some(NetworkRequest::Forget("net2".into()))
        );
        assert_eq!(ui.take_request(), None, "taken once");
    }

    #[test]
    fn a_move_or_hold_boot_keeps_the_network() {
        let list = names(3);
        let mut ui = KnownNetworksUi::default();
        ui.apply(Select, &list);
        ui.apply(Down, &list);
        assert_eq!(ui.confirming(), None, "a move cancels the question");
        assert_eq!(ui.selected(3), 0, "and does not move");
        assert_eq!(ui.take_request(), None);

        ui.apply(Select, &list);
        assert!(ui.cancel(), "hold BOOT cancels the question");
        assert!(!ui.cancel(), "nothing left to cancel: hold BOOT leaves");
        ui.apply(Select, &list);
        assert_eq!(ui.take_request(), None, "asking again starts over");
        assert_eq!(ui.confirming(), Some("net1"));
    }

    #[test]
    fn the_selection_stays_inside_the_list_after_a_name_goes() {
        let mut ui = KnownNetworksUi::default();
        let six = names(6);
        for _ in 0..5 {
            ui.apply(Down, &six);
        }
        ui.apply(Select, &six);
        ui.apply(Select, &six);
        assert_eq!(
            ui.take_request(),
            Some(NetworkRequest::Forget("net6".into()))
        );
        // The main loop removes the last name: the selection moves to the
        // new last row.
        let five = names(5);
        assert_eq!(ui.selected(five.len()), 4);
        ui.apply(Up, &five);
        assert_eq!(ui.selected(five.len()), 3);

        // A name in the middle goes: the same row now holds the next name.
        ui.apply(Select, &five);
        ui.apply(Select, &five);
        assert_eq!(
            ui.take_request(),
            Some(NetworkRequest::Forget("net4".into()))
        );
        let four: Vec<String> = ["net1", "net2", "net3", "net5"].map(String::from).to_vec();
        assert_eq!(ui.selected(four.len()), 3);

        // The last one goes: nothing to select, ask or forget.
        assert_eq!(ui.selected(0), 0);
        ui.apply(Select, &[]);
        assert_eq!(ui.confirming(), None);
        assert_eq!(ui.take_request(), None);
    }

    #[test]
    fn a_name_gone_while_asking_is_not_requested() {
        let mut ui = KnownNetworksUi::default();
        ui.apply(Select, &names(2));
        ui.apply(Select, &names(2)[1..]);
        assert_eq!(ui.take_request(), None);
    }
}
