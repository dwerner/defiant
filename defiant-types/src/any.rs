use super::*;
use prost::Arena;

impl<'arena> Any<'arena> {
    /// Serialize the given message type `M` as [`Any`].
    pub fn from_msg<M>(msg: &M, arena: &'arena Arena) -> Result<Any<'arena>, EncodeError>
    where
        M: Name + Message<'arena>,
    {
        let type_url_string = M::type_url();
        let type_url = arena.alloc_str(&type_url_string);

        // Encode directly into arena (zero-copy!)
        let value = msg.arena_encode(arena);

        Ok(Any { type_url, value })
    }

    /// Decode the given message type `M` from [`Any`], validating that it has
    /// the expected type URL.
    pub fn to_msg<M>(&self, arena: &'arena Arena) -> Result<M, DecodeError>
    where
        M: Message<'arena> + Name + Sized,
    {
        let expected_type_url = M::type_url();

        if let (Some(expected), Some(actual)) = (
            TypeUrl::new(&expected_type_url),
            TypeUrl::new(&self.type_url),
        ) {
            if expected == actual {
                return M::decode(self.value, arena);
            }
        }

        let mut err = DecodeError::new(format!(
            "expected type URL: \"{}\" (got: \"{}\")",
            expected_type_url, &self.type_url
        ));
        err.push("unexpected type URL", "type_url");
        Err(err)
    }
}

impl<'arena> Name for Any<'arena> {
    const PACKAGE: &'static str = PACKAGE;
    const NAME: &'static str = "Any";

    fn type_url() -> String {
        type_url_for::<Self>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_any_serialization() {
        let arena = Arena::new();
        let message = Timestamp::date(2000, 1, 1).unwrap();
        let any = Any::from_msg(&message, &arena).unwrap();
        assert_eq!(
            any.type_url,
            "type.googleapis.com/google.protobuf.Timestamp"
        );

        let message2 = any.to_msg::<Timestamp>(&arena).unwrap();
        assert_eq!(message, message2);

        // Wrong type URL
        assert!(any.to_msg::<Duration>(&arena).is_err());
    }
}
