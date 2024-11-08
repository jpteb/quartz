use std::marker::PhantomData;

use crate::{
    archetype::ArchetypeId,
    component::Component,
    storage::{TableId, TableRow},
    World,
};

pub struct Query<'world, T: Component> {
    world: &'world World,
    archetype_ids: Vec<ArchetypeId>,
    table_ids: Vec<TableId>,
    current_table: usize,
    current: usize,
    _phtm: PhantomData<&'world T>,
}

impl<'world, T: Component> Query<'world, T> {
    pub(crate) fn new(
        world: &'world World,
        archetype_ids: Vec<ArchetypeId>,
        table_ids: Vec<TableId>,
    ) -> Self {
        Self {
            world,
            archetype_ids,
            table_ids,
            current_table: 0,
            current: 0,
            _phtm: PhantomData,
        }
    }
}

impl<'world, T: Component> Iterator for Query<'world, T> {
    type Item = &'world T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_table >= self.table_ids.len() {
            return None;
        }
        let table_id = self.table_ids[self.current_table];
        // TODO: do this more intelligently
        let table = {
            let table = self.world.tables.get(table_id).expect("must exist");
            if self.current < table.len() {
                table
            } else {
                self.current_table += 1;
                self.current = 0; // Set the index inside the table to 0 for the new table
                if self.current_table >= self.table_ids.len() {
                    return None;
                }
                let table_id = self.table_ids[self.current_table];
                self.world.tables.get(table_id).expect("must exist")
            }
        };
        let id = self.world.component_id::<T>()?;

        let ptr = unsafe { table.get_component(id, TableRow(self.current))? };

        unsafe {
            let ptr = table.get_component(id, TableRow(self.current))?;

            self.current += 1;

            Some(ptr.deref())
        }
    }
}
