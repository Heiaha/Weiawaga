pub struct Statistics {
    pub leafs: u64,
    pub qleafs: u64,
    pub beta_cutoffs: u64,
    pub qbeta_cutoffs: u64,
    pub tt_hits: u64,
    pub nodes: u64,
    pub qnodes: u64,
}

impl Statistics {
    pub fn new() -> Self {
        Statistics {
            leafs: 0,
            qleafs: 0,
            beta_cutoffs: 0,
            qbeta_cutoffs: 0,
            tt_hits: 0,
            nodes: 0,
            qnodes: 0,
        }
    }

    pub fn total_nodes(&self) -> u64 {
        self.nodes + self.qnodes
    }
}
