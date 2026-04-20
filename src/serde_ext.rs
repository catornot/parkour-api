use serde::{Serialize, Serializer};

pub fn serialize_option_flat<S, T>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize + Default,
{
    match value {
        Some(obj) => obj.serialize(serializer),
        None => T::default().serialize(serializer),
    }
}
