use crate::{
    archetype::ArchetypeId,
    storage::{TableId, TableRow},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity {
    index: u32,
    generation: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct EntityLocation {
    archetype_id: ArchetypeId,
    table_id: TableId,
    table_row: TableRow,
}
