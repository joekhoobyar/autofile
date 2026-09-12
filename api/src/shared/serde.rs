// Deserialize an Option<Option<T>> where the outer Option indicates presence of the field,
// and the inner Option is the actual value (Some or None).
pub fn de_present_option<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::de::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    use serde::Deserialize;
    // If the field is present:
    // - null -> Option<T>::None
    // - value -> Option<T>::Some(value)
    // Then we wrap it in Some(...) to record presence.
    Ok(Some(<Option<T>>::deserialize(d)?))
}
