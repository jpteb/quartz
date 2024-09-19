use std::{
    alloc::Layout,
    any::{type_name, TypeId},
    borrow::Cow,
    collections::HashMap,
};

pub trait Component: Send + Sync + 'static {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ComponentId(usize);

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
}

// impl ComponentInfo {
//     pub(crate) fn new<T: Component>() -> Self {
//         
//     }
// }

#[derive(Debug)]
pub(crate) struct Components {
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
            let info = ComponentInfo {
                id,
                name: Cow::Borrowed(type_name::<T>()),
                type_id,
                layout: Layout::new::<T>(),
            };
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

    #[inline]
    pub fn component_id<T: Component>(&self) -> Option<ComponentId> {
        self.indices.get(&TypeId::of::<T>()).copied()
    }

    pub fn components(&self) -> impl Iterator<Item = ComponentId> + use<'_> {
        self.components.iter().map(|info| info.id)
    }
}

#[cfg(test)]
mod tests {
    use std::any::type_name;

    use super::{Component, Components, ComponentId};

    impl Component for u8 {}
    impl Component for u32 {}

    struct MyComponent;
    impl Component for MyComponent {}

    #[test]
    fn test_registration() {
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
        assert_eq!(
            Some(ComponentId::new(2)),
            components.component_id::<u8>()
        );
    }
}
