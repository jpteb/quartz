#![feature(alloc_layout_extra)]
#![allow(unused)]
pub mod archetype;
pub mod component;
pub mod entity;
pub mod ptr;
pub mod query;
pub mod storage;

use archetype::Archetypes;
use component::{Bundle, Component, ComponentId, Components};
use entity::{Entities, Entity, EntityLocation};
use query::{MutQuery, Query, Queryable};
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

    pub fn despawn(&mut self, entity: Entity) {
        if let Some(location) = self.entities.free(entity) {
            let archetype = self.archetypes.get_mut_unchecked(location.archetype_id);
            if let Some(swapped_entity) = archetype.swap_remove(location.table_row) {
                let swap_location = self
                    .entities
                    .get(swapped_entity)
                    .expect("Entity must exist, as it was just swapped");

                self.entities.set(
                    swapped_entity,
                    EntityLocation {
                        table_row: location.table_row,
                        ..*swap_location
                    },
                );
            }

            let table = self.tables.get_mut_unchecked(location.table_id);
            table.swap_remove(location.table_row);
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

    pub fn query<'w, T: Queryable<'w>>(&'w self) -> Query<'w, T> {
        Query::new(self)
    }

    pub fn query_mut<'w, T: Queryable<'w>>(&'w mut self) -> MutQuery<'w, T> {
        MutQuery::new(self)
    }

    pub fn component_id<T: Component>(&self) -> Option<ComponentId> {
        self.components.component_id::<T>()
    }
}

#[cfg(test)]
mod tests {
    use archetype::ArchetypeId;
    use component::Component;
    use storage::{TableId, TableRow};

    use super::*;

    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    struct MyComponent(u32);
    impl Component for MyComponent {}

    #[derive(Debug, PartialEq, Clone, Copy)]
    struct Position {
        x: f32,
        y: f32,
        z: f32,
    }
    impl Component for Position {}

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
            assert_eq!(world.get::<MyComponent>(entity), Some(&MyComponent(i)));
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
            Position {
                x: 0.0,
                y: 1.0,
                z: 2.0,
            },
        ));
        assert_eq!(entity, Entity::from(0, 0));

        assert_eq!(world.get::<MyComponent>(entity), Some(&MyComponent(0)));
        assert_eq!(
            world.get::<Position>(entity),
            Some(&Position {
                x: 0.0,
                y: 1.0,
                z: 2.0,
            }),
        );
    }

    #[test]
    fn world_get() {
        let mut world = World::new();

        let entity = world.spawn(Position {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        });

        assert_eq!(
            world.get::<Position>(entity),
            Some(&Position {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            })
        );

        let comp = world
            .get_mut::<Position>(entity)
            .expect("Was spawned a few instructions ago");
        comp.z = 42.0;

        assert_eq!(
            world.get::<Position>(entity),
            Some(&Position {
                x: 1.0,
                y: 2.0,
                z: 42.0,
            })
        );
    }

    #[test]
    fn despawn() {
        let mut world = World::new();

        let e0 = world.spawn(Position {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        });
        let e1 = world.spawn(Position {
            x: 2.0,
            y: 3.0,
            z: 4.0,
        });

        assert_eq!(
            world.get::<Position>(e0),
            Some(&Position {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            })
        );
        assert_eq!(
            world.entities.get(e0),
            Some(&EntityLocation {
                archetype_id: ArchetypeId(0),
                table_id: TableId(0),
                table_row: TableRow(0)
            })
        );
        assert_eq!(
            world.get::<Position>(e1),
            Some(&Position {
                x: 2.0,
                y: 3.0,
                z: 4.0,
            })
        );
        assert_eq!(
            world.entities.get(e1),
            Some(&EntityLocation {
                archetype_id: ArchetypeId(0),
                table_id: TableId(0),
                table_row: TableRow(1)
            })
        );

        world.despawn(e0);

        assert_eq!(world.get::<Position>(e0), None);
        assert_eq!(world.entities.get(e0), None);
        assert_eq!(
            world.entities.get(e1),
            Some(&EntityLocation {
                archetype_id: ArchetypeId(0),
                table_id: TableId(0),
                table_row: TableRow(0)
            })
        );
        assert_eq!(
            world.get::<Position>(e1),
            Some(&Position {
                x: 2.0,
                y: 3.0,
                z: 4.0,
            })
        );

        world.despawn(e1);
        assert_eq!(world.get::<Position>(e1), None);
        assert_eq!(world.entities.get(e1), None);
    }

    #[test]
    fn swap_remove() {
        let mut world = World::new();
        let e0 = world.spawn(MyComponent(0));
        let e1 = world.spawn(MyComponent(1));
        let e2 = world.spawn(MyComponent(2));

        world.despawn(e0);
        assert_eq!(world.get::<MyComponent>(e2), Some(&MyComponent(2)));
        assert_eq!(world.get::<MyComponent>(e1), Some(&MyComponent(1)));
        assert_eq!(world.get::<MyComponent>(e0), None);

        world.despawn(e2);
        assert_eq!(world.get::<MyComponent>(e2), None);
        assert_eq!(world.get::<MyComponent>(e1), Some(&MyComponent(1)));
        assert_eq!(world.get::<MyComponent>(e0), None);

        world.despawn(e1);
        assert_eq!(world.get::<MyComponent>(e2), None);
        assert_eq!(world.get::<MyComponent>(e1), None);
        assert_eq!(world.get::<MyComponent>(e0), None);
    }

    // #[test]
    // fn query() {
    //     let mut world = World::new();
    //
    //     let e0 = world.spawn(MyComponent(0));
    //     let e1 = world.spawn(MyComponent(1));
    //     let e2 = world.spawn(MyComponent(2));
    //
    //     let mut query = world.query::<MyComponent>();
    //
    //     assert_eq!(query.next(), Some(&MyComponent(0)));
    //     assert_eq!(query.next(), Some(&MyComponent(1)));
    //     assert_eq!(query.next(), Some(&MyComponent(2)));
    //     assert_eq!(query.next(), None);
    // }
    //
    // #[test]
    // fn query_multiple() {
    //     let mut world = World::new();
    //
    //     let e0 = world.spawn(MyComponent(0));
    //     let e1 = world.spawn(MyComponent(1));
    //     let e2 = world.spawn((
    //         Position {
    //             x: 1.0,
    //             y: 2.0,
    //             z: 3.0,
    //         },
    //         MyComponent(2),
    //     ));
    //
    //     let mut query = world.query::<MyComponent>();
    //
    //     assert_eq!(query.next(), Some(&MyComponent(0)));
    //     assert_eq!(query.next(), Some(&MyComponent(1)));
    //     assert_eq!(query.next(), Some(&MyComponent(2)));
    //     assert_eq!(query.next(), None);
    //
    //     let mut query = world.query::<Position>();
    //     assert_eq!(
    //         query.next(),
    //         Some(&Position {
    //             x: 1.0,
    //             y: 2.0,
    //             z: 3.0,
    //         })
    //     );
    //     assert_eq!(query.next(), None);
    // }
}
