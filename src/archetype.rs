use std::collections::{HashMap, HashSet};

use crate::{
    component::ComponentId,
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
    components: HashSet<ComponentId>,
}

impl Archetype {
    fn new(id: ArchetypeId, table: TableId, component_ids: &[ComponentId]) -> Self {
        let mut components = HashSet::new();

        for comp_id in component_ids {
            components.insert(*comp_id);
        }

        Self {
            id,
            entities: Vec::new(),
            components,
            table,
        }
    }

    fn contains(&self, id: ComponentId) -> bool {
        self.components.contains(&id)
    }

    pub(crate) fn allocate(&mut self, entity: Entity, table_row: TableRow) -> EntityLocation {
        debug_assert!(self.entities.len() == table_row.index());
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

    pub(crate) fn swap_remove(&mut self, row: TableRow) -> Option<Entity> {
        let is_last = self.entities.len() - 1 == row.index();
        let _ = self.entities.swap_remove(row.index());

        if !is_last {
            // Return the now moved entity
            Some(self.entities[row.index()].entity)
        } else {
            None
        }
    }

    fn is_superset_of(&self, sub: &HashSet<ComponentId>) -> bool {
        self.components.is_superset(sub)
    }
}

#[derive(Debug, Default)]
pub struct Archetypes {
    archetypes: Vec<Archetype>,
    archetype_index: HashMap<ArchetypeComponents, ArchetypeId>,
    component_index: HashMap<ComponentId, HashSet<ArchetypeId>>,
}

impl Archetypes {
    pub fn get_id_or_insert(&mut self, table_id: TableId, ids: &[ComponentId]) -> ArchetypeId {
        let identifier = ArchetypeComponents {
            components: ids.into(),
        };

        *self.archetype_index.entry(identifier).or_insert_with(|| {
            let id = ArchetypeId(self.archetypes.len());

            for comp_id in ids {
                self.component_index
                    .entry(*comp_id)
                    .or_insert_with(HashSet::new)
                    .insert(id);
            }

            self.archetypes
                .push(Archetype::new(id, table_id, ids.into()));
            id
        })
    }

    /// Receives the [`Archetype`] for the given [`ArchetypeId`].
    ///
    /// Panics: If the archetype does not exist in this world.
    pub(crate) fn get_unchecked(&self, archetype_id: ArchetypeId) -> &Archetype {
        &self.archetypes[archetype_id.index()]
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

    pub(crate) fn get_query_archetypes(
        &self,
        components: &[ComponentId],
    ) -> (Vec<ArchetypeId>, Vec<TableId>) {
        let initial = if let Some(initial) = self.component_index.get(&components[0]) {
            initial
        } else {
            return (vec![], vec![]);
        };

        let mut comps = HashSet::new();
        for comp in components {
            comps.insert(*comp);
        }

        let mut archetype_ids = initial
            .iter()
            .filter(|id| self.archetypes[id.index()].is_superset_of(&comps))
            .copied()
            .collect::<Vec<_>>();
        let mut table_ids = archetype_ids
            .iter()
            .map(|id| self.archetypes[id.index()].table)
            .collect::<Vec<_>>();
        archetype_ids.sort_unstable();
        table_ids.sort_unstable();
        (archetype_ids, table_ids)
    }
}
