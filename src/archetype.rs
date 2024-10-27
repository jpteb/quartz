use std::collections::{HashMap, HashSet};

use crate::{
    component::{Component, ComponentId, Components},
    storage::{Table, TableId, TableRow},
    Entity,
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct EntityRecord {
    entity: Entity,
    row: TableRow,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) struct ArchetypeComponents {
    components: Box<[ComponentId]>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArchetypeId(pub(crate) usize);

#[derive(Debug)]
pub struct Archetype {
    id: ArchetypeId,
    table: TableId,
    entities: Vec<EntityRecord>,
    components: HashMap<ComponentId, usize>,
}

impl Archetype {
    fn new(
        id: ArchetypeId,
        components: &Components,
        table: TableId,
        mut component_ids: Vec<ComponentId>,
    ) -> Self {
        let mut comps = HashMap::new();
        component_ids.sort();

        for component_id in components.components() {
            comps.insert(component_id, comps.len());
        }

        Self {
            id,
            entities: Vec::new(),
            components: comps,
            table,
        }
    }

    fn contains(&self, id: ComponentId) -> bool {
        self.components.contains_key(&id)
    }
}

#[derive(Debug, Default)]
pub struct Archetypes {
    archetypes: Vec<Archetype>,
    archetype_index: HashMap<ArchetypeComponents, ArchetypeId>,
    component_index: HashMap<ComponentId, HashSet<ArchetypeId>>,
}

impl Archetypes {
    pub(crate) fn has_component(
        &self,
        archetype_id: ArchetypeId,
        component: ComponentId,
    ) -> Option<bool> {
        self.component_index
            .get(&component)
            .and_then(|s| Some(s.contains(&archetype_id)))
    }
}
