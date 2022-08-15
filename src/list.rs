/// The list and events for handling movement within the list. No UI.
use std::collections::VecDeque;

/// # List
///
/// `List` represents a list of items and a selection of one from that list.
/// To encode the selection, `List` maintains items in
/// a queue `above` of items above the selection,
/// a selected item which can be `Option::None` in case the list is empty, and
/// a queue `below` of items below the selection.
///
/// For instance `above = [1, 2]`, `selected = Some(3)`, `below = [4, 5, 6]` results in the list
/// - 1
/// - 2
/// - (3)
/// - 4
/// - 5
/// - 6
///
/// Maintain the INVARIANT that `selected` can only be empty if there are no elements in the list.
pub struct List<T>
where
    T: Clone,
{
    capacity: usize,
    above: VecDeque<T>,
    selected: Option<T>,
    below: VecDeque<T>,
}

impl<T> List<T>
where
    T: Clone,
{
    pub fn new(capacity: usize) -> Self {
        List {
            capacity,
            above: VecDeque::new(),
            selected: None,
            below: VecDeque::new(),
        }
    }

    /// Items in order from top to bottom
    pub fn items<'a>(&'a self) -> Box<dyn Iterator<Item = T> + 'a> {
        Box::new(
            self.above
                .iter()
                .chain(self.selected.iter())
                .chain(self.below.iter())
                .cloned(),
        )
    }

    pub fn tagged_iter<'a>(&'a self) -> Box<dyn Iterator<Item = (bool, T)> + 'a> {
        let selected_index = self.len_above();
        Box::new(
            self.items()
                .enumerate()
                .map(move |(index, item)| (index == selected_index, item)),
        )
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn len(&self) -> usize {
        self.above.len() + self.selected.iter().count() + self.below.len()
    }

    pub fn len_above(&self) -> usize {
        self.above.len()
    }

    pub fn is_empty(&self) -> bool {
        // NOTE that because of the invariant the line below is equivalent to
        //  `self.above.is_empty() && self.selected.is_none() && self.below.is_empty()`
        self.selected.is_none()
    }

    pub fn up(&mut self) {
        if let Some(item_above) = self.above.pop_back() {
            if let Some(selected) = self.selected.take() {
                self.below.push_front(selected);
                self.selected = Some(item_above);
            } else {
                unreachable!("the invariant has been violated")
            }
        }
    }

    pub fn down(&mut self) {
        if let Some(item_below) = self.below.pop_front() {
            if let Some(selected) = self.selected.take() {
                self.above.push_back(selected);
                self.selected = Some(item_below);
            } else {
                unreachable!("the invariant has been violated")
            }
        }
    }

    /// Takes the current matches and updates the visible contents.
    ///
    /// The input matches are assumed to be sorted in descending order of score.
    pub fn update(&mut self, matches: &[T]) {
        log::info!("Updating view with {} match(es)", matches.len());
        let is_empty = self.is_empty();
        let selected_len = self.selected.iter().count();
        let below_len = self.below.len();
        let above_len = self.capacity - selected_len - below_len;
        assert_eq!(above_len + selected_len + below_len, self.capacity);

        self.above.clear();
        self.below.clear();
        self.selected = None;

        // take the highest scoring items
        let iter = matches.iter().take(self.capacity as usize).cloned();

        if is_empty {
            // extend above so the bottom item gets selected if the List was initially empty
            self.above.extend(iter.rev());
        } else {
            // otherwise fill up from below
            self.below.extend(iter.clone().take(below_len).rev());
            self.selected = iter.clone().nth(below_len);
            self.above.extend(iter.skip(below_len + 1).rev());
        }

        // ensure invariant
        if self.selected.is_none() {
            // select the top-most item by default
            self.selected = self.below.pop_front().or_else(|| self.above.pop_back());
        }
    }

    pub fn get_selected(&self) -> &T {
        self.selected.as_ref().unwrap()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use super::super::item::Item;

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

    struct Setup {
        items: Vec<Item<TestItem>>,
        few_items: Vec<Item<TestItem>>,
        view: List<Item<TestItem>>,
    }

    impl Setup {
        fn new(lines_to_show: i8) -> Self {
            let view = List::<Item<TestItem>>::new(lines_to_show as usize);

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
        assert_eq!(setup.view.len(), 8);
        assert_eq!(setup.view.len_above(), 7); // 0-indexed
        assert_eq!(setup.view.get_selected().item.as_ref().unwrap().name, "A")
    }

    #[test]
    fn test_up() {
        // GIVEN
        let mut setup = Setup::new(8);
        setup.view.update(&setup.items);

        // WHEN
        setup.view.up(); // 6
        setup.view.up(); // 5
        setup.view.up(); // 4

        // THEN
        assert_eq!(setup.view.len(), 8);
        assert_eq!(setup.view.len_above(), 4);
    }

    #[test]
    fn test_up_to_extremis() {
        // GIVEN
        let mut setup = Setup::new(8);
        setup.view.update(&setup.items);
        assert!(setup.items.len() > 0);
        assert_eq!(
            setup.view.len(),
            setup.view.capacity().min(setup.items.len())
        );

        // WHEN
        // More than lines_to_show
        setup.view.up();
        setup.view.up();
        setup.view.up();
        setup.view.up();
        setup.view.up();
        setup.view.up();
        setup.view.up();
        setup.view.up();
        setup.view.up();
        setup.view.up();
        setup.view.up();
        setup.view.up();
        setup.view.up();

        // THEN
        assert_eq!(setup.view.len(), 8);
        assert_eq!(setup.view.len_above(), 0);
    }

    #[test]
    fn test_down_at_bottom() {
        // GIVEN
        let mut setup = Setup::new(8);
        setup.view.update(&setup.items);

        // WHEN
        setup.view.down(); // 7

        // THEN
        assert_eq!(setup.view.len(), 8);
        assert_eq!(setup.view.len_above(), 7);
    }

    #[test]
    fn test_down() {
        // GIVEN
        let mut setup = Setup::new(8);
        setup.view.update(&setup.items);

        // WHEN
        setup.view.up(); // 6
        setup.view.up(); // 5
        setup.view.up(); // 4
        setup.view.down(); // 5

        // THEN
        assert_eq!(setup.view.len(), 8);
        assert_eq!(setup.view.len_above(), 5);
    }

    #[test]
    fn test_few() {
        // GIVEN
        let mut setup = Setup::new(8);

        // WHEN
        setup.view.update(&setup.few_items);
        setup.view.up(); // 6
        setup.view.up(); // 5
        setup.view.up(); // 5
        setup.view.up(); // 5

        // THEN
        assert_eq!(setup.view.len(), 3);
        assert_eq!(setup.view.len_above(), 0);
    }
}
