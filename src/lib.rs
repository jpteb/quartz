#![feature(alloc_layout_extra)]
#![allow(unused)]
use std::collections::{HashMap, HashSet};

pub mod archetype;
pub mod component;
pub mod entity;
pub mod ptr;
pub mod storage;

use archetype::{ArchetypeComponents, ArchetypeId, Archetypes};
use component::{Bundle, ComponentId, Components};
use entity::{Entities, Entity};
use storage::{TableId, TableRow, Tables};

#[derive(Debug)]
pub struct World {
    entities: Entities,
    archetypes: Archetypes,
    components: Components,
    tables: Tables,
}

impl World {
    pub fn spawn<B: Bundle>(&mut self, bundle: B) -> Entity {
        self.entities
            .alloc(|entity| {
                let mut component_ids = Vec::new();
                bundle.component_ids(&mut self.components, &mut |id| {
                    component_ids.push(id);
                });
                component_ids.sort_unstable();

                let table_id = self
                    .tables
                    .get_id_or_insert(&component_ids, &self.components);

                let mut table = self.tables.get(table_id);

                Ok(entity::EntityLocation {
                    archetype_id: ArchetypeId(0),
                    table_id,
                    table_row: TableRow(0),
                })
            })
            .unwrap()
    }

    fn has_component(&self, entity: Entity, component: ComponentId) -> Option<bool> {
        let archetype_id = {
            if let Some(loc) = self.entities.get(entity) {
                loc.archetype_id
            } else {
                return None;
            }
        };

        self.archetypes.has_component(archetype_id, component)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_has_component() {
    //     let mut world = World::default();
    //     let comps = vec![ComponentId::new(0), ComponentId::new(1)];
    //
    //     let entity = world.spawn(comps);
    //
    //     assert!(world.has_component(entity, ComponentId::new(0)));
    //     assert!(world.has_component(entity, ComponentId::new(1)));
    //     assert!(!world.has_component(entity, ComponentId::new(2)));
    // }
    //
    // #[test]
    // fn test_get_archetype() {
    //     let mut world = World::default();
    //     let comps1 = vec![ComponentId::new(0), ComponentId::new(1)];
    //     let entity1 = world.spawn(comps1.clone());
    //     let comps2 = vec![ComponentId::new(0), ComponentId::new(2)];
    //     let entity2 = world.spawn(comps2);
    //     let entity3 = world.spawn(comps1);
    //
    //     assert_eq!(
    //         world.get_archetypes_by_comp(ComponentId::new(0)),
    //         vec![ArchetypeId(0), ArchetypeId(1)]
    //     );
    //     assert_eq!(
    //         world.get_archetypes_by_comp(ComponentId::new(1)),
    //         vec![ArchetypeId(0)]
    //     );
    //     assert_eq!(
    //         world.get_archetypes_by_comp(ComponentId::new(2)),
    //         vec![ArchetypeId(1)]
    //     );
    // }
}
