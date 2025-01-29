use core::{alloc::Layout, ptr::NonNull};

use std::{
    alloc::handle_alloc_error,
    collections::HashMap,
    ops::{Add, AddAssign},
};

use crate::{
    component::{ComponentId, ComponentInfo, Components},
    entity::Entity,
    ptr::{MutPtr, OwningPtr, Ptr},
};

#[derive(Debug, Default)]
pub(crate) struct Tables {
    tables: Vec<Table>,
    table_index: HashMap<Box<[ComponentId]>, TableId>,
}

impl Tables {
    pub(crate) fn get_id_or_insert(
        &mut self,
        ids: &[ComponentId],
        components: &Components,
    ) -> TableId {
        *self.table_index.entry(ids.into()).or_insert_with(|| {
            let id = TableId(self.tables.len());
            self.tables.push(Table::from_components(ids, components));
            id
        })
    }

    pub(crate) fn get(&self, id: TableId) -> Option<&Table> {
        self.tables.get(id.index())
    }

    pub(crate) fn get_mut(&mut self, id: TableId) -> Option<&mut Table> {
        self.tables.get_mut(id.index())
    }

    /// Retrieves the [`Table`] for the given [`TableId`].
    ///
    /// Panics: If the given id does not exist inside this world.
    pub(crate) fn get_mut_unchecked(&mut self, id: TableId) -> &mut Table {
        &mut self.tables[id.index()]
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.tables.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TableId(pub(crate) usize);

impl TableId {
    pub(crate) fn index(&self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TableRow(pub(crate) usize);

impl TableRow {
    #[inline]
    pub(crate) const fn index(&self) -> usize {
        self.0
    }
}

impl Add for TableRow {
    type Output = TableRow;

    fn add(self, rhs: Self) -> Self::Output {
        TableRow(self.0 + rhs.0)
    }
}

impl AddAssign<usize> for TableRow {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl PartialOrd<usize> for TableRow {
    fn partial_cmp(&self, other: &usize) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl PartialEq<usize> for TableRow {
    fn eq(&self, other: &usize) -> bool {
        self.0.eq(other)
    }
}

#[derive(Debug)]
pub struct Table {
    columns: HashMap<ComponentId, Column>,
    entities: Vec<Entity>,
}

impl Table {
    pub(crate) fn from_components(ids: &[ComponentId], components: &Components) -> Self {
        let mut table = Self {
            columns: HashMap::new(),
            entities: Vec::new(),
        };

        ids.iter()
            .map(|id| (id, components.get_info(id).unwrap()))
            .for_each(|(id, info)| {
                table.columns.insert(*id, Column::with_capacity(info, 0));
            });

        table
    }

    fn capacity(&self) -> usize {
        self.entities.capacity()
    }

    pub(crate) fn len(&self) -> usize {
        self.entities.len()
    }

    pub(crate) fn allocate(&mut self, entity: Entity) -> TableRow {
        self.reserve(1);
        let table_row = TableRow(self.len());
        self.entities.push(entity);

        table_row
    }

    pub(crate) fn reserve(&mut self, additional: usize) {
        if self.capacity() - self.len() < additional {
            self.entities.reserve(additional);
            self.realloc_columns(self.capacity() + additional);
        }
    }

    fn realloc_columns(&mut self, new_capacity: usize) {
        for col in self.columns.values_mut() {
            col.realloc(new_capacity);
        }
    }

    fn get_column(&self, id: ComponentId) -> Option<&Column> {
        self.columns.get(&id)
    }

    pub(crate) fn get_column_mut(&mut self, id: ComponentId) -> Option<&mut Column> {
        self.columns.get_mut(&id)
    }

    pub(crate) unsafe fn get_component(&self, id: ComponentId, row: TableRow) -> Option<Ptr<'_>> {
        self.get_column(id)
            .map(|col| col.get_unchecked(row.index()))
    }

    pub(crate) unsafe fn get_component_mut(
        &mut self,
        id: ComponentId,
        row: TableRow,
    ) -> Option<MutPtr<'_>> {
        self.get_column_mut(id)
            .map(|col| col.get_unchecked_mut(row.index()))
    }

    pub(crate) fn swap_remove(&mut self, table_row: TableRow) {
        let index = table_row.index();
        if index == self.entities.len() - 1 {
            for col in self.columns.values_mut() {
                col.drop_last();
            }
        } else {
            for col in self.columns.values_mut() {
                col.swap_remove(index);
            }
        }
        self.entities.swap_remove(index);
    }
}

impl Drop for Table {
    fn drop(&mut self) {
        self.entities.clear();
    }
}

#[derive(Debug)]
pub(crate) struct Column {
    item_layout: Layout,
    data: NonNull<u8>,
    drop: Option<unsafe fn(OwningPtr<'_>)>,
    len: usize,
    capacity: usize,
}

impl Column {
    fn new(component_info: &ComponentInfo) -> Self {
        let item_layout = component_info.layout;
        let data = unsafe {
            NonNull::new_unchecked(std::ptr::without_provenance_mut(item_layout.align()))
        };

        Self {
            item_layout,
            data,
            drop: component_info.drop,
            len: 0,
            capacity: 0,
        }
    }

    pub fn with_capacity(component_info: &ComponentInfo, capacity: usize) -> Self {
        let item_layout = component_info.layout;

        // let data = if capacity == 0 || item_layout.size() == 0 {
        //     // create an aligned dangling pointer
        //     unsafe { NonNull::new_unchecked(std::ptr::without_provenance_mut(item_layout.align())) }
        // } else {
        //     let (array_layout, _off) = item_layout
        //         .repeat(capacity)
        //         .expect("Array layout creation should be successful!");
        //
        //     let data = unsafe { std::alloc::alloc(array_layout) };
        //     NonNull::new(data).unwrap_or_else(|| handle_alloc_error(array_layout))
        // };
        //
        // Self {
        //     item_layout,
        //     data,
        //     drop: component_info.drop,
        //     len: 0,
        //     capacity,
        // }
        let mut init = Self::new(component_info);
        if capacity != 0 {
            init.realloc(capacity);
        }
        init
    }

    fn is_zst(&self) -> bool {
        self.item_layout.size() == 0
    }

    fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn realloc(&mut self, new_capacity: usize) {
        if !self.is_zst() {
            let (array_layout, _) = self
                .item_layout
                .repeat(self.capacity)
                .expect("Array layout creation should succeed");

            let (new_layout, _) = self
                .item_layout
                .repeat(new_capacity)
                .expect("Array layout creation should succeed");

            let data = if self.capacity() != 0 {
                unsafe { std::alloc::realloc(self.data.as_ptr(), array_layout, new_layout.size()) }
            } else {
                unsafe { std::alloc::alloc(new_layout) }
            };

            self.data = NonNull::new(data).unwrap_or_else(|| handle_alloc_error(new_layout));
        }
        self.capacity = new_capacity;
    }

    #[inline]
    fn get_ptr(&self) -> Ptr<'_> {
        unsafe { Ptr::new(self.data) }
    }

    #[inline]
    fn get_ptr_mut(&mut self) -> MutPtr<'_> {
        unsafe { MutPtr::new(self.data) }
    }

    pub(crate) unsafe fn initialize_unchecked(&mut self, index: usize, value: OwningPtr) {
        let size = self.item_layout.size();
        let dst = self.data.byte_add(index * size);
        //TODO: is this always nonoverlapping?
        std::ptr::copy_nonoverlapping(value.as_ptr(), dst.as_ptr(), size);
        self.len += 1;
    }

    unsafe fn get_unchecked(&self, index: usize) -> Ptr<'_> {
        self.get_ptr().byte_add(self.item_layout.size() * index)
    }

    unsafe fn get_unchecked_mut(&mut self, index: usize) -> MutPtr<'_> {
        let size = self.item_layout.size();
        self.get_ptr_mut().byte_add(size * index)
    }

    unsafe fn clear(&mut self) {
        if let Some(drop) = self.drop {
            self.drop = None;
            let size = self.item_layout.size();
            let len = self.len;

            for i in 0..len {
                let item = self.get_ptr_mut().byte_add(i * size).promote();

                drop(item);
            }

            self.drop = Some(drop);
        }
    }

    fn drop_last(&mut self) {
        let size = self.item_layout.size();
        let len = self.len;
        if let Some(drop) = self.drop {
            self.drop = None;

            unsafe {
                let item = self.get_ptr_mut().byte_add(len * size).promote();

                drop(item);
            }

            self.drop = Some(drop);
        }
        self.len -= 1;
    }

    fn swap_remove(&mut self, index: usize) {
        debug_assert_ne!(index, self.len - 1);
        unsafe {
            core::ptr::swap_nonoverlapping::<u8>(
                self.get_unchecked_mut(index).as_ptr(),
                self.get_unchecked_mut(self.len - 1).as_ptr(),
                self.item_layout.size(),
            )
        };
        // self.drop_last();
        self.len -= 1;
    }
}

impl Drop for Column {
    fn drop(&mut self) {
        unsafe {
            if self.capacity != 0 {
                self.clear();
                if !self.is_zst() {
                    std::alloc::dealloc(
                        self.data.as_ptr(),
                        self.item_layout
                            .repeat(self.capacity)
                            .expect("Array layout creation should be successful")
                            .0,
                    )
                }
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        component::{Component, Components},
        ptr::OwningPtr,
    };

    use super::{Column, Tables};

    struct MyComponent {
        _position: (f32, f32, f32),
    }
    impl Component for MyComponent {}

    #[test]
    fn create_column() {
        let mut components = Components::new();
        let component_id = components.register_component::<MyComponent>();
        let component_info = components.get_info(&component_id).unwrap();

        let mut column = Column::with_capacity(component_info, 5);
        assert_eq!(column.item_layout, component_info.layout);

        let c1 = MyComponent {
            _position: (1.0, 2.0, 3.0),
        };
        let c2 = MyComponent {
            _position: (3.0, 2.0, 1.0),
        };

        OwningPtr::make(c1, |ptr| unsafe { column.initialize_unchecked(0, ptr) });

        let mut ptr: *const f32 = column.data.as_ptr().cast();
        for i in 1..4 {
            unsafe {
                assert_eq!(*ptr, i as f32);
                ptr = ptr.add(1);
            }
        }
        OwningPtr::make(c2, |ptr| unsafe { column.initialize_unchecked(1, ptr) });
        for i in (1..4).rev() {
            unsafe {
                assert_eq!(*ptr, i as f32);
                ptr = ptr.add(1);
            }
        }
    }

    #[test]
    fn tables() {
        let mut tables = Tables::default();
        let mut components = Components::new();

        let comp_id1 = components.register_component::<MyComponent>();
        let comp_id2 = components.register_component::<u32>();

        let comp_mix1 = vec![comp_id1];
        let comp_mix12 = vec![comp_id1, comp_id2];

        let table_id1 = tables.get_id_or_insert(&comp_mix1, &components);
        let table_id2 = tables.get_id_or_insert(&comp_mix1, &components);
        assert_eq!(table_id1, table_id2);
        let table_id3 = tables.get_id_or_insert(&comp_mix12, &components);
        assert_ne!(table_id1, table_id3);
        assert_eq!(tables.tables.len(), 2);
        assert_eq!(tables.table_index.len(), 2);
    }

    #[test]
    fn column_get_component() {
        let mut components = Components::new();
        let component_id = components.register_component::<u32>();
        let component_info = components.get_info(&component_id).unwrap();

        let my_comp: u32 = 5;

        let mut column = Column::with_capacity(component_info, 1);
        OwningPtr::make(my_comp, |ptr| unsafe {
            column.initialize_unchecked(0, ptr)
        });

        unsafe {
            let ptr = column.get_unchecked(0);
            assert_eq!(ptr.deref::<u32>(), &5);
        }
    }

    #[test]
    fn swap_remove() {
        const COMP_COUNT: usize = 5;
        let mut components = Components::new();
        let component_id = components.register_component::<u32>();
        let component_info = components.get_info(&component_id).unwrap();

        let mut column = Column::with_capacity(component_info, COMP_COUNT);
        assert_eq!(column.capacity(), COMP_COUNT);

        for i in 0..COMP_COUNT {
            OwningPtr::make(i as u32, |ptr| unsafe {
                column.initialize_unchecked(i, ptr)
            });
        }
        assert_eq!(column.len, 5);

        unsafe {
            let ptr = column.get_unchecked(2);
            assert_eq!(ptr.deref::<u32>(), &2);
        }

        column.swap_remove(2);
        assert_eq!(column.len, 4);

        unsafe {
            let ptr = column.get_unchecked(2);
            assert_eq!(ptr.deref::<u32>(), &4);
        }

        column.drop_last();
        assert_eq!(column.len, 3);
        column.drop_last();
        assert_eq!(column.len, 2);

        unsafe {
            let ptr = column.get_unchecked(1);
            assert_eq!(ptr.deref::<u32>(), &1);
        }
    }
}
