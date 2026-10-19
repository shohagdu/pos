use uuid::Uuid;

/// New primary key. UUIDv7 is time-ordered, so InnoDB inserts append to the
/// end of each tenant's key range (DB design §2.1).
pub fn new_id() -> Uuid {
    Uuid::now_v7()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_v7_and_increasing() {
        let a = new_id();
        let b = new_id();
        assert_eq!(a.get_version_num(), 7);
        assert!(b > a);
    }
}
