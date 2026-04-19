use core::fmt;
use std::{marker::PhantomData, mem::MaybeUninit};

use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{SeqAccess, Visitor},
};

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(from = "[f64; 3]", into = "[f64; 3]")]
pub struct Vector3 {
    x: f64,
    y: f64,
    z: f64,
}

impl From<[f64; 3]> for Vector3 {
    fn from(arr: [f64; 3]) -> Self {
        Vector3 {
            x: arr[0],
            y: arr[1],
            z: arr[2],
        }
    }
}

impl From<Vector3> for [f64; 3] {
    fn from(val: Vector3) -> Self {
        [val.x, val.y, val.z]
    }
}

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

pub fn serialize_iter<S, C, T>(value: &C, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    for<'a> &'a C: IntoIterator<Item = &'a T>,
    T: Into<Vector3> + Clone + Copy,
{
    let vec: Vec<Vector3> = value.into_iter().map(|v| (*v).into()).collect();
    vec.serialize(s)
}

pub fn deserialize_iter<'de, D, C, T>(d: D) -> Result<C, D::Error>
where
    D: Deserializer<'de>,
    C: FromIterator<T>,
    T: From<Vector3>,
{
    let vec = Vec::<Vector3>::deserialize(d)?;
    vec.into_iter().map(|s| Ok(s.into())).collect()
}

pub fn serialize_iter_arr<S, C, T>(value: &C, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    for<'a> &'a C: IntoIterator<Item = &'a [T; 2]>,
    T: Into<Vector3> + Clone + Copy,
{
    let vec: Vec<[Vector3; 2]> = value
        .into_iter()
        .map(|v| v.map(|elem| (elem).into()))
        .collect();
    vec.serialize(s)
}

pub fn deserialize_iter_arr<'de, D, C, T>(d: D) -> Result<C, D::Error>
where
    D: Deserializer<'de>,
    C: FromIterator<[T; 2]>,
    T: From<Vector3>,
{
    let vec = Vec::<[Vector3; 2]>::deserialize(d)?;
    vec.into_iter()
        .map(|s| Ok(s.map(|elem| elem.into())))
        .collect()
}

pub fn deserialize_arr_vector<'de, const N: usize, D>(
    deserialize: D,
) -> Result<[[f64; 3]; N], D::Error>
where
    D: Deserializer<'de>,
{
    struct TupleVisitor<const N: usize, TI> {
        marker: PhantomData<TI>,
    }

    impl<'de, const N: usize, TI> Visitor<'de> for TupleVisitor<N, TI>
    where
        TI: Deserialize<'de>,
    {
        type Value = [TI; N];

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_fmt(format_args!("an array of size {}", N))
        }

        #[inline]
        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut arr = std::array::from_fn::<_, N, _>(|_| MaybeUninit::<TI>::uninit());

            for (i, elm) in arr.iter_mut().enumerate() {
                elm.write(
                    seq.next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(i, &self))?,
                );
            }

            Ok(arr.map(|value| unsafe { value.assume_init() }))
        }
    }

    deserialize
        .deserialize_tuple(
            N,
            TupleVisitor::<N, Vector3> {
                marker: PhantomData,
            },
        )
        .map(|arr| arr.map(|vec| [vec.x, vec.y, vec.z]))
}

pub fn serialize_vector<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Into<Vector3> + Clone + Copy,
{
    (*value).into().serialize(serializer)
}

pub fn deserialize_vector<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: From<Vector3>,
{
    Vector3::deserialize(deserializer).map(|vec| T::from(vec))
}
