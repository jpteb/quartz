use std::marker::PhantomData;

use crate::{
    archetype::ArchetypeId,
    component::{Component, ComponentId},
    entity::Entity,
    storage::{Table, TableId, TableRow},
    World,
};

pub trait Queryable<'w> {
    type Item;
    type State;

    fn init_state(world: &World) -> Self::State;
    fn fetch(world: &'w World, state: &Self::State, table: &'w Table, row: TableRow) -> Self::Item;
    fn get_component_ids(world: &World) -> Vec<ComponentId>;
}

pub struct ComponentFetcher<'w> {
    table: Option<&'w Table>,
}

impl<'w, T: Component> Queryable<'w> for &T {
    type Item = &'w T;
    type State = ComponentId;

    fn init_state(world: &World) -> Self::State {
        world
            .component_id::<T>()
            .expect("Tried to query a Component that has not been spawned in the world")
    }

    fn fetch(world: &'w World, state: &Self::State, table: &'w Table, row: TableRow) -> Self::Item {
        unsafe {
            let ptr = table
                .get_component(*state, row)
                .expect("failed to receive item from table");
            ptr.deref()
        }
    }

    fn get_component_ids(world: &World) -> Vec<ComponentId> {
        vec![world
            .component_id::<T>()
            .expect("Tried to query a Component that has not been spawned in the world")]
    }
}

impl<'w, T1: Component, T2: Component> Queryable<'w> for (&T1, &T2) {
    type Item = (&'w T1, &'w T2);
    type State = (ComponentId, ComponentId);

    fn fetch(world: &'w World, state: &Self::State, table: &'w Table, row: TableRow) -> Self::Item {
        let (id0, id1) = state;
        unsafe {
            let ptr1 = table
                .get_component(*id0, row)
                .expect("failed to receive item from table");
            let ptr2 = table
                .get_component(*id1, row)
                .expect("failed to receive item from table");
            (ptr1.deref(), ptr2.deref())
        }
    }

    fn init_state(world: &World) -> Self::State {
        (
            world
                .component_id::<T1>()
                .expect("Tried to query a Component that has not been spawned in the world"),
            world
                .component_id::<T2>()
                .expect("Tried to query a Component that has not been spawned in the world"),
        )
    }

    fn get_component_ids(world: &World) -> Vec<ComponentId> {
        vec![
            world
                .component_id::<T1>()
                .expect("Component needs to be initialized for this world"),
            world
                .component_id::<T2>()
                .expect("Component needs to be initialized for this world"),
        ]
    }
}

pub struct Query<'world, T: Queryable<'world>> {
    world: &'world World,
    matched_tables: Vec<TableId>,
    current_table: usize,
    current_row: TableRow,
    state: T::State,
}

impl<'world, T: Queryable<'world>> Query<'world, T> {
    pub(crate) fn new(world: &'world World) -> Self {
        let mut matched_tables: Vec<TableId> = Vec::new();
        let component_ids = T::get_component_ids(world);
        let (archetype_ids, matched_tables) = world.archetypes.get_query_archetypes(&component_ids);
        let state = T::init_state(world);

        Self {
            world,
            matched_tables,
            current_table: 0,
            current_row: TableRow(0),
            state,
        }
    }
}

impl<'world, T: Queryable<'world>> Iterator for Query<'world, T> {
    type Item = T::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_table >= self.matched_tables.len() {
            return None;
        }
        let table = {
            let table_id = self.matched_tables[self.current_table];
            self.world.tables.get(table_id)?
        };
        if self.current_row >= table.len() {
            self.current_table += 1;
            self.current_row = TableRow(0);
            return self.next();
        }

        let row = self.current_row;
        self.current_row += 1;
        Some(T::fetch(self.world, &self.state, table, row))
    }
}

#[cfg(test)]
mod tests {
    use crate::{component::Component, entity::Entity, World};

    use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};
    #[derive(Debug, PartialEq, Eq, Clone, Copy, IntoBytes, FromBytes, Immutable, KnownLayout)]
    struct MyComponent(u32);
    impl Component for MyComponent {}

    #[derive(Debug, PartialEq, Clone, Copy, IntoBytes, FromBytes, Immutable, KnownLayout)]
    struct Position {
        x: f32,
        y: f32,
        z: f32,
    }
    impl Component for Position {}

    #[test]
    fn query() {
        let mut world = World::new();
        let entity = world.spawn(MyComponent(1));

        assert_eq!(entity, Entity::from(0, 0));

        let mut query = world.query::<&MyComponent>();
        assert_eq!(query.next(), Some(&MyComponent(1)));
        assert_eq!(query.next(), None);
    }

    #[test]
    fn multi_query() {
        let mut world = World::new();
        let entity = world.spawn(MyComponent(1));
        let entity2 = world.spawn((
            MyComponent(1337),
            Position {
                x: 0.0,
                y: 1.0,
                z: 2.0,
            },
        ));

        assert_eq!(entity, Entity::from(0, 0));
        assert_eq!(entity2, Entity::from(0, 1));

        let mut query = world.query::<&MyComponent>();
        assert_eq!(query.next(), Some(&MyComponent(1)));
        assert_eq!(query.next(), Some(&MyComponent(1337)));
        assert_eq!(query.next(), None);

        let mut query = world.query::<&Position>();
        assert_eq!(
            query.next(),
            Some(&Position {
                x: 0.0,
                y: 1.0,
                z: 2.0,
            })
        );
        assert_eq!(query.next(), None);

        let mut query = world.query::<(&MyComponent, &Position)>();
        assert_eq!(
            query.next(),
            Some((
                &MyComponent(1337),
                &Position {
                    x: 0.0,
                    y: 1.0,
                    z: 2.0,
                }
            ))
        );
        assert_eq!(query.next(), None);
    }

    #[test]
    fn large_query() {
        const ENTITY_COUNT: u32 = 1000;
        let mut world = World::new();

        for i in 0..ENTITY_COUNT {
            world.spawn(MyComponent(i));
        }

        let mut count = 0;
        for (i, e) in world.query::<&MyComponent>().enumerate() {
            assert_eq!(e, &MyComponent(i as u32));
            count += 1;
        }
        assert_eq!(count, ENTITY_COUNT);
    }
}
