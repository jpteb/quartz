use core::{alloc::Layout, ptr::NonNull};

use std::{
    alloc::handle_alloc_error,
    collections::HashMap,
    ops::{Add, AddAssign},
};

use zerocopy::IntoBytes;

use crate::{
    component::{Component, ComponentId, ComponentInfo, Components},
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
            self.reserve_columns(additional);
        }
    }

    fn reserve_columns(&mut self, additional: usize) {
        for col in self.columns.values_mut() {
            col.reserve(additional);
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
pub(crate) struct Column<const N: usize> {
    item_layout: Layout,
    data: Vec<[u8; N]>,
    drop: Option<unsafe fn(OwningPtr<'_>)>,
    // len: usize,
    // capacity: usize,
}

impl<const N: usize> Column<N> {
    fn new(component_info: &ComponentInfo) -> Self {
        let item_layout = component_info.layout;
        let data = Vec::new();

        Self {
            item_layout,
            data,
            drop: component_info.drop,
            // len: 0,
            // capacity: 0,
        }
    }

    pub fn with_capacity(component_info: &ComponentInfo, capacity: usize) -> Self {
        let mut init = Self::new(component_info);
        if capacity != 0 {
            init.reserve(capacity);
        }
        init
    }

    fn is_zst(&self) -> bool {
        self.item_layout.size() == 0
    }

    fn len(&self) -> usize {
        debug_assert_eq!(self.data.len() % self.item_layout.size(), 0);
        self.data.len() / self.item_layout.size()
    }

    fn capacity(&self) -> usize {
        debug_assert_eq!(self.data.capacity() % self.item_layout.size(), 0);
        self.data.capacity() / self.item_layout.size()
    }

    pub fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional * self.item_layout.size());
    }

    pub fn push<T: Component>(&mut self, component: T) {
        let len = dbg!(self.len());
        let size = self.item_layout.size();
        if len == self.capacity() {
            self.reserve(1);
        }
        dbg!(&self.data.capacity());
        // component.write_to(&mut self.data[len..]);
        // // SAFETY: The necessary bytes have been allocated by the call to reserve.
        // // The data has been initialized by zerocopy with the write_to call above.
        // unsafe {
        //     self.data.set_len(len + size);
        // }
        self.data.push(component.as_bytes());
    }

    pub fn get<T: Component>(&self, index: usize) -> Option<&T> {
        let size = self.item_layout.size();
        let index = index * size;
        // Some(T::ref_from_bytes(&self.data[index..index + size]).unwrap())
        None
    }

    #[inline]
    fn get_ptr(&self) -> Ptr<'_> {
        let ptr = self.data.as_ptr();
        let nn = NonNull::new(ptr.cast_mut()).unwrap();
        unsafe { Ptr::new(nn) }
    }

    #[inline]
    fn get_ptr_mut(&mut self) -> MutPtr<'_> {
        let ptr = self.data.as_mut_ptr();
        let nn = NonNull::new(ptr).unwrap();
        unsafe { MutPtr::new(nn) }
    }

    pub(crate) unsafe fn initialize_unchecked(&mut self, index: usize, value: OwningPtr) {
        let size = self.item_layout.size();
        let dst = self.get_ptr_mut().byte_add(index * size);
        //TODO: is this always nonoverlapping?
        std::ptr::copy_nonoverlapping(value.as_ptr(), dst.as_ptr(), size);
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
            let len = self.len();

            for i in 0..len {
                let item = self.get_ptr_mut().byte_add(i * size).promote();

                drop(item);
            }

            self.drop = Some(drop);
        }
        self.data.clear();
    }

    fn drop_last(&mut self) {
        let size = self.item_layout.size();
        let len = self.len();
        if let Some(drop) = self.drop {
            self.drop = None;

            unsafe {
                let item = self.get_ptr_mut().byte_add(len * size).promote();

                drop(item);
            }

            self.drop = Some(drop);
        }
        self.data.truncate(len - size);
    }

    fn swap_remove(&mut self, index: usize) {
        debug_assert_ne!(index, self.len() - 1);
        unsafe {
            core::ptr::swap_nonoverlapping::<u8>(
                self.get_unchecked_mut(index).as_ptr(),
                self.get_unchecked_mut(self.len() - 1).as_ptr(),
                self.item_layout.size(),
            )
        };
        // self.drop_last();
    }
}

impl<const N: usize> Drop for Column<N> {
    fn drop(&mut self) {
        unsafe {
            if self.capacity() != 0 {
                self.clear();
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

    use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};
    #[derive(Debug, PartialEq, IntoBytes, FromBytes, Immutable, KnownLayout)]
    struct MyComponent {
        // _position: (f32, f32, f32),
        _position: [f32; 3],
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
            _position: [1.0, 2.0, 3.0],
        };
        let c2 = MyComponent {
            _position: [3.0, 2.0, 1.0],
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
    fn push_column() {
        let mut components = Components::new();
        let component_id = components.register_component::<MyComponent>();
        let component_info = components.get_info(&component_id).unwrap();

        let mut column = Column::with_capacity(&component_info, 5);
        assert_eq!(column.data.capacity(), component_info.layout.size() * 5);

        let c1 = MyComponent {
            _position: [1.0, 2.0, 3.0],
        };

        column.push(c1);
        assert_eq!(
            column.get(0),
            Some(&MyComponent {
                _position: [1.0, 2.0, 3.0],
            })
        );
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
        assert_eq!(column.len(), 5);

        unsafe {
            let ptr = column.get_unchecked(2);
            assert_eq!(ptr.deref::<u32>(), &2);
        }

        column.swap_remove(2);
        assert_eq!(column.len(), 4);

        unsafe {
            let ptr = column.get_unchecked(2);
            assert_eq!(ptr.deref::<u32>(), &4);
        }

        column.drop_last();
        assert_eq!(column.len(), 3);
        column.drop_last();
        assert_eq!(column.len(), 2);

        unsafe {
            let ptr = column.get_unchecked(1);
            assert_eq!(ptr.deref::<u32>(), &1);
        }
    }
}
