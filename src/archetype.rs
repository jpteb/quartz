use std::collections::{HashMap, HashSet};

use crate::{
    component::{ComponentId, Components},
    entity::EntityLocation,
    storage::{TableId, TableRow},
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

impl ArchetypeId {
    pub(crate) fn index(&self) -> usize {
        self.0
    }
}

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

    pub(crate) fn allocate(&mut self, entity: Entity, table_row: TableRow) -> EntityLocation {
        self.entities.push(EntityRecord {
            entity,
            row: table_row,
        });

        EntityLocation {
            archetype_id: self.id,
            table_id: self.table,
            table_row,
        }
    }
}

#[derive(Debug, Default)]
pub struct Archetypes {
    archetypes: Vec<Archetype>,
    archetype_index: HashMap<ArchetypeComponents, ArchetypeId>,
    component_index: HashMap<ComponentId, HashSet<ArchetypeId>>,
}

impl Archetypes {
    pub fn get_id_or_insert(
        &mut self,
        components: &Components,
        table_id: TableId,
        ids: &[ComponentId],
    ) -> ArchetypeId {
        let identifier = ArchetypeComponents {
            components: ids.into(),
        };

        *self
            .archetype_index
            .entry(identifier)
            .or_insert_with(|| {
                let id = ArchetypeId(self.archetypes.len());

                for comp_id in ids {
                    self.component_index
                        .entry(*comp_id)
                        .or_insert_with(HashSet::new)
                        .insert(id);
                }

                self.archetypes
                    .push(Archetype::new(id, components, table_id, ids.into()));
                id
            })
    }

    pub(crate) fn get_mut(&mut self, archetype_id: ArchetypeId) -> Option<&mut Archetype> {
        self.archetypes.get_mut(archetype_id.index())
    }

    /// Receives the [`Archetype`] for the given [`ArchetypeId`].
    ///
    /// Panics: If the archetype does not exist in this world.
    pub(crate) fn get_mut_unchecked(&mut self, archetype_id: ArchetypeId) -> &mut Archetype {
        &mut self.archetypes[archetype_id.index()]
    }

    pub fn len(&self) -> usize {
        self.archetypes.len()
    }
}
