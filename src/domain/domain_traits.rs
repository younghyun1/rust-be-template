pub trait IsSoftDeletable {
    fn is_soft_deleted(&self) -> bool;
}

pub fn purge_soft_deleted<T: IsSoftDeletable>(vec: Vec<T>) -> Vec<T> {
    vec.into_iter()
        .filter(|element| !element.is_soft_deleted())
        .collect()
}
