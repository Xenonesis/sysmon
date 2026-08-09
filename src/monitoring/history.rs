use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct BoundedHistory<T> {
    capacity: usize,
    values: VecDeque<T>,
}

impl<T> BoundedHistory<T> {
    pub fn new(capacity: usize) -> Self {
        Self { capacity, values: VecDeque::with_capacity(capacity) }
    }

    pub fn push(&mut self, value: T) {
        if self.capacity == 0 {
            return;
        }
        if self.values.len() == self.capacity {
            self.values.pop_front();
        }
        self.values.push_back(value);
    }

    pub fn is_empty(&self) -> bool { self.values.is_empty() }
    pub fn clear(&mut self) { self.values.clear() }
    pub fn iter(&self) -> impl Iterator<Item = &T> { self.values.iter() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evicts_oldest_value() {
        let mut history = BoundedHistory::new(2);
        history.push(1);
        history.push(2);
        history.push(3);
        assert_eq!(history.iter().copied().collect::<Vec<_>>(), vec![2, 3]);
    }

    #[test]
    fn zero_capacity_discards_values() {
        let mut history = BoundedHistory::new(0);
        history.push(1);
        assert!(history.is_empty());
    }
}
