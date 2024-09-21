use core::{
    alloc::Layout,
    fmt::{self, Formatter, Pointer},
    marker::PhantomData,
    ptr::NonNull,
};

use std::{
    alloc::handle_alloc_error,
    collections::HashMap,
    mem::{self, ManuallyDrop},
};

use crate::{
    component::{ComponentId, ComponentInfo, Components},
    Entity,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TableId(usize);

impl TableId {
    pub(crate) fn index(&self) -> usize {
        self.0
    }
}

#[derive(Debug)]
pub(crate) struct TableRow(usize);

impl TableRow {
    #[inline]
    const fn from(index: usize) -> Self {
        Self(index)
    }

    #[inline]
    const fn index(&self) -> usize {
        self.0
    }
}

#[derive(Debug)]
pub(crate) struct Table {
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

    fn get_column(&self, id: ComponentId) -> Option<&Column> {
        self.columns.get(&id)
    }

    unsafe fn get_component(&self, id: ComponentId, row: TableRow) -> Option<Ptr<'_>> {
        self.get_column(id)
            .map(|col| col.get_unchecked(row.index()))
    }
}

impl Drop for Table {
    fn drop(&mut self) {
        let len = self.entities.len();
        self.entities.clear();

        for col in self.columns.values_mut() {
            unsafe {
                col.drop(len);
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct Column {
    item_layout: Layout,
    data: NonNull<u8>,
    drop: Option<unsafe fn(OwningPtr<'_>)>,
    capacity: usize,
}

impl Column {
    pub fn with_capacity(component_info: &ComponentInfo, capacity: usize) -> Self {
        let item_layout = component_info.layout;
        let (array_layout, _off) = item_layout
            .repeat(capacity)
            .expect("Array layout creation should be successful!");

        let data = if capacity == 0 {
            // create an aligned dangling pointer
            unsafe { NonNull::new_unchecked(std::ptr::without_provenance_mut(item_layout.align())) }
        } else {
            let data = unsafe { std::alloc::alloc(array_layout) };
            NonNull::new(data).unwrap_or_else(|| handle_alloc_error(array_layout))
        };

        Self {
            item_layout,
            data,
            drop: component_info.drop,
            capacity,
        }
    }

    fn is_zst(&self) -> bool {
        self.item_layout.size() == 0
    }

    pub fn realloc(&mut self, new_capacity: usize) {
        let (array_layout, _) = self
            .item_layout
            .repeat(self.capacity)
            .expect("Array layout creation should succeed");

        let (new_layout, _) = self
            .item_layout
            .repeat(new_capacity)
            .expect("Array layout creation should succeed");

        let data =
            unsafe { std::alloc::realloc(self.data.as_ptr(), array_layout, new_layout.size()) };

        self.data = NonNull::new(data).unwrap_or_else(|| handle_alloc_error(new_layout));
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

    pub fn initialize(&mut self, index: usize, value: OwningPtr) {
        if index < self.capacity {
            unsafe { self.initialize_unchecked(index, value) };
        }
    }

    pub unsafe fn initialize_unchecked(&mut self, index: usize, value: OwningPtr) {
        let size = self.item_layout.size();
        let dst = self.data.byte_add(index * size);
        std::ptr::copy_nonoverlapping(value.as_ptr(), dst.as_ptr(), size);
    }

    pub unsafe fn get_unchecked(&self, index: usize) -> Ptr<'_> {
        self.get_ptr().byte_add(self.item_layout.size() * index)
    }

    unsafe fn clear(&mut self, len: usize) {
        if let Some(drop) = self.drop {
            self.drop = None;
            let size = self.item_layout.size();

            for i in 0..len {
                let item = self.get_ptr_mut().byte_add(i * size).promote();

                drop(item);
            }

            self.drop = Some(drop);
        }
    }

    fn drop(&mut self, len: usize) {
        unsafe {
            if self.capacity != 0 {
                self.clear(len);
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

#[derive(Debug)]
#[repr(transparent)]
pub(crate) struct Ptr<'a>(NonNull<u8>, PhantomData<&'a u8>);
#[derive(Debug)]
#[repr(transparent)]
pub(crate) struct MutPtr<'a>(NonNull<u8>, PhantomData<&'a mut u8>);
#[derive(Debug)]
#[repr(transparent)]
pub(crate) struct OwningPtr<'a>(NonNull<u8>, PhantomData<&'a mut u8>);

macro_rules! impl_ptr {
    ($ptr:ident) => {
        impl<'a> From<$ptr<'a>> for NonNull<u8> {
            fn from(ptr: $ptr<'a>) -> Self {
                ptr.0
            }
        }

        impl $ptr<'_> {
            #[inline]
            pub unsafe fn byte_offset(self, count: isize) -> Self {
                Self(
                    unsafe { NonNull::new_unchecked(self.as_ptr().offset(count)) },
                    PhantomData,
                )
            }

            #[inline]
            pub unsafe fn byte_add(self, count: usize) -> Self {
                Self(
                    unsafe { NonNull::new_unchecked(self.as_ptr().add(count)) },
                    PhantomData,
                )
            }
        }

        impl Pointer for $ptr<'_> {
            #[inline]
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                Pointer::fmt(&self.0, f)
            }
        }
    };
}

impl_ptr!(Ptr);
impl_ptr!(MutPtr);
impl_ptr!(OwningPtr);

impl<'a> Ptr<'a> {
    #[inline]
    pub unsafe fn new(inner: NonNull<u8>) -> Self {
        Self(inner, PhantomData)
    }

    #[inline]
    pub unsafe fn deref<T>(self) -> &'a T {
        let ptr = self.as_ptr().cast::<T>();
        unsafe { &*ptr }
    }

    #[inline]
    pub fn as_ptr(self) -> *mut u8 {
        self.0.as_ptr()
    }
}

impl<'a, T: ?Sized> From<&'a T> for Ptr<'a> {
    #[inline]
    fn from(value: &'a T) -> Self {
        unsafe { Self::new(NonNull::from(value).cast()) }
    }
}

impl<'a> MutPtr<'a> {
    #[inline]
    pub unsafe fn new(inner: NonNull<u8>) -> Self {
        Self(inner, PhantomData)
    }

    #[inline]
    pub unsafe fn promote(self) -> OwningPtr<'a> {
        OwningPtr(self.0, PhantomData)
    }

    #[inline]
    pub unsafe fn deref_mut<T>(self) -> &'a mut T {
        let ptr = self.as_ptr().cast::<T>();
        unsafe { &mut *ptr }
    }

    #[inline]
    pub fn as_ptr(self) -> *mut u8 {
        self.0.as_ptr()
    }

    #[inline]
    pub fn as_ref(&self) -> Ptr<'_> {
        unsafe { Ptr::new(self.0) }
    }
}

impl<'a, T: ?Sized> From<&'a mut T> for MutPtr<'a> {
    #[inline]
    fn from(value: &'a mut T) -> Self {
        unsafe { Self::new(NonNull::from(value).cast()) }
    }
}

impl<'a> OwningPtr<'a> {
    #[inline]
    pub unsafe fn new(inner: NonNull<u8>) -> Self {
        Self(inner, PhantomData)
    }

    #[inline]
    pub unsafe fn read<T>(self) -> T {
        let ptr = self.as_ptr().cast::<T>();
        unsafe { ptr.read() }
    }

    #[inline]
    pub unsafe fn drop_as<T>(self) {
        let ptr = self.as_ptr().cast::<T>();

        unsafe { ptr.drop_in_place() }
    }

    #[inline]
    pub fn as_ptr(&self) -> *mut u8 {
        self.0.as_ptr()
    }

    #[inline]
    pub fn as_ref(&self) -> Ptr<'_> {
        unsafe { Ptr::new(self.0) }
    }

    #[inline]
    pub fn as_mut(&mut self) -> MutPtr<'_> {
        unsafe { MutPtr::new(self.0) }
    }

    #[inline]
    pub fn make<T, F: FnOnce(OwningPtr<'_>) -> R, R>(value: T, f: F) -> R {
        let mut temp = ManuallyDrop::new(value);

        f(unsafe { MutPtr::from(&mut *temp).promote() })
    }
}

#[cfg(test)]
mod test {
    use crate::{
        component::{Component, ComponentInfo, Components},
        storage::OwningPtr,
    };

    use super::{Column, Tables};

    struct MyComponent {
        position: (f32, f32, f32),
    }
    impl Component for MyComponent {}

    #[test]
    fn create_column() {
        let mut components = Components::new();
        let component_id = components.register_component::<MyComponent>();
        let component_info = components.get_info(&component_id).unwrap();

        let mut column = Column::with_capacity(component_info, 5);
        assert_eq!(column.item_layout, component_info.layout);

        let mut c1 = MyComponent {
            position: (1.0, 2.0, 3.0),
        };
        let mut c2 = MyComponent {
            position: (3.0, 2.0, 1.0),
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

        column.drop(2);
    }

    #[test]
    fn illegal_access() {
        let mut components = Components::new();
        let component_id = components.register_component::<u32>();
        let component_info = components.get_info(&component_id).unwrap();

        let my_comp: u32 = 5;

        let mut column = Column::with_capacity(component_info, 1);
        OwningPtr::make(my_comp, |ptr| unsafe { column.initialize(100, ptr) });

        column.drop(1);
    }

    #[test]
    fn tables() {
        let mut tables = Tables::default();
        let mut components = Components::new();

        let comp_id1 = components.register_component::<MyComponent>();
        let comp_id2 = components.register_component::<u32>();
        let comp_id3 = components.register_component::<u8>();

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
    fn get_component() {
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

        column.drop(1);
    }
}
