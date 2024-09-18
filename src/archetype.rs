use std::collections::{HashMap, HashSet};

use crate::{
    component::{Component, ComponentId, Components}, storage::BlobVec, Entity
};

type Column = BlobVec;

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

#[derive(Debug, Default)]
pub struct Archetype {
    id: ArchetypeId,
    entities: Vec<EntityRecord>,
    components: HashMap<ComponentId, ArchetypeRecord>,
    data: Vec<Column>,
}

impl Archetype {
    pub fn new(id: ArchetypeId, components: Components) -> Self {
        let mut data = Vec::new();
        let mut comps = HashMap::new();

        for component_id in components.components() {
            comps.insert(
                component_id,
                ArchetypeRecord {
                    id,
                    column: data.len(),
                },
            );

            data.push(Column::new());
        }

        Self {
            id,
            entities: Vec::new(),
            components: comps,
            data,
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
