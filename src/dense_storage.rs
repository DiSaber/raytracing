/// A generational index storage container
#[derive(Debug, Clone)]
pub struct DenseStorage<T> {
    // (generation, value)
    storage: Vec<(u32, Option<T>)>,
    recycled_indices: Vec<usize>,
}

/// Stores the dense storage index and generational index
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DenseStorageIndex(pub usize, pub u32);

impl<T> DenseStorage<T> {
    pub fn new() -> Self {
        Self {
            storage: Vec::new(),
            recycled_indices: Vec::new(),
        }
    }

    pub fn push(&mut self, value: T) -> DenseStorageIndex {
        if let Some(i) = self.recycled_indices.pop() {
            let (generation, v) = &mut self.storage[i];
            *v = Some(value);
            DenseStorageIndex(i, *generation)
        } else {
            self.storage.push((0, Some(value)));
            DenseStorageIndex(self.storage.len() - 1, 0)
        }
    }

    #[allow(unused)]
    pub fn get(&self, index: DenseStorageIndex) -> Option<&T> {
        self.storage
            .get(index.0)
            .filter(|(generation, _)| *generation == index.1)
            .and_then(|(_, value)| value.as_ref())
    }

    #[allow(unused)]
    pub fn remove(&mut self, index: DenseStorageIndex) -> Option<T> {
        if let Some((generation, value)) = self.storage.get_mut(index.0) {
            if value.is_some() {
                *generation += 1;
                self.recycled_indices.push(index.0);
            }

            value.take()
        } else {
            None
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, (u32, Option<T>)> {
        self.storage.iter()
    }
}

impl<T> Default for DenseStorage<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> IntoIterator for DenseStorage<T> {
    type Item = (u32, Option<T>);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.storage.into_iter()
    }
}
