//! [`Timestamp`]: an RFC 3339 instant that keeps the offset it was written with.

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

use jiff::fmt::temporal::{DateTimeParser, PiecesOffset};
use jiff::tz::{Offset, TimeZone};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A TM Forum timestamp: an RFC 3339 instant that keeps the offset it arrived
/// with.
///
/// TMF payloads carry timestamps as RFC 3339 strings, and TM Forum's own
/// examples use offsets other than `Z` — `2020-09-23T16:42:23-04:00` appears in
/// the TMF620 v5 examples. Normalising those to UTC would re-emit them as
/// `2020-09-23T20:42:23Z`: the same instant, a different document, and a
/// spurious diff in any middleware that compares payloads. So a `Timestamp`
/// keeps the offset — and keeps `Z` as `Z`, which RFC 9557 gives a meaning of
/// its own ("UTC known, local offset unknown").
///
/// ```
/// use rutmf::core::Timestamp;
///
/// let t: Timestamp = "2020-09-23T16:42:23-04:00".parse().unwrap();
/// assert_eq!(t.to_string(), "2020-09-23T16:42:23-04:00");
///
/// let z: Timestamp = "2020-09-23T20:42:23Z".parse().unwrap();
/// assert_eq!(z.to_string(), "2020-09-23T20:42:23Z");
/// assert_eq!(t, z, "the same instant, so equal");
/// ```
///
/// # Strictly RFC 3339
///
/// Only an RFC 3339 `date-time` parses: a date, a time with seconds, and an
/// offset. A date without a time, a time without an offset, a basic-format or
/// comma-decimal ISO 8601 spelling, and an impossible date are all refused —
/// accepting them would mean a billing date that silently means something else.
///
/// ```
/// use rutmf::core::Timestamp;
///
/// for bad in ["2024-03-10", "2024-03-10T01:02:03", "20240310T010203Z",
///             "2024-03-10T01:02Z", "2024-03-10T01:02:03,5Z", "2020-01-00T00:00:00Z"] {
///     assert!(bad.parse::<Timestamp>().is_err(), "{bad}");
/// }
/// ```
///
/// A leap second (`23:59:60`) is accepted and read as `23:59:59`; the
/// underlying library does not model leap seconds. Fractional seconds are
/// written in their shortest form, so `.500` comes back as `.5` — the same
/// instant.
///
/// # Building one
///
/// Every builder setter takes `impl Into<_>`, so the current time, a
/// [`jiff`](crate::jiff) instant or a zoned date-time all go straight in:
///
/// ```
/// use rutmf::core::Timestamp;
/// use rutmf::product::ProductOfferingCreate;
///
/// let body = ProductOfferingCreate::builder()
///     .name("Business Internet")
///     .lifecycle_status("Active")
///     .last_update(Timestamp::now())
///     .build();
/// # let _ = body;
/// ```
///
/// For arithmetic and calendar work, take the [`instant`](Self::instant) or
/// the [`zoned`](Self::to_zoned_datetime) view and use `jiff`, re-exported as
/// [`rutmf::jiff`](crate::jiff) so the versions always match:
///
/// ```
/// use rutmf::core::Timestamp;
/// use rutmf::jiff::ToSpan;
///
/// let issued: Timestamp = "2026-03-01T09:00:00+01:00".parse().unwrap();
/// let due = issued.to_zoned_datetime().checked_add(14.days()).unwrap();
/// assert_eq!(Timestamp::from(due).to_string(), "2026-03-15T09:00:00+01:00");
/// ```
///
/// Equality, ordering and hashing compare the instant alone, as RFC 3339
/// comparison does: two spellings of one moment are equal.
#[derive(Clone, Copy)]
pub struct Timestamp {
    instant: jiff::Timestamp,
    /// `None` when the value was written with `Z`.
    offset: Option<Offset>,
}

impl Timestamp {
    /// The current time, written with `Z`.
    ///
    /// Needs a system clock: on `wasm32-unknown-unknown` without one, build
    /// timestamps from an instant you have instead.
    #[must_use]
    pub fn now() -> Self {
        Self::from_instant(jiff::Timestamp::now())
    }

    /// `instant`, written with `Z`.
    #[must_use]
    pub const fn from_instant(instant: jiff::Timestamp) -> Self {
        Self {
            instant,
            offset: None,
        }
    }

    /// The same instant, written with `offset` instead.
    #[must_use]
    pub const fn with_offset(self, offset: Offset) -> Self {
        Self {
            instant: self.instant,
            offset: Some(offset),
        }
    }

    /// The instant, for comparison and arithmetic.
    #[must_use]
    pub const fn instant(&self) -> jiff::Timestamp {
        self.instant
    }

    /// The offset the timestamp is written with; UTC for `Z`.
    #[must_use]
    pub fn offset(&self) -> Offset {
        self.offset.unwrap_or(Offset::UTC)
    }

    /// Whether the timestamp is written with `Z` rather than a numeric offset.
    #[must_use]
    pub const fn is_zulu(&self) -> bool {
        self.offset.is_none()
    }

    /// The timestamp as a zoned date-time in its own fixed offset, for
    /// calendar arithmetic.
    #[must_use]
    pub fn to_zoned_datetime(&self) -> jiff::ZonedDateTime {
        self.instant
            .to_zoned_datetime(TimeZone::fixed(self.offset()))
    }
}

/// Why a string is not a [`Timestamp`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimestampError {
    reason: String,
}

impl fmt::Display for TimestampError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not an RFC 3339 date-time: {}", self.reason)
    }
}

impl std::error::Error for TimestampError {}

impl TimestampError {
    fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

/// Whether `s` has the RFC 3339 `date-time` shape:
/// `YYYY-MM-DD[Tt ]HH:MM:SS[.f+](Z|z|±HH:MM)`.
///
/// `jiff` parses a superset (ISO 8601 basic format, minutes without seconds, a
/// decimal comma, six-digit years), so the shape is checked first. Value ranges
/// are left to the parser, which refuses day zero and the like.
fn has_rfc3339_shape(s: &[u8]) -> bool {
    fn digits(s: &[u8], at: usize, n: usize) -> bool {
        s.get(at..at + n)
            .is_some_and(|d| d.iter().all(u8::is_ascii_digit))
    }
    let fixed = digits(s, 0, 4)
        && s.get(4) == Some(&b'-')
        && digits(s, 5, 2)
        && s.get(7) == Some(&b'-')
        && digits(s, 8, 2)
        && matches!(s.get(10), Some(b'T' | b't' | b' '))
        && digits(s, 11, 2)
        && s.get(13) == Some(&b':')
        && digits(s, 14, 2)
        && s.get(16) == Some(&b':')
        && digits(s, 17, 2);
    if !fixed {
        return false;
    }
    let mut rest = &s[19..];
    if let Some(fraction) = rest.strip_prefix(b".") {
        let n = fraction.iter().take_while(|b| b.is_ascii_digit()).count();
        if n == 0 {
            return false;
        }
        rest = &fraction[n..];
    }
    match rest {
        b"Z" | b"z" => true,
        [b'+' | b'-', ..] => {
            rest.len() == 6 && digits(rest, 1, 2) && rest[3] == b':' && digits(rest, 4, 2)
        }
        _ => false,
    }
}

impl FromStr for Timestamp {
    type Err = TimestampError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        static PARSER: DateTimeParser = DateTimeParser::new();

        if !has_rfc3339_shape(s.as_bytes()) {
            return Err(TimestampError::new(format!(
                "{s:?} is not YYYY-MM-DDTHH:MM:SS[.f]±HH:MM or Z"
            )));
        }
        let pieces = PARSER
            .parse_pieces(s)
            .map_err(|e| TimestampError::new(e.to_string()))?;
        let time = pieces
            .time()
            .ok_or_else(|| TimestampError::new("no time of day"))?;
        let offset = match pieces.offset() {
            Some(PiecesOffset::Zulu) => None,
            Some(PiecesOffset::Numeric(numeric)) => Some(numeric.offset()),
            _ => return Err(TimestampError::new("no UTC offset")),
        };
        let instant = offset
            .unwrap_or(Offset::UTC)
            .to_timestamp(pieces.date().to_datetime(time))
            .map_err(|e| TimestampError::new(e.to_string()))?;
        Ok(Self { instant, offset })
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.offset {
            None => fmt::Display::fmt(&self.instant, f),
            Some(offset) => fmt::Display::fmt(&self.instant.display_with_offset(offset), f),
        }
    }
}

impl fmt::Debug for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Timestamp({self})")
    }
}

impl PartialEq for Timestamp {
    fn eq(&self, other: &Self) -> bool {
        self.instant == other.instant
    }
}

impl Eq for Timestamp {}

impl PartialOrd for Timestamp {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Timestamp {
    fn cmp(&self, other: &Self) -> Ordering {
        self.instant.cmp(&other.instant)
    }
}

impl Hash for Timestamp {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.instant.hash(state);
    }
}

impl From<jiff::Timestamp> for Timestamp {
    fn from(instant: jiff::Timestamp) -> Self {
        Self::from_instant(instant)
    }
}

/// Keeps the zoned value's current offset; the time-zone rules are not part of
/// an RFC 3339 timestamp and are dropped.
impl From<jiff::ZonedDateTime> for Timestamp {
    fn from(zoned: jiff::ZonedDateTime) -> Self {
        Self::from_instant(zoned.timestamp()).with_offset(zoned.offset())
    }
}

impl From<Timestamp> for jiff::Timestamp {
    fn from(timestamp: Timestamp) -> Self {
        timestamp.instant
    }
}

impl Serialize for Timestamp {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = <std::borrow::Cow<'de, str>>::deserialize(deserializer)?;
        raw.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "schemars")]
impl schemars::JsonSchema for Timestamp {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Timestamp".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "string", "format": "date-time" })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(s: &str) -> String {
        s.parse::<Timestamp>()
            .unwrap_or_else(|e| panic!("{s}: {e}"))
            .to_string()
    }

    /// jiff-time/SR-003: the offset is kept, and `Z` stays `Z`.
    #[test]
    fn the_offset_it_was_written_with_is_the_one_it_is_written_back_with() {
        for s in [
            "2024-03-10T01:02:03+02:00",
            "2024-03-10T01:02:03-05:30",
            "2024-03-10T01:02:03+00:00",
            "2020-01-22T12:00:00Z",
            "2024-03-10T01:02:03.5Z",
            "2024-03-10T01:02:03.123456789+01:00",
        ] {
            assert_eq!(round_trip(s), s);
        }
        // Re-spellings that keep the instant and the offset.
        assert_eq!(
            round_trip("2020-01-22T12:00:00.000Z"),
            "2020-01-22T12:00:00Z"
        );
        assert_eq!(round_trip("2024-03-10t01:02:03z"), "2024-03-10T01:02:03Z");
        assert_eq!(
            round_trip("2024-03-10 01:02:03.500+02:00"),
            "2024-03-10T01:02:03.5+02:00"
        );
        assert_eq!(round_trip("2016-12-31T23:59:60Z"), "2016-12-31T23:59:59Z");
    }

    /// jiff-time/SR-002: nothing but an RFC 3339 date-time parses.
    #[test]
    fn anything_but_an_rfc_3339_date_time_is_refused() {
        for bad in [
            "",
            "2024-03-10",
            "2024-03-10T01:02:03",
            "20240310T010203Z",
            "2024-03-10T01:02Z",
            "2024-03-10T01:02:03,5Z",
            "+002024-03-10T01:02:03Z",
            "2020-01-00T00:00:00.000Z",
            "2024-02-30T00:00:00Z",
            "2024-03-10T24:00:00Z",
            "2024-03-10T01:02:03Z[Europe/Paris]",
            "2024-03-10T01:02:03+0200",
            "2024-03-10T01:02:03.Z",
            "2024-03-10T01:02:03+02",
        ] {
            assert!(bad.parse::<Timestamp>().is_err(), "{bad:?} parsed");
        }
    }

    #[test]
    fn equality_and_order_are_by_instant() {
        let a: Timestamp = "2020-09-23T16:42:23-04:00".parse().unwrap();
        let b: Timestamp = "2020-09-23T20:42:23Z".parse().unwrap();
        let later: Timestamp = "2020-09-23T20:42:24Z".parse().unwrap();
        assert_eq!(a, b);
        assert!(a < later);
        assert_eq!(a.offset(), Offset::constant(-4));
        assert!(b.is_zulu() && !a.is_zulu());
    }

    #[test]
    fn serde_is_the_rfc_3339_string() {
        let t: Timestamp = serde_json::from_str(r#""2024-03-10T01:02:03+02:00""#).unwrap();
        assert_eq!(
            serde_json::to_string(&t).unwrap(),
            r#""2024-03-10T01:02:03+02:00""#
        );
        assert!(serde_json::from_str::<Timestamp>(r#""2024-03-10""#).is_err());
        assert!(serde_json::from_str::<Timestamp>("1710032523").is_err());
    }
}
