/// Front montant (Down) ou relâchement (Up) d'un usage consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Down(u16),
    Up(u16),
}

/// Octet 0 = report ID (ignoré), puis usages u16 little-endian ; les 0 sont ignorés.
pub fn parse_consumer_report(bytes: &[u8]) -> Vec<u16> {
    bytes
        .get(1..)
        .unwrap_or(&[])
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .filter(|&u| u != 0)
        .collect()
}

/// Suit les usages tenus d'un rapport à l'autre et en déduit les fronts.
#[derive(Debug, Default)]
pub struct ReportTracker {
    prev: Vec<u16>,
}

impl ReportTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Up d'abord (ordre du rapport précédent), puis Down (ordre du nouveau rapport).
    /// Un usage répété dans un même rapport ne compte qu'une fois.
    pub fn feed(&mut self, usages: &[u16]) -> Vec<Edge> {
        let mut new: Vec<u16> = Vec::with_capacity(usages.len());
        for &u in usages {
            if !new.contains(&u) {
                new.push(u);
            }
        }
        let mut edges: Vec<Edge> = self
            .prev
            .iter()
            .filter(|u| !new.contains(u))
            .map(|&u| Edge::Up(u))
            .collect();
        edges.extend(
            new.iter()
                .filter(|u| !self.prev.contains(u))
                .map(|&u| Edge::Down(u)),
        );
        self.prev = new;
        edges
    }

    /// Relâche tout ce qui est tenu (sans Down).
    pub fn clear(&mut self) -> Vec<Edge> {
        std::mem::take(&mut self.prev)
            .into_iter()
            .map(Edge::Up)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_skips_zero() {
        assert_eq!(
            parse_consumer_report(&[2, 0x00, 0x05, 0, 0, 0x01, 0x05]),
            vec![0x0500, 0x0501]
        );
    }

    #[test]
    fn two_keys_out_of_order() {
        let mut t = ReportTracker::new();
        assert_eq!(t.feed(&[0x0500]), vec![Edge::Down(0x0500)]);
        assert_eq!(t.feed(&[0x0500, 0x0501]), vec![Edge::Down(0x0501)]);
        assert_eq!(t.feed(&[0x0501]), vec![Edge::Up(0x0500)]);
        assert_eq!(t.feed(&[]), vec![Edge::Up(0x0501)]);
    }

    #[test]
    fn clear_releases_all() {
        let mut t = ReportTracker::new();
        t.feed(&[0x0500, 0x0501]);
        let mut e = t.clear();
        e.sort_by_key(|x| format!("{x:?}"));
        assert_eq!(e, vec![Edge::Up(0x0500), Edge::Up(0x0501)]);
        assert_eq!(t.feed(&[]), vec![]);
    }
}
