use std::{
    alloc::{handle_alloc_error, Layout},
    collections::HashMap,
    ptr::NonNull,
};

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

    pub unsafe fn initialize_unchecked(&mut self, index: usize, value: *mut u8) {
        let size = self.layout.size();
        let dst = self.data.byte_add(index * size);
        std::ptr::copy_nonoverlapping(value, dst.as_ptr(), size);
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

#[cfg(test)]
mod tests {
    use crate::component::{Component, ComponentInfo, Components};

    use super::Column;

    struct MyComponent {
        position: (f32, f32, f32),
        velocity: (f32, f32, f32),
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
            position: (0.0, 0.0, 0.0),
            velocity: (1.0, 0.0, 0.0),
        };

        let c1p: *mut MyComponent = &mut c1;
        let c1p: *mut u8 = c1p.cast();
        unsafe { column.initialize_unchecked(0, c1p) };
    }
}
