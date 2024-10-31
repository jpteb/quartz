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
    pub fn new() -> Self {
        Self {
            entities: Entities::default(),
            archetypes: Archetypes::default(),
            components: Components::new(),
            tables: Tables::default(),
        }
    }

    pub fn spawn<B: Bundle>(&mut self, bundle: B) -> Entity {
        self.entities
            .alloc(|entity| {
                let mut component_ids = Vec::new();
                B::component_ids(&mut self.components, &mut |id| {
                    component_ids.push(id);
                });
                component_ids.sort_unstable();

                let table_id = self
                    .tables
                    .get_id_or_insert(&component_ids, &self.components);

                let archetype_id =
                    self.archetypes
                        .get_id_or_insert(&self.components, table_id, &component_ids);

                let table_row = if let Some(table) = self.tables.get_mut(table_id) {
                    let row = table.allocate(entity);
                    bundle.get(&mut self.components, &mut |id, ptr| unsafe {
                        table
                            .get_column_mut(id)
                            .expect("the selected table must have the correct column for this component")
                            .initialize_unchecked(row.0, ptr);
                    });
                    row
                } else {
                    return Err(());
                };

                Ok(entity::EntityLocation {
                    archetype_id,
                    table_id,
                    table_row,
                })
            })
            .expect("entity allocation should not fail")
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
    use component::Component;

    use super::*;

    struct MyComponent(u32);
    impl Component for MyComponent {}

    #[test]
    fn spawn() {
        let mut world = World::new();
        let entity = world.spawn(MyComponent(1));

        assert_eq!(entity, Entity::from(0, 0));

        let comp_id = world.components.component_id::<MyComponent>().unwrap();
        assert_eq!(world.has_component(entity, comp_id), Some(true));
    }

    #[test]
    fn spawn_multiple() {
        let mut world = World::new();

        let e0 = world.spawn(MyComponent(0));
        let e1 = world.spawn(MyComponent(1));

        assert_eq!(e0, Entity::from(0, 0));
        assert_eq!(e1, Entity::from(0, 1));

        assert_eq!(world.archetypes.len(), 1);
        assert_eq!(world.components.len(), 1);
        assert_eq!(world.entities.len(), 2);
        assert_eq!(world.tables.len(), 1);
    }

    #[test]
    fn spawn_batch() {
        const BATCH_SIZE: u32 = 1000000;
        let mut world = World::new();

        for i in 0..BATCH_SIZE {
            let entity = world.spawn(MyComponent(i));
            assert_eq!(entity, Entity::from(0, i));
        }
        assert_eq!(world.archetypes.len(), 1);
        assert_eq!(world.components.len(), 1);
        assert_eq!(world.entities.len(), BATCH_SIZE as usize);
        assert_eq!(world.tables.len(), 1);
    }
}
