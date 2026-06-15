//! Strongly typed identifiers.
//!
//! Using distinct newtypes instead of bare `String`s makes it impossible to,
//! for example, pass a [`ServerId`] where an [`OperationId`] is expected. This
//! is part of the "make invalid states unrepresentable" principle.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
        )]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Wraps a raw string as this identifier.
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// Borrows the underlying string.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }
    };
}

string_id! {
    /// Canonical identifier of an operation verb, e.g. `"db.create"`.
    OperationId
}
string_id! {
    /// Identifier of a server within the steward state, e.g. `"srv-prod-1"`.
    ServerId
}
string_id! {
    /// Identifier of a project / application, e.g. `"app-web"`.
    ProjectId
}
string_id! {
    /// Identifier of an issued capability token.
    TokenId
}
string_id! {
    /// Identifier of an append-only audit event, e.g. `"evt_01J..."`.
    AuditId
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip_through_json_transparently() {
        let id = OperationId::new("db.create");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"db.create\"");
        let back: OperationId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn distinct_id_types_do_not_unify() {
        // This is a compile-time guarantee in real code; here we just assert the
        // string values are accessible and display correctly.
        let server = ServerId::from("srv-prod-1");
        let project = ProjectId::from("app-web");
        assert_eq!(server.to_string(), "srv-prod-1");
        assert_eq!(project.as_str(), "app-web");
    }
}
