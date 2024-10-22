#![feature(alloc_layout_extra)]
#![allow(unused)]
use std::collections::{HashMap, HashSet};

mod archetype;
mod component;
mod entity;
mod storage;

use archetype::{Archetype, ArchetypeComponents, ArchetypeId, ArchetypeRecord};
use component::{ComponentId, Components};
use entity::Entity;

#[derive(Debug, Default)]
pub struct World {
    entities: Vec<Entity>,
    archetypes: Vec<Archetype>,
    entity_index: HashMap<Entity, ArchetypeId>,
    archetype_index: HashMap<ArchetypeComponents, ArchetypeId>,
    component_index: HashMap<ComponentId, HashSet<ArchetypeId>>,
    archetype_count: ArchetypeId,
}

impl World {
    fn has_component(&self, entity: Entity, component: ComponentId) -> bool {
        let archetype_id = self.entity_index.get(&entity).unwrap();
        self.component_index
            .get(&component)
            .is_some_and(|a| a.contains(archetype_id))
    }

    fn get_archetypes_by_comp(&self, component: ComponentId) -> Vec<ArchetypeId> {
        let mut result = self
            .component_index
            .get(&component)
            .map_or(vec![], |s| s.iter().copied().collect::<Vec<_>>());
        result.sort();
        result
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
