use core::{
    fmt::{self, Formatter, Pointer},
    marker::PhantomData,
    ptr::NonNull,
};

use std::mem::ManuallyDrop;

#[derive(Debug)]
#[repr(transparent)]
pub struct Ptr<'a>(NonNull<u8>, PhantomData<&'a u8>);
#[derive(Debug)]
#[repr(transparent)]
pub struct MutPtr<'a>(NonNull<u8>, PhantomData<&'a mut u8>);
#[derive(Debug)]
#[repr(transparent)]
pub struct OwningPtr<'a>(NonNull<u8>, PhantomData<&'a mut u8>);

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
