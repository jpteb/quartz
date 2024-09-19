use core::{
    alloc::Layout,
    fmt::{self, Formatter, Pointer},
    marker::PhantomData,
    ptr::NonNull,
};

use std::{alloc::handle_alloc_error, collections::HashMap, mem::{self, ManuallyDrop}};

use crate::component::{ComponentId, ComponentInfo};

#[derive(Debug)]
pub(crate) struct TableId(usize);

#[derive(Debug)]
pub(crate) struct Table {
    id: TableId,
    columns: HashMap<ComponentId, Column>,
}

#[derive(Debug)]
pub(crate) struct Column {
    layout: Layout,
    data: NonNull<u8>,
    capacity: usize,
}

impl Column {
    pub fn new_with_capacity(component_info: &ComponentInfo, capacity: usize) -> Self {
        let (array_layout, _off) = component_info
            .layout
            .repeat(capacity)
            .expect("Array layout creation should be successful!");

        let data = unsafe { std::alloc::alloc(array_layout) };
        let data = NonNull::new(data).unwrap_or_else(|| handle_alloc_error(array_layout));

        Self {
            layout: component_info.layout,
            data,
            capacity,
        }
    }

    pub unsafe fn initialize_unchecked(&mut self, index: usize, value: OwningPtr) {
        let size = self.layout.size();
        let dst = self.data.byte_add(index * size);
        std::ptr::copy_nonoverlapping(value.as_ptr(), dst.as_ptr(), size);
    }
}

impl Drop for Column {
    fn drop(&mut self) {
        unsafe {
            std::alloc::dealloc(
                self.data.as_ptr(),
                self.layout
                    .repeat(self.capacity)
                    .expect("Array layout creation should be successful")
                    .0,
            )
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

    use super::Column;

    struct MyComponent {
        position: (f32, f32, f32),
    }
    impl Component for MyComponent {}

    #[test]
    fn create_column() {
        let mut components = Components::new();
        let component_id = components.register_component::<MyComponent>();
        let component_info = components.get_info(component_id).unwrap();

        let mut column = Column::new_with_capacity(component_info, 5);
        assert_eq!(column.layout, component_info.layout);

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
    }
}
