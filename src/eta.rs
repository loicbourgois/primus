use std::time::Instant;
pub struct ETA {
    pub start: Instant,
    pub l: usize,
    pub i: usize,
}

impl ETA {
    pub fn new(l: usize) -> ETA {
        ETA {
            start: Instant::now(),
            l: l,
            i: 0,
        }
    }
    pub fn tick(&mut self) {
        self.i += 1;
    }
}

impl std::fmt::Display for ETA {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let percent_done = (self.i as f64) / (self.l as f64);
        let percent_remaining = 1.0 - percent_done;
        let elapsed_ms = self.start.elapsed().as_secs() as f64;
        let total = elapsed_ms / percent_done;
        let remaining = total - elapsed_ms;
        let percent_done_2 = percent_done * 100.0;
        let i = self.i;
        let l = self.l;
        write!(
            f,
            "{i}/{l} | {remaining:.0}s | {percent_done_2:.1}% | {elapsed_ms:.0}s"
        )
    }
}
