use core::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default, Debug)]
pub struct Validity {
    pub overlaps: AtomicUsize,
    pub adjacents: AtomicUsize,
    pub out_of_orders: AtomicUsize,
}

impl Validity {
    #[inline(always)]
    pub fn is_valid(&self) -> bool {
        self.overlaps.load(Ordering::Relaxed) == 0 
        && self.adjacents.load(Ordering::Relaxed) == 0 
        && self.out_of_orders.load(Ordering::Relaxed) == 0
    }

    #[inline(always)]
    pub fn record_overlap(&self) {
        self.overlaps.fetch_add(1, Ordering::Release);
    }

    #[inline(always)]
    pub fn record_adjacent(&self) {
        self.adjacents.fetch_add(1, Ordering::Release);
    }

    #[inline(always)]
    pub fn record_out_of_order(&self) {
        self.out_of_orders.fetch_add(1, Ordering::Release);
    }
}

impl From<Validity> for bool {
    fn from(v: Validity) -> bool {
        v.is_valid()
    }
}