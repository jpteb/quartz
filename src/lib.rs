#![feature(alloc_layout_extra)]
pub mod archetype;
pub mod component;
pub mod entity;
pub mod ptr;
pub mod storage;

use archetype::Archetypes;
use component::{Bundle, Component, Components};
use entity::{Entities, Entity};
use storage::Tables;

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
            entities: Entities::new(),
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
                        .get_id_or_insert(table_id, &component_ids);

                let table_row = {
                    // Safety: The table id was just received or created in the call above
                    let table = self.tables.get_mut_unchecked(table_id);
                    let row = table.allocate(entity);
                    bundle.get(&mut self.components, &mut |id, ptr| unsafe {
                        table
                            .get_column_mut(id)
                            .expect("the selected table must have the correct column for this component")
                            .initialize_unchecked(row.index(), ptr);
                    });
                    row
                };

                let location = self.archetypes.get_mut_unchecked(archetype_id).allocate(entity, table_row);

                Ok(location)
            })
            .expect("entity allocation should not fail")
    }

    // TODO: finish this
    pub fn despawn(&mut self, entity: Entity) {
        if let Some(location) = self.entities.free(entity) {
            let archetype = self.archetypes.get_mut_unchecked(location.archetype_id);
            if let Some(swapped_entity) = archetype.swap_remove(location.table_row) {
                let swap_location = self
                    .entities
                    .get(swapped_entity)
                    .expect("Entity must exist, as it was just swapped");
            }
        }
    }

    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        let component_id = self.components.component_id::<T>()?;
        let location = self.entities.get(entity)?;
        let table = self.tables.get(location.table_id)?;

        unsafe {
            let ptr = table.get_component(component_id, location.table_row)?;

            Some(ptr.deref::<T>())
        }
    }

    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        let component_id = self.components.component_id::<T>()?;
        let location = self.entities.get(entity)?;
        let table = self.tables.get_mut(location.table_id)?;

        unsafe {
            let ptr = table.get_component_mut(component_id, location.table_row)?;

            Some(ptr.deref_mut::<T>())
        }
    }
}

#[cfg(test)]
mod tests {
    use component::Component;

    use super::*;

    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    struct MyComponent(u32);
    impl Component for MyComponent {}

    #[derive(Debug, PartialEq, Clone, Copy)]
    struct MySecond {
        x: f32,
        y: f32,
        z: f32,
    }
    impl Component for MySecond {}

    #[test]
    fn spawn() {
        let mut world = World::new();
        let entity = world.spawn(MyComponent(1));

        assert_eq!(entity, Entity::from(0, 0));

        assert_eq!(world.get::<MyComponent>(entity), Some(&MyComponent(1)));
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
        const BATCH_SIZE: u32 = 1_000;
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

    #[test]
    fn spawn_bundle() {
        let mut world = World::new();

        let entity = world.spawn((
            MyComponent(0),
            MySecond {
                x: 0.0,
                y: 1.0,
                z: 2.0,
            },
        ));
        assert_eq!(entity, Entity::from(0, 0));

        assert_eq!(world.get::<MyComponent>(entity), Some(&MyComponent(0)));
        assert_eq!(
            world.get::<MySecond>(entity),
            Some(&MySecond {
                x: 0.0,
                y: 1.0,
                z: 2.0,
            }),
        );
    }

    #[test]
    fn world_get() {
        let mut world = World::new();

        let entity = world.spawn(MySecond {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        });

        assert_eq!(
            world.get::<MySecond>(entity),
            Some(&MySecond {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            })
        );

        let comp = world
            .get_mut::<MySecond>(entity)
            .expect("Was spawned a few instructions ago");
        comp.z = 42.0;

        assert_eq!(
            world.get::<MySecond>(entity),
            Some(&MySecond {
                x: 1.0,
                y: 2.0,
                z: 42.0,
            })
        );
    }

    #[test]
    fn world_get_none() {
        let mut world = World::new();

        let entity = world.spawn(MySecond {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        });

        assert_eq!(
            world.get::<MySecond>(entity),
            Some(&MySecond {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            })
        );

        world.despawn(entity);

        assert_eq!(world.get::<MySecond>(entity), None);
    }
}
