/// The list and events for handling movement within the list. No UI.
use super::item::Item;

pub struct List<T>
where
    T: Clone,
{
    /// Index into the matches of the item shown at the top of the view.
    pub top_index: usize,
    /// Index into the matches of the item shown at the bottom of the view.
    pub bottom_index: usize,
    /// The height of the view.
    pub lines_to_show: usize,
    /// The view-relative index of the selected item.
    pub selected_index: usize,
    /// The items currently visible in the view, padded with blanks.
    pub items: Vec<Item<T>>,
}

impl<T> List<T>
where
    T: Clone,
{
    pub fn new(lines_to_show: usize) -> Self {
        // A zero-height list can't render anything, so enforce a minimum.
        let lines_to_show = lines_to_show.max(1);
        List {
            items: vec![],
            top_index: lines_to_show - 1,
            selected_index: lines_to_show - 1,
            lines_to_show,
            bottom_index: 0,
        }
    }

    pub fn up(&mut self, matches: &[Item<T>]) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        } else if self.top_index + 1 < matches.len() {
            self.bottom_index += 1;
            self.top_index += 1;
        }
        self.floor_selected_index();
    }

    pub fn down(&mut self) {
        // Should we move the selection down?
        if self.selected_index < self.top_index {
            self.selected_index += 1;
        }

        // Should we scroll down?
        if self.selected_index > self.lines_to_show - 1 && self.bottom_index > 0 {
            self.bottom_index -= 1;
            self.top_index -= 1;
            // if we've scrolled down then we don't want to change the selected index
            // The selected index is for the view, so it stays the same.
            if self.selected_index > 0 {
                self.selected_index -= 1;
            }
        }
        self.floor_selected_index();
    }

    /// Moves the selection to the bottom of the list.
    pub fn reset_selection(&mut self) {
        self.top_index = self.lines_to_show - 1;
        self.bottom_index = 0;
        self.selected_index = self.lines_to_show - 1;
    }

    fn floor_selected_index(&mut self) {
        let index_of_first_blank = self.items.iter().rev().position(|item| item.is_blank);
        if let Some(rev_index) = index_of_first_blank {
            let index = self.lines_to_show - rev_index;

            // This first condition handles the scenario where the user is trying to go up on a short list
            if self.selected_index < index && index < self.lines_to_show {
                self.selected_index = index;
            }
            // This second condition handles the scenario where the list was empty, but the user deleted
            // some text and now we need to create a selection again.
            else if self.selected_index < index {
                self.selected_index = index - 1;
            }
        }
    }

    /// Takes the current matches and updates the visible contents.
    pub fn update(&mut self, matches: &[Item<T>]) {
        log::info!("Updating view with {} match(es)", matches.len());
        let mut to_render: Vec<Item<T>> = Vec::new();
        // Get everything in our display window
        for i in self.bottom_index..self.top_index + 1 {
            if matches.len() > i {
                to_render.push(matches[i].clone());
            } else {
                to_render.push(Item::empty());
            }
        }
        to_render.reverse();

        self.items = to_render;
        self.floor_selected_index();
    }

    pub fn get_selected(&self) -> Option<&Item<T>> {
        self.items.get(self.selected_index)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[derive(Clone)]
    struct TestItem {
        name: String,
    }

    fn item(name: &str) -> Item<TestItem> {
        Item::new(
            String::from(name),
            TestItem {
                name: String::from(name),
            },
        )
    }

    fn selected_name(view: &List<TestItem>) -> String {
        view.get_selected()
            .and_then(|selected| selected.item.as_ref())
            .map(|item| item.name.clone())
            .expect("expected a selected item")
    }

    struct Setup {
        items: Vec<Item<TestItem>>,
        few_items: Vec<Item<TestItem>>,
        view: List<TestItem>,
    }

    impl Setup {
        fn new(lines_to_show: usize) -> Self {
            let view = List::<TestItem>::new(lines_to_show);

            Setup {
                items: vec![
                    item("A"),
                    item("B"),
                    item("C"),
                    item("D"),
                    item("E"),
                    item("F"),
                    item("G"),
                    item("H"),
                    item("I"),
                    item("J"),
                    item("K"),
                    item("L"),
                    item("M"),
                ],
                few_items: vec![item("A"), item("B"), item("C")],
                view,
            }
        }
    }

    #[test]
    fn test_update() {
        // GIVEN
        let mut setup = Setup::new(8);

        // WHEN
        setup.view.update(&setup.items);

        // THEN
        assert_eq!(setup.view.items.len(), 8);
        assert_eq!(setup.view.selected_index, 7); // 0-indexed
        assert_eq!(selected_name(&setup.view), "A");
    }

    #[test]
    fn test_up() {
        // GIVEN
        let mut setup = Setup::new(8);
        setup.view.update(&setup.items);

        // WHEN
        setup.view.up(&setup.items); // 6
        setup.view.up(&setup.items); // 5
        setup.view.up(&setup.items); // 4

        // THEN
        assert_eq!(setup.view.items.len(), 8);
        assert_eq!(setup.view.selected_index, 4);
        assert_eq!(selected_name(&setup.view), "D");
    }

    #[test]
    fn test_up_to_extremis() {
        // GIVEN
        let mut setup = Setup::new(8);
        setup.view.update(&setup.items);

        // WHEN
        // More than lines_to_show
        for _ in 0..13 {
            setup.view.up(&setup.items);
        }

        // THEN
        assert_eq!(setup.view.items.len(), 8);
        assert_eq!(setup.view.selected_index, 0);
    }

    #[test]
    fn test_up_scrolls_beyond_the_view() {
        // GIVEN
        let mut setup = Setup::new(8);
        setup.view.update(&setup.items);

        // WHEN
        // 7 to get to the top of the view, 2 more to scroll
        for _ in 0..9 {
            setup.view.up(&setup.items);
        }
        setup.view.update(&setup.items);

        // THEN
        assert_eq!(setup.view.selected_index, 0);
        assert_eq!(setup.view.top_index, 9);
        assert_eq!(setup.view.bottom_index, 2);
        assert_eq!(selected_name(&setup.view), "J");
    }

    #[test]
    fn test_down_at_bottom() {
        // GIVEN
        let mut setup = Setup::new(8);
        setup.view.update(&setup.items);

        // WHEN
        setup.view.down(); // 7

        // THEN
        assert_eq!(setup.view.items.len(), 8);
        assert_eq!(setup.view.selected_index, 7);
    }

    #[test]
    fn test_down() {
        // GIVEN
        let mut setup = Setup::new(8);
        setup.view.update(&setup.items);

        // WHEN
        setup.view.up(&setup.items); // 6
        setup.view.up(&setup.items); // 5
        setup.view.up(&setup.items); // 4
        setup.view.down(); // 5

        // THEN
        assert_eq!(setup.view.items.len(), 8);
        assert_eq!(setup.view.selected_index, 5);
    }

    #[test]
    fn test_few() {
        // GIVEN
        let mut setup = Setup::new(8);

        // WHEN
        setup.view.update(&setup.few_items);
        setup.view.up(&setup.few_items); // 6
        setup.view.up(&setup.few_items); // 5
        setup.view.up(&setup.few_items); // 5
        setup.view.up(&setup.few_items); // 5

        // THEN
        assert_eq!(setup.view.items.len(), 8); // Still 8, but blanks
        assert_eq!(setup.view.selected_index, 5);
        assert_eq!(selected_name(&setup.view), "C");
    }

    #[test]
    fn test_up_with_no_matches_does_not_scroll() {
        // GIVEN
        let mut setup = Setup::new(8);
        let no_items: Vec<Item<TestItem>> = vec![];
        setup.view.update(&no_items);

        // WHEN
        // In 0.4 this overflowed the u8 match count and scrolled into nothing.
        setup.view.up(&no_items);
        setup.view.up(&no_items);

        // THEN
        assert_eq!(setup.view.top_index, 7);
        assert_eq!(setup.view.bottom_index, 0);
        assert!(setup.view.get_selected().is_some_and(|item| item.is_blank));
    }

    #[test]
    fn test_reset_selection() {
        // GIVEN
        let mut setup = Setup::new(8);
        setup.view.update(&setup.items);
        for _ in 0..9 {
            setup.view.up(&setup.items);
        }

        // WHEN
        setup.view.reset_selection();
        setup.view.update(&setup.items);

        // THEN
        assert_eq!(setup.view.selected_index, 7);
        assert_eq!(setup.view.top_index, 7);
        assert_eq!(setup.view.bottom_index, 0);
        assert_eq!(selected_name(&setup.view), "A");
    }

    #[test]
    fn test_minimum_of_one_line() {
        // GIVEN a zero-height request
        let mut view = List::<TestItem>::new(0);

        // WHEN
        view.update(&[item("A")]);

        // THEN it behaves as a one-line list rather than panicking
        assert_eq!(view.items.len(), 1);
        assert_eq!(view.selected_index, 0);
    }
}
