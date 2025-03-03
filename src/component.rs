use std::{alloc::Layout, any::TypeId, borrow::Cow, collections::HashMap, mem::needs_drop};

use crate::ptr::OwningPtr;

pub trait Component: Send + Sync + 'static {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentId(usize);

impl ComponentId {
    pub(crate) fn new(id: usize) -> Self {
        Self(id)
    }

    pub(crate) fn index(&self) -> usize {
        self.0
    }
}

#[derive(Debug)]
pub(crate) struct ComponentInfo {
    id: ComponentId,
    name: Cow<'static, str>,
    type_id: TypeId,
    pub(crate) layout: Layout,
    pub(crate) drop: Option<for<'a> unsafe fn(OwningPtr<'a>)>,
}

impl ComponentInfo {
    pub(crate) fn new<T: Component>(id: ComponentId) -> Self {
        Self {
            id,
            name: Cow::Borrowed(std::any::type_name::<T>()),
            type_id: TypeId::of::<T>(),
            layout: Layout::new::<T>(),
            drop: needs_drop::<T>().then_some(Self::drop_ptr::<T> as _),
        }
    }

    unsafe fn drop_ptr<T>(x: OwningPtr<'_>) {
        x.drop_as::<T>()
    }
}

#[derive(Debug)]
pub struct Components {
    components: Vec<ComponentInfo>,
    indices: HashMap<TypeId, ComponentId>,
}

impl Components {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            indices: HashMap::new(),
        }
    }

    pub fn register_component<T: Component>(&mut self) -> ComponentId {
        let type_id = TypeId::of::<T>();
        *self.indices.entry(type_id).or_insert_with(|| {
            let id = ComponentId::new(self.components.len());
            let info = ComponentInfo::new::<T>(id);
            self.components.push(info);
            id
        })
    }

    pub(crate) fn get_info(&self, id: &ComponentId) -> Option<&ComponentInfo> {
        self.components.get(id.index())
    }

    pub fn get_id(&self, type_id: TypeId) -> Option<ComponentId> {
        self.indices.get(&type_id).copied()
    }

    pub fn component_id<T: Component>(&self) -> Option<ComponentId> {
        self.indices.get(&TypeId::of::<T>()).copied()
    }

    pub fn components(&self) -> impl Iterator<Item = ComponentId> + use<'_> {
        self.components.iter().map(|info| info.id)
    }

    pub fn len(&self) -> usize {
        self.components.len()
    }
}

pub trait Bundle {
    fn get_components(self, func: &mut impl FnMut(OwningPtr<'_>));
    fn component_ids(components: &mut Components, func: &mut impl FnMut(ComponentId));
    fn get(self, components: &Components, func: &mut impl FnMut(ComponentId, OwningPtr<'_>));
}

impl<C: Component> Bundle for C {
    fn get_components(self, func: &mut impl FnMut(OwningPtr<'_>)) {
        OwningPtr::make(self, |ptr| func(ptr));
    }

    fn component_ids(components: &mut Components, func: &mut impl FnMut(ComponentId)) {
        func(components.register_component::<C>());
    }

    fn get(self, components: &Components, func: &mut impl FnMut(ComponentId, OwningPtr<'_>)) {
        OwningPtr::make(self, |ptr| {
            func(components.component_id::<C>().unwrap(), ptr)
        });
    }
}

impl<C0: Component, C1: Component> Bundle for (C0, C1) {
    fn get_components(self, func: &mut impl FnMut(OwningPtr<'_>)) {
        OwningPtr::make(self.0, |ptr| func(ptr));
        OwningPtr::make(self.1, |ptr| func(ptr));
    }

    fn component_ids(components: &mut Components, func: &mut impl FnMut(ComponentId)) {
        func(components.register_component::<C0>());
        func(components.register_component::<C1>());
    }

    fn get(self, components: &Components, func: &mut impl FnMut(ComponentId, OwningPtr<'_>)) {
        OwningPtr::make(self.0, |ptr| {
            func(components.component_id::<C0>().unwrap(), ptr)
        });
        OwningPtr::make(self.1, |ptr| {
            func(components.component_id::<C1>().unwrap(), ptr)
        });
    }
}

#[cfg(test)]
mod tests {
    use std::any::type_name;

    use super::{Component, ComponentId, Components};

    impl Component for u8 {}
    impl Component for u32 {}

    struct MyComponent;
    impl Component for MyComponent {}

    #[test]
    fn component_registration() {
        let mut components = Components::new();

        let id = components.register_component::<u32>();
        let val: u32 = 0;
        assert_eq!(id, ComponentId::new(0));
        assert_eq!(Some(ComponentId::new(0)), components.component_id::<u32>());
        assert_eq!("u32", components.components[id.index()].name);

        let id = components.register_component::<MyComponent>();
        assert_eq!(id, ComponentId::new(1));
        assert_eq!(
            Some(ComponentId::new(1)),
            components.component_id::<MyComponent>()
        );
        assert_eq!(
            type_name::<MyComponent>(),
            components.components[id.index()].name
        );

        let id = components.register_component::<MyComponent>();
        assert_eq!(id, ComponentId::new(1));
        assert_eq!(
            Some(ComponentId::new(1)),
            components.component_id::<MyComponent>()
        );

        assert_eq!(None, components.component_id::<u8>());

        let id = components.register_component::<u8>();
        assert_eq!(id, ComponentId::new(2));
        assert_eq!(Some(ComponentId::new(2)), components.component_id::<u8>());
    }
}
