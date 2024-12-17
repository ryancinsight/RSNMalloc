use core::sync::atomic::{AtomicUsize, Ordering};

#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Relation {
    Before = 0,
    AdjacentBefore = 1,
    Overlapping = 2,
    AdjacentAfter = 3,
    After = 4,
}

pub struct AtomicRelation {
    value: AtomicUsize,
}

impl AtomicRelation {
    pub fn new(relation: Relation) -> Self {
        Self {
            value: AtomicUsize::new(relation as usize),
        }
    }

    pub fn load(&self, ordering: Ordering) -> Relation {
        match self.value.load(ordering) {
            0 => Relation::Before,
            1 => Relation::AdjacentBefore,
            2 => Relation::Overlapping,
            3 => Relation::AdjacentAfter,
            4 => Relation::After,
            _ => panic!("Invalid value for Relation"),
        }
    }

    pub fn store(&self, relation: Relation, ordering: Ordering) {
        self.value.store(relation as usize, ordering);
    }

    pub fn compare_exchange(
        &self,
        current: Relation,
        new: Relation,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Relation, Relation> {
        match self
            .value
            .compare_exchange(current as usize, new as usize, success, failure)
        {
            Ok(_) => Ok(new),
            Err(old) => match old {
                0 => Err(Relation::Before),
                1 => Err(Relation::AdjacentBefore),
                2 => Err(Relation::Overlapping),
                3 => Err(Relation::AdjacentAfter),
                4 => Err(Relation::After),
                _ => panic!("Invalid value for Relation"),
            },
        }
    }
}
