//! TMF630 query construction: attribute selection, filtering, sorting, paging.

use std::collections::BTreeMap;
use std::fmt;

/// A comparison in a TMF630 attribute filter.
///
/// TMF630 expresses a filter as a query parameter named after the attribute,
/// optionally suffixed with an operator: `price.value.gte=100`. Bare equality
/// needs no suffix, which is why [`FilterOp::Eq`] renders as nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
#[non_exhaustive]
pub enum FilterOp {
    /// Equal — the bare `attribute=value` form.
    #[default]
    Eq,
    /// Not equal.
    Ne,
    /// Greater than.
    Gt,
    /// Greater than or equal.
    Gte,
    /// Less than.
    Lt,
    /// Less than or equal.
    Lte,
    /// Matches a regular expression.
    Regex,
}

impl FilterOp {
    /// The suffix this operator adds to the attribute name, if any.
    #[must_use]
    pub fn suffix(self) -> Option<&'static str> {
        Some(match self {
            Self::Eq => return None,
            Self::Ne => "ne",
            Self::Gt => "gt",
            Self::Gte => "gte",
            Self::Lt => "lt",
            Self::Lte => "lte",
            Self::Regex => "regex",
        })
    }

    /// The query-parameter name this operator produces for `attribute`.
    #[must_use]
    pub fn parameter_for(self, attribute: &str) -> String {
        match self.suffix() {
            Some(suffix) => format!("{attribute}.{suffix}"),
            None => attribute.to_owned(),
        }
    }
}

/// A TMF630 collection query.
///
/// Builds the `fields`, `offset`, `limit`, `sort` and attribute-filter
/// parameters TMF630 defines for a collection.
///
/// # What the documents actually declare
///
/// `fields`, `offset` and `limit` appear on every list endpoint of all fourteen
/// vendored specifications. `sort`, `filter`, `after` and `before` are declared
/// by **TMF621 and TMF639 alone** — TMF630 defines sorting generally and most
/// deployments implement it, so [`sort`](Self::sort) is offered everywhere, but
/// a server may ignore a parameter it never declared and hand back its own
/// ordering.
///
/// Attribute filters need no declaration: TMF630 makes every unreserved query
/// parameter a filter on the attribute it names.
///
/// ```
/// use rutmf::api::Query;
///
/// let q = Query::new()
///     .fields(["id", "name", "lifecycleStatus"])
///     .filter("lifecycleStatus", "Active")
///     .sort("name")
///     .limit(20);
///
/// assert_eq!(
///     q.to_query_string(),
///     "fields=id%2Cname%2ClifecycleStatus&lifecycleStatus=Active&limit=20&sort=name",
/// );
/// ```
///
/// # Filtering beyond equality
///
/// TMF630 defines comparison operators as a suffix on the attribute name, and
/// alternatives as a comma-separated value list:
///
/// ```
/// use rutmf::api::{FilterOp, Query};
///
/// let q = Query::new()
///     .filter_any("state", ["acknowledged", "inProgress"])
///     .filter_op("orderDate", FilterOp::Gte, "2026-01-01")
///     .filter_op("orderDate", FilterOp::Lt, "2027-01-01");
///
/// assert_eq!(
///     q.to_query_string(),
///     "orderDate.gte=2026-01-01&orderDate.lt=2027-01-01&state=acknowledged%2CinProgress",
/// );
/// ```
///
/// Repeating an attribute *with the same operator* widens the filter rather
/// than replacing it, which is what the comma list means on the wire:
///
/// ```
/// # use rutmf::api::Query;
/// let q = Query::new().filter("state", "held").filter("state", "pending");
/// assert_eq!(q.to_query_string(), "state=held%2Cpending");
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Query {
    fields: Vec<String>,
    /// Keyed by the rendered parameter name, so `a`, `a.gte` and `a.lt` are
    /// independent while repeats of one of them accumulate.
    filters: BTreeMap<String, Vec<String>>,
    sort: Vec<String>,
    offset: Option<usize>,
    limit: Option<usize>,
    after: Option<String>,
    before: Option<String>,
    json_path: Option<String>,
}

impl Query {
    /// An empty query.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Restricts the response to the named attributes (`fields=`).
    ///
    /// Servers always return `id` and `href` regardless.
    #[must_use]
    pub fn fields<I, S>(mut self, fields: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.fields.extend(fields.into_iter().map(Into::into));
        self
    }

    /// Adds an equality filter, e.g. `lifecycleStatus=Active`.
    ///
    /// Dotted paths select nested attributes (`productSpecification.id`).
    /// Calling this twice for one attribute matches *either* value.
    ///
    /// # A value containing a comma is ambiguous on the wire
    ///
    /// TMF630 reads a comma as "any of" and defines **no escape for one inside a
    /// value**, so `filter("name", "Acme, Inc")` is indistinguishable from
    /// asking for `Acme` or `Inc`. That is a gap in the specification rather
    /// than a choice this crate can make: percent-encoding the comma would be
    /// rejected by servers that decode before splitting, and doubling it is a
    /// convention nothing implements.
    ///
    /// Filter on a member without commas where you can. Where you cannot, the
    /// three collections that declare it accept a `JSONPath` expression through
    /// [`json_path`](Self::json_path), which has real quoting.
    #[must_use]
    pub fn filter(self, attribute: impl AsRef<str>, value: impl Into<String>) -> Self {
        self.filter_op(attribute, FilterOp::Eq, value)
    }

    /// Adds a filter with an explicit comparison, e.g. `price.value.gte=100`.
    ///
    /// Combine two calls on one attribute to express a range.
    #[must_use]
    pub fn filter_op(
        mut self,
        attribute: impl AsRef<str>,
        op: FilterOp,
        value: impl Into<String>,
    ) -> Self {
        self.filters
            .entry(op.parameter_for(attribute.as_ref()))
            .or_default()
            .push(value.into());
        self
    }

    /// Matches an attribute against any of several values.
    ///
    /// Renders as the comma-separated list TMF630 reads as alternatives:
    /// `state=acknowledged,inProgress`.
    #[must_use]
    pub fn filter_any<I, S>(mut self, attribute: impl AsRef<str>, values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.filters
            .entry(attribute.as_ref().to_owned())
            .or_default()
            .extend(values.into_iter().map(Into::into));
        self
    }

    /// Sorts by an attribute; prefix with `-` for descending order.
    #[must_use]
    pub fn sort(mut self, attribute: impl Into<String>) -> Self {
        self.sort.push(attribute.into());
        self
    }

    /// Sets the start index of the page (`offset=`).
    #[must_use]
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Sets the page size (`limit=`).
    #[must_use]
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Starts the page after an opaque cursor (`after=`).
    ///
    /// TMF621 and TMF639 declare cursor pagination alongside `offset`/`limit`
    /// on three collections — `troubleTicket`, `troubleTicketSpecification` and
    /// `resource` — and no other vendored specification declares it at all. The
    /// cursor is opaque: it comes from a server, and the only thing a client may
    /// do with one is send it back.
    ///
    /// Prefer following the `next` link when a server sends one; a stream does
    /// that on its own. Use this to *resume* from a cursor you stored.
    #[must_use]
    pub fn after(mut self, cursor: impl Into<String>) -> Self {
        self.after = Some(cursor.into());
        self
    }

    /// Ends the page before an opaque cursor (`before=`).
    ///
    /// The mirror of [`after`](Self::after), and declared by the same three
    /// collections.
    #[must_use]
    pub fn before(mut self, cursor: impl Into<String>) -> Self {
        self.before = Some(cursor.into());
        self
    }

    /// Filters with a `JSONPath` expression (`filter=`).
    ///
    /// This is **not** the TMF630 attribute filtering that
    /// [`filter`](Self::filter) produces, and the two are not interchangeable.
    /// The same three collections that declare cursor pagination also declare a
    /// `filter` parameter holding a `JSONPath` expression, and they are the only
    /// ones that declare a filter parameter at all — everywhere else, filtering
    /// is done with the attribute name itself as the parameter.
    ///
    /// ```
    /// use rutmf::api::Query;
    ///
    /// let q = Query::new().json_path("$[?(@.severity=='critical')]");
    /// assert!(q.to_query_string().starts_with("filter="));
    /// ```
    #[must_use]
    pub fn json_path(mut self, expression: impl Into<String>) -> Self {
        self.json_path = Some(expression.into());
        self
    }

    /// The configured page size, if any.
    #[must_use]
    pub fn limit_value(&self) -> Option<usize> {
        self.limit
    }

    /// The configured start index, if any.
    #[must_use]
    pub fn offset_value(&self) -> Option<usize> {
        self.offset
    }

    /// Renders the query as parameters, in a stable order.
    ///
    /// The TMF630 reserved names — `fields`, `sort`, `offset`, `limit`, `after`,
    /// `before` and `filter` — win over a filter of the same name: on the wire
    /// they *are* those parameters, and no v5 schema declares a member with one
    /// of those names.
    #[must_use]
    pub fn to_params(&self) -> BTreeMap<String, String> {
        let mut params: BTreeMap<String, String> = self
            .filters
            .iter()
            .map(|(name, values)| (name.clone(), values.join(",")))
            .collect();
        if !self.fields.is_empty() {
            params.insert("fields".to_owned(), self.fields.join(","));
        }
        if !self.sort.is_empty() {
            params.insert("sort".to_owned(), self.sort.join(","));
        }
        if let Some(offset) = self.offset {
            params.insert("offset".to_owned(), offset.to_string());
        }
        if let Some(limit) = self.limit {
            params.insert("limit".to_owned(), limit.to_string());
        }
        if let Some(cursor) = &self.after {
            params.insert("after".to_owned(), cursor.clone());
        }
        if let Some(cursor) = &self.before {
            params.insert("before".to_owned(), cursor.clone());
        }
        if let Some(expression) = &self.json_path {
            params.insert("filter".to_owned(), expression.clone());
        }
        params
    }

    /// Renders the query as a percent-encoded query string.
    #[must_use]
    pub fn to_query_string(&self) -> String {
        self.to_params()
            .iter()
            .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
            .collect::<Vec<_>>()
            .join("&")
    }
}

impl fmt::Display for Query {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_query_string())
    }
}

/// Percent-encodes a query component per RFC 3986.
fn encode(value: &str) -> String {
    use std::fmt::Write as _;

    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => {
                let _ = write!(out, "%{byte:02X}");
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_params_in_stable_order() {
        let a = Query::new().filter("b", "2").filter("a", "1").limit(5);
        let b = Query::new().limit(5).filter("a", "1").filter("b", "2");
        assert_eq!(a.to_query_string(), b.to_query_string());
        assert_eq!(a.to_query_string(), "a=1&b=2&limit=5");
    }

    #[test]
    fn encodes_reserved_characters() {
        let q = Query::new().filter("name", "Basic Firewall & More");
        assert_eq!(q.to_query_string(), "name=Basic%20Firewall%20%26%20More");
    }

    #[test]
    fn joins_fields_and_sort() {
        let q = Query::new().fields(["id", "name"]).sort("-name").sort("id");
        let params = q.to_params();
        assert_eq!(params["fields"], "id,name");
        assert_eq!(params["sort"], "-name,id");
    }

    #[test]
    fn a_repeated_attribute_widens_rather_than_replacing() {
        // Replacing rather than widening would silently drop the first value.
        let q = Query::new()
            .filter("state", "held")
            .filter("state", "pending");
        assert_eq!(q.to_params()["state"], "held,pending");
    }

    #[test]
    fn operators_render_as_a_suffix_on_the_attribute() {
        let q = Query::new()
            .filter_op("orderDate", FilterOp::Gte, "2026-01-01")
            .filter_op("orderDate", FilterOp::Lt, "2027-01-01");
        let params = q.to_params();
        assert_eq!(params["orderDate.gte"], "2026-01-01");
        assert_eq!(params["orderDate.lt"], "2027-01-01");
        assert_eq!(params.len(), 2, "the two bounds must not collide");
    }

    #[test]
    fn equality_needs_no_suffix() {
        assert_eq!(FilterOp::Eq.parameter_for("state"), "state");
        assert_eq!(FilterOp::Regex.parameter_for("name"), "name.regex");
    }

    #[test]
    fn alternatives_render_as_one_comma_list() {
        let q = Query::new().filter_any("state", ["acknowledged", "inProgress"]);
        assert_eq!(q.to_params()["state"], "acknowledged,inProgress");
    }
}
