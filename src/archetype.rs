use std::collections::{HashMap, HashSet};

use crate::{
    component::{Component, ComponentId, Components}, storage::{Table, TableId}, Entity
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct EntityRecord {
    entity: Entity,
    row: usize,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) struct ArchetypeComponents {
    components: Box<[ComponentId]>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct ArchetypeId(pub(crate) usize);

#[derive(Debug)]
pub struct Archetype {
    id: ArchetypeId,
    entities: Vec<EntityRecord>,
    components: HashMap<ComponentId, ArchetypeRecord>,
    table: TableId,
}

impl Archetype {
    pub fn new(id: ArchetypeId, table: TableId, components: Components) -> Self {
        let mut comps = HashMap::new();

        for component_id in components.components() {
            comps.insert(
                component_id,
                ArchetypeRecord {
                    id,
                    column: comps.len(),
                },
            );
        }

        Self {
            id,
            entities: Vec::new(),
            components: comps,
            table,
        }
    }

    fn insert<T: Component>(&mut self, entity: Entity, components: Vec<T>) {
        let record = EntityRecord {
            entity,
            row: self.entities.len(),
        };
    }

    pub fn contains(&self, id: ComponentId) -> bool {
        self.components.contains_key(&id)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct ArchetypeRecord {
    pub(crate) id: ArchetypeId,
    column: usize,
}
