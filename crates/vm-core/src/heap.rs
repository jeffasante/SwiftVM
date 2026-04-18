use crate::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SwiftObject {
    pub type_name: String,
    pub retain_count: u32,
    pub fields: HashMap<String, Value>,
}

#[derive(Debug, Default, Clone)]
pub struct ArcHeap {
    pub objects: HashMap<u64, SwiftObject>,
    next_id: u64,
}

impl ArcHeap {
    pub fn alloc(&mut self, type_name: impl Into<String>) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.objects.insert(
            id,
            SwiftObject {
                type_name: type_name.into(),
                retain_count: 1,
                fields: HashMap::new(),
            },
        );
        id
    }

    pub fn retain(&mut self, id: u64) -> bool {
        if let Some(obj) = self.objects.get_mut(&id) {
            obj.retain_count = obj.retain_count.saturating_add(1);
            true
        } else {
            false
        }
    }

    pub fn release(&mut self, id: u64) -> bool {
        if let Some(obj) = self.objects.get_mut(&id) {
            if obj.retain_count > 1 {
                obj.retain_count -= 1;
                return true;
            }
        }

        self.objects.remove(&id).is_some()
    }

    pub fn set_prop(&mut self, id: u64, name: impl Into<String>, value: Value) -> bool {
        if let Some(obj) = self.objects.get_mut(&id) {
            obj.fields.insert(name.into(), value);
            true
        } else {
            false
        }
    }

    pub fn get_prop(&self, id: u64, name: &str) -> Option<Value> {
        self.objects
            .get(&id)
            .and_then(|obj| obj.fields.get(name))
            .cloned()
    }

    pub fn has_object(&self, id: u64) -> bool {
        self.objects.contains_key(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_retain_release_lifecycle() {
        let mut heap = ArcHeap::default();
        let id = heap.alloc("User");
        assert!(heap.has_object(id));
        assert!(heap.retain(id));
        assert_eq!(heap.objects.get(&id).unwrap().retain_count, 2);
        assert!(heap.release(id));
        assert!(heap.has_object(id));
        assert!(heap.release(id));
        assert!(!heap.has_object(id));
    }
}
