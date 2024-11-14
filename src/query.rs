use std::marker::PhantomData;

use crate::{
    archetype::ArchetypeId,
    component::{Component, ComponentId},
    entity::Entity,
    storage::{Table, TableId, TableRow},
    World,
};

pub trait Queryable {
    type Item<'w>;
    type Fetcher<'w>;
    type State: Clone + Copy;

    fn init_fetcher<'w>(world: &'w World) -> Self::Fetcher<'w>;
    fn set_table<'w>(fetcher: &'w mut Self::Fetcher<'w>, table: &'w Table);

    fn fetch<'w>(
        fetcher: &'w mut Self::Fetcher<'w>,
        entity: Entity,
        row: TableRow,
    ) -> Self::Item<'w>;

    fn matches_components(state: &Self::State, f: &impl Fn(ComponentId) -> bool) -> bool;
}

pub struct ComponentFetcher<'w> {
    table: &'w Table,
}

impl<T: Component> Queryable for T {
    type Item<'w> = &'w T;
    type Fetcher<'w> = ComponentFetcher<'w>;
    type State = ComponentId;

    fn init_fetcher<'w>(world: &'w World) -> Self::Fetcher<'w> {
        todo!()
    }

    fn set_table<'w>(fetcher: &'w mut Self::Fetcher<'w>, table: &'w Table) {
        fetcher.table = table;
    }

    fn fetch<'w>(
        fetcher: &'w mut Self::Fetcher<'w>,
        entity: Entity,
        row: TableRow,
    ) -> Self::Item<'w> {
        todo!()
    }

    fn matches_components(state: &Self::State, f: &impl Fn(ComponentId) -> bool) -> bool {
        f(*state)
    }
}

pub struct Query<'world, T: Queryable> {
    world: &'world World,
    matched_tables: Vec<TableId>,
    fetcher: T::Fetcher<'world>,
    current_table: usize,
    current: usize,
}

impl<'world, T: Queryable> Query<'world, T> {
    pub(crate) fn new(world: &'world World) -> Self {
        let fetcher = T::init_fetcher(world);
        let mut matched_tables = Vec::new();

        todo!();

        Self {
            world,
            matched_tables,
            fetcher,
            current_table: 0,
            current: 0,
        }
    }
}

impl<'world, T: Queryable> Iterator for Query<'world, T> {
    type Item = T::Item<'world>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
        // if self.current_table >= self.table_ids.len() {
        //     return None;
        // }
        // let table_id = self.table_ids[self.current_table];
        // // TODO: do this more intelligently
        // let table = {
        //     let table = self.world.tables.get(table_id).expect("must exist");
        //     if self.current < table.len() {
        //         table
        //     } else {
        //         self.current_table += 1;
        //         self.current = 0; // Set the index inside the table to 0 for the new table
        //         if self.current_table >= self.table_ids.len() {
        //             return None;
        //         }
        //         let table_id = self.table_ids[self.current_table];
        //         self.world.tables.get(table_id).expect("must exist")
        //     }
        // };
        // let id = self.world.component_id::<T>()?;
        //
        // let ptr = unsafe { table.get_component(id, TableRow(self.current))? };
        //
        // unsafe {
        //     let ptr = table.get_component(id, TableRow(self.current))?;
        //
        //     self.current += 1;
        //
        //     Some(ptr.deref())
        // }
    }
}
