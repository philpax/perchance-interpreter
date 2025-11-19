use crate::compiler::{CompiledItem, CompiledList};

#[derive(Debug, Clone)]
pub(super) struct ConsumableListState {
    pub source_list: CompiledList,
    pub remaining_indices: Vec<usize>,
}

#[derive(Debug, Clone)]
pub(super) enum Value {
    Text(String),
    List(String),               // Reference to a list by name
    ListInstance(CompiledList), // An actual list instance (for sublists)
    ItemInstance(CompiledItem), // An item with its properties (sublists) intact
    Array(Vec<String>),         // Multiple items (for selectMany/selectUnique before joinItems)
    ConsumableList(String),     // Reference to a consumable list by unique ID
    ImportedGenerator(String),  // Reference to an imported generator by name
}
