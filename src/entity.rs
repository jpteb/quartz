use crate::{
    archetype::{Archetype, ArchetypeId},
    storage::{Table, TableId, TableRow},
};

type Generation = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity {
    index: u32,
    generation: Generation,
}

impl Entity {
    fn from(index: u32, generation: Generation) -> Self {
        Self { index, generation }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityLocation {
    archetype_id: ArchetypeId,
    table_id: TableId,
    table_row: TableRow,
}

#[derive(Debug)]
enum Entry {
    Free { next_free: usize },
    Occupied { loc: EntityLocation },
}

#[derive(Debug)]
struct EntityEntry {
    entry: Entry,
    generation: Generation,
}

/// The struct handling all [`Entity`]s used in the ECS
#[derive(Debug)]
pub struct Entities {
    entities: Vec<EntityEntry>,
    free_head: usize,
    len: usize,
}

impl Entities {
    fn new() -> Self {
        Self {
            entities: Vec::new(),
            free_head: 0,
            len: 0,
        }
    }

    /// Allocate a new entity.
    ///
    /// The closure `f` needs to use the newly created [`Entity`] and use it for further
    /// allocations in [`Archetype`]s and [`Table`]s. After the allocation is used, the caller
    /// needs to provide the [`EntityLocation`] of the entity.
    pub(crate) fn alloc(
        &mut self,
        f: impl FnOnce(Entity) -> Result<EntityLocation, ()>,
    ) -> Result<Entity, ()> {
        if let Some(EntityEntry { entry, generation }) = self.entities.get_mut(self.free_head) {
            if let Entry::Free { next_free } = entry {
                let entity = Entity::from(self.free_head as u32, *generation);
                if let Ok(loc) = f(entity) {
                    self.free_head = *next_free;
                    *entry = Entry::Occupied { loc };
                    self.len += 1;
                    return Ok(entity);
                }
            } else {
                panic!("Entities free list is corrupt, failed to allocate entity!");
            }
        } else {
            let entity = Entity::from(self.entities.len() as u32, 0);
            if let Ok(loc) = f(entity) {
                self.entities.push(EntityEntry {
                    generation: 0,
                    entry: Entry::Occupied { loc },
                });
                self.free_head = self.entities.len();
                self.len += 1;
                return Ok(entity);
            }
        }

        Err(())
    }

    pub fn get(&self, entity: Entity) -> Option<&EntityLocation> {
        if let Some(EntityEntry { entry, generation }) = self.entities.get(entity.index as usize) {
            if let Entry::Occupied { loc } = entry {
                if *generation == entity.generation {
                    return Some(loc);
                }
            }
        }

        None
    }

    pub fn free(&mut self, entity: Entity) {
        if let Some(EntityEntry { entry, generation }) =
            self.entities.get_mut(entity.index as usize)
        {
            if *generation == entity.generation {
                if let Entry::Occupied { .. } = entry {
                    *generation += 1;
                    *entry = Entry::Free {
                        next_free: self.free_head,
                    };
                    self.free_head = entity.index as usize;
                    self.len -= 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{archetype::ArchetypeId, storage::TableId, storage::TableRow};

    use super::{Entities, EntityLocation};

    #[test]
    fn alloc_entity() {
        let mut entities = Entities::new();
        let entity = entities
            .alloc(|entity| {
                Ok(EntityLocation {
                    archetype_id: ArchetypeId(0),
                    table_id: TableId(0),
                    table_row: TableRow(0),
                })
            })
            .unwrap();

        assert_eq!(entities.len, 1);
        assert_eq!(entity.index, 0);
        assert_eq!(entity.generation, 0);

        let entity = entities
            .alloc(|entity| {
                Ok(EntityLocation {
                    archetype_id: ArchetypeId(1),
                    table_id: TableId(1),
                    table_row: TableRow(0),
                })
            })
            .unwrap();

        assert_eq!(entities.len, 2);
        assert_eq!(entity.index, 1);
        assert_eq!(entity.generation, 0);

        entities.free(entity);

        assert_eq!(entities.len, 1);
        assert_eq!(entities.get(entity), None);

        let double_entity = entities
            .alloc(|entity| {
                Ok(EntityLocation {
                    archetype_id: ArchetypeId(1),
                    table_id: TableId(1),
                    table_row: TableRow(0),
                })
            })
            .unwrap();

        assert_eq!(entities.len, 2);
        assert_eq!(double_entity.index, 1);
        assert_eq!(double_entity.generation, 1);
        assert_eq!(entities.get(entity), None);
        assert_eq!(
            entities.get(double_entity),
            Some(&EntityLocation {
                archetype_id: ArchetypeId(1),
                table_id: TableId(1),
                table_row: TableRow(0),
            })
        );
    }

    #[test]
    fn double_free() {
        let mut entities = Entities::new();
        let entity1 = entities
            .alloc(|entity| {
                Ok(EntityLocation {
                    archetype_id: ArchetypeId(0),
                    table_id: TableId(0),
                    table_row: TableRow(0),
                })
            })
            .unwrap();

        entities.free(entity1);

        let entity2 = entities
            .alloc(|entity| {
                Ok(EntityLocation {
                    archetype_id: ArchetypeId(0),
                    table_id: TableId(0),
                    table_row: TableRow(1),
                })
            })
            .unwrap();

        assert_eq!(entity1.index, entity2.index);
        assert_ne!(entity1.generation, entity2.generation);

        entities.free(entity1);

        assert_eq!(entities.len, 1);
        assert_eq!(
            entities.get(entity2),
            Some(&EntityLocation {
                archetype_id: ArchetypeId(0),
                table_id: TableId(0),
                table_row: TableRow(1)
            })
        );

        entities.free(entity2);
        assert_eq!(entities.len, 0);
    }
}
