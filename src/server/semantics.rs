//! TMF630 collection semantics, implemented over plain JSON.
//!
//! These are the behaviours a conformant server owes a client: which resources
//! a filter selects, how they are ordered, which members `fields=` returns, and
//! what a `PATCH` does.
//!
//! They live apart from the router because they are the *rules*, not the
//! plumbing: testable on their own, and usable directly by a store that wants
//! to apply one without going through HTTP.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde_json::{Map, Value};

/// Query parameters that control the response shape rather than filter it.
const RESERVED: &[&str] = &[
    "fields", "offset", "limit", "sort", "after", "before", "filter",
];

/// Whether a query parameter shapes the response rather than filtering it.
///
/// Everything else in a query string is an attribute filter, which is what
/// makes TMF630 filtering open-ended: a server cannot know every attribute a
/// resource might have, so it treats every unreserved parameter as one.
#[must_use]
pub fn is_reserved(parameter: &str) -> bool {
    RESERVED.contains(&parameter)
}

/// The comparison suffixes TMF630 defines on a filter parameter.
const OPERATORS: &[&str] = &["eq", "ne", "gt", "gte", "lt", "lte", "regex"];

/// Whether a resource satisfies every attribute filter in `query`.
///
/// TMF630 expresses filters as query parameters named after the attribute, with
/// dots selecting nested members (`productSpecification.id=9881`), an optional
/// operator suffix (`orderDate.gte=2026-01-01`), and a comma-separated value
/// list meaning "any of" (`state=held,pending`). Filters combine with `AND`.
///
/// ```
/// use rutmf::server::matches_filters;
/// use std::collections::BTreeMap;
///
/// let resource = serde_json::json!({"lifecycleStatus": "Active", "isBundle": false});
/// let mut query = BTreeMap::new();
/// query.insert("lifecycleStatus".to_owned(), "Active".to_owned());
///
/// assert!(matches_filters(&resource, &query));
/// ```
///
/// # `regex` is a glob here, not a regular expression
///
/// The `.regex` suffix is matched with `*` and `?` wildcards rather than a
/// regular-expression engine — pulling one in to make a test double more
/// faithful would be the tail wagging the dog. So `name.regex=Basic*` matches
/// what you would expect, and an expression using any other regex construct
/// simply does not match rather than erroring.
///
/// A production server should run the client's expression through a real engine.
/// Filtering is the one TMF630 behaviour this module deliberately approximates,
/// which is why it is called out here rather than only where it is implemented.
///
/// # An absent member matches no filter, including `ne`
///
/// A resource that does not carry the member at all is excluded, whichever
/// operator was used — so `state.ne=held` selects resources whose state is
/// something other than `held`, not resources that merely fail to be `held`.
/// TMF630 does not say, and this is the reading that keeps `eq` and `ne` from
/// partitioning the collection differently from the way the client reads them:
/// under the other reading, `state=held` and `state.ne=held` together would
/// return more resources than the collection contains.
#[must_use]
pub fn matches_filters(resource: &Value, query: &BTreeMap<String, String>) -> bool {
    query
        .iter()
        .filter(|(key, _)| !is_reserved(key))
        .all(|(parameter, expected)| {
            let (path, op) = split_operator(parameter);
            let actual = resolve_path_multi(resource, path);
            if actual.is_empty() {
                return false;
            }
            // `ne` is the one operator that does not distribute as "any": on a
            // collection, "state is not held" means *no* element is held,
            // whereas reading it as "some element is not held" would match a
            // resource that plainly is.
            if op == "ne" {
                return !actual.iter().any(|value| {
                    expected
                        .split(',')
                        .any(|candidate| scalar_eq(value, candidate))
                });
            }
            // A comma list is a set of alternatives, so any match satisfies it,
            // and a repeated member satisfies it if any occurrence does.
            actual.iter().any(|value| {
                expected
                    .split(',')
                    .any(|candidate| compare(value, op, candidate))
            })
        })
}

/// Splits `attribute.op` into the attribute path and the operator.
///
/// A trailing segment is only an operator if it is one of the names TMF630
/// defines; `productSpecification.id` keeps both segments as a path.
fn split_operator(parameter: &str) -> (&str, &str) {
    match parameter.rsplit_once('.') {
        Some((path, suffix)) if OPERATORS.contains(&suffix) => (path, suffix),
        _ => (parameter, "eq"),
    }
}

/// Applies one comparison between a JSON value and a query-string operand.
///
/// `ne` never reaches here: it does not distribute over a collection the way
/// the others do, so [`matches_filters`] answers it before calling this. An arm
/// for it would be unreachable code that looked like the definition.
fn compare(actual: &Value, op: &str, expected: &str) -> bool {
    match op {
        "eq" => scalar_eq(actual, expected),
        "regex" => actual.as_str().is_some_and(|s| matches_glob(s, expected)),
        "gt" | "gte" | "lt" | "lte" => match order(actual, expected) {
            Some(Ordering::Less) => op == "lt" || op == "lte",
            Some(Ordering::Equal) => op == "gte" || op == "lte",
            Some(Ordering::Greater) => op == "gt" || op == "gte",
            None => false,
        },
        _ => false,
    }
}

/// Orders a JSON value against a query-string operand.
///
/// Numbers compare numerically, timestamps compare as instants, and everything
/// else compares as text — which is what makes lifecycle names order sensibly.
///
/// # Why timestamps are parsed rather than compared as text
///
/// Comparing RFC 3339 as text is right only while every value carries the same
/// offset, and TMF payloads do not: TM Forum's own TMF620 examples write
/// `-04:00`, and this crate keeps whatever offset arrived rather than
/// normalising it. Text comparison then gets the *sign* wrong, not merely the
/// precision — `2026-01-01T01:00:00+02:00` sorts after `2026-01-01T00:00:00Z`
/// as text, and is an hour before it as an instant. A range filter over a
/// mixed-offset collection would silently return the wrong resources, which is
/// the failure mode a filter has no way to report.
///
/// A date-only operand — `orderDate.gte=2026-01-01`, which is how a client
/// writes a day bound — is read as the start of that day in UTC.
fn order(actual: &Value, expected: &str) -> Option<Ordering> {
    if let (Some(a), Ok(b)) = (actual.as_f64(), expected.parse::<f64>()) {
        return a.partial_cmp(&b);
    }
    if let Value::String(s) = actual
        && let (Some(a), Some(b)) = (instant(s), instant(expected))
    {
        return Some(a.cmp(&b));
    }
    match actual {
        Value::String(s) => Some(s.as_str().cmp(expected)),
        Value::Number(n) => Some(n.to_string().as_str().cmp(expected)),
        _ => None,
    }
}

/// Reads an RFC 3339 timestamp, or a bare `YYYY-MM-DD` as that day's midnight
/// UTC.
///
/// Returns `None` for anything else, which is what sends the comparison back to
/// text — a lifecycle name is not a date and must not be treated as one.
fn instant(raw: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    use chrono::TimeZone as _;

    if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(raw) {
        return Some(parsed.to_utc());
    }
    let date = chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d").ok()?;
    chrono::Utc
        .from_local_datetime(&date.and_time(chrono::NaiveTime::MIN))
        .single()
}

/// A deliberately small subset of regular expressions: `*` and `?` wildcards.
///
/// A real server runs the client's expression through a regex engine. Pulling
/// one in for a test double would be the tail wagging the dog, so this covers
/// the prefix and suffix matching that filter tests actually use, and any other
/// expression simply does not match. That is stated here so a surprising test
/// failure has an explanation.
fn matches_glob(haystack: &str, pattern: &str) -> bool {
    let pattern = pattern.trim_start_matches('^').trim_end_matches('$');
    let (text, pattern) = (haystack.as_bytes(), pattern.as_bytes());

    // The standard linear backtracking match: remember the last `*` and, on a
    // mismatch, resume from one character later in the text.
    let (mut star, mut resume_at) = (None, 0);
    let (mut t, mut p) = (0, 0);

    while t < text.len() {
        if p < pattern.len() && (pattern[p] == b'?' || pattern[p] == text[t]) {
            t += 1;
            p += 1;
        } else if p < pattern.len() && pattern[p] == b'*' {
            star = Some(p);
            resume_at = t;
            p += 1;
        } else if let Some(last_star) = star {
            p = last_star + 1;
            resume_at += 1;
            t = resume_at;
        } else {
            return false;
        }
    }
    pattern[p..].iter().all(|byte| *byte == b'*')
}

/// Orders `resources` by a TMF630 `sort=` parameter.
///
/// Attributes are comma-separated and applied in order; a `-` prefix reverses
/// one. A resource missing the attribute sorts last, so a partial data set
/// still produces a stable page rather than an arbitrary one.
pub fn sort_resources(resources: &mut [Value], sort: &str) {
    let keys: Vec<(bool, &str)> = sort
        .split(',')
        .map(str::trim)
        .filter(|k| !k.is_empty())
        .map(|k| match k.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, k.strip_prefix('+').unwrap_or(k)),
        })
        .collect();

    resources.sort_by(|a, b| {
        for (descending, key) in &keys {
            let left = resolve_path(a, key);
            let right = resolve_path(b, key);
            let ordering = match (left, right) {
                (Some(l), Some(r)) => compare_values(l, r),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            };
            let ordering = if *descending {
                ordering.reverse()
            } else {
                ordering
            };
            if ordering != Ordering::Equal {
                return ordering;
            }
        }
        Ordering::Equal
    });
}

fn compare_values(left: &Value, right: &Value) -> Ordering {
    match (left, right) {
        (Value::Number(a), Value::Number(b)) => a
            .as_f64()
            .partial_cmp(&b.as_f64())
            .unwrap_or(Ordering::Equal),
        // Two timestamps order as instants, for the reason `order` gives: with
        // mixed offsets, text ordering is wrong rather than merely coarse, and
        // `sort=-orderDate` over such a collection would hand back a page in an
        // order the client did not ask for and cannot see is wrong.
        (Value::String(a), Value::String(b)) => match (instant(a), instant(b)) {
            (Some(a), Some(b)) => a.cmp(&b),
            _ => a.cmp(b),
        },
        (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
        _ => left.to_string().cmp(&right.to_string()),
    }
}

/// Projects a resource down to the members named in `fields`.
///
/// `id`, `href` and `@type` are always retained: TMF630 requires them
/// regardless of what the client asked for. A `None` or empty selection returns
/// the resource unchanged.
///
/// A dotted name selects *into* a member, so `fields=productSpecification.id`
/// keeps `productSpecification` with only its `id` — rather than dropping it,
/// which is what a top-level-only match does to the very query the dotted form
/// exists to express. Selecting both `a` and `a.b` keeps the whole of `a`, the
/// broader request winning. Arrays are projected element-wise.
#[must_use]
pub fn project_fields(resource: &Value, fields: Option<&str>) -> Value {
    let Some(fields) = fields.map(str::trim).filter(|f| !f.is_empty()) else {
        return resource.clone();
    };

    let mut selection = FieldTree::default();
    for name in fields.split(',') {
        selection.insert(name.trim());
    }
    // TMF630 returns the identity members whatever the client asked for.
    for always in ["id", "href", "@type"] {
        selection.insert(always);
    }
    selection.project(resource)
}

/// A `fields=` selection, as the tree its dotted names describe.
#[derive(Default)]
struct FieldTree {
    /// Whether this node was named outright, which selects it whole.
    whole: bool,
    children: BTreeMap<String, FieldTree>,
}

impl FieldTree {
    fn insert(&mut self, name: &str) {
        let Some((head, rest)) = split_field(name) else {
            return;
        };
        let child = self.children.entry(head.to_owned()).or_default();
        match rest {
            Some(rest) => child.insert(rest),
            None => child.whole = true,
        }
    }

    fn project(&self, value: &Value) -> Value {
        match value {
            // Project through a collection: `characteristic.name` selects the
            // name of every characteristic, not of the array itself.
            Value::Array(items) => Value::Array(
                items
                    .iter()
                    .map(|item| self.project(item))
                    .collect::<Vec<_>>(),
            ),
            Value::Object(map) => {
                let mut out = Map::new();
                // Iterate the resource, not the selection, so member order is
                // preserved.
                for (key, member) in map {
                    let Some(child) = self.children.get(key) else {
                        continue;
                    };
                    if child.whole || child.children.is_empty() {
                        out.insert(key.clone(), member.clone());
                    } else {
                        out.insert(key.clone(), child.project(member));
                    }
                }
                Value::Object(out)
            }
            other => other.clone(),
        }
    }
}

/// Splits a dotted field name into its first segment and the rest.
fn split_field(name: &str) -> Option<(&str, Option<&str>)> {
    if name.is_empty() {
        return None;
    }
    Some(match name.split_once('.') {
        Some((head, rest)) if !head.is_empty() && !rest.is_empty() => (head, Some(rest)),
        _ => (name, None),
    })
}

/// Applies an RFC 7386 JSON Merge Patch to `target` in place.
///
/// Members present in `patch` replace their counterparts, an explicit `null`
/// deletes a member, and nested objects merge recursively.
///
/// ```
/// use rutmf::server::apply_merge_patch;
///
/// let mut target = serde_json::json!({"name": "old", "description": "keep"});
/// apply_merge_patch(&mut target, &serde_json::json!({"name": "new", "description": null}));
///
/// assert_eq!(target, serde_json::json!({"name": "new"}));
/// ```
pub fn apply_merge_patch(target: &mut Value, patch: &Value) {
    let Some(patch_object) = patch.as_object() else {
        *target = patch.clone();
        return;
    };

    if !target.is_object() {
        *target = Value::Object(Map::new());
    }
    let Some(target_object) = target.as_object_mut() else {
        return;
    };

    for (key, value) in patch_object {
        if value.is_null() {
            target_object.shift_remove(key);
        } else if let Some(existing) = target_object.get_mut(key) {
            apply_merge_patch(existing, value);
        } else {
            let mut fresh = Value::Null;
            apply_merge_patch(&mut fresh, value);
            target_object.insert(key.clone(), fresh);
        }
    }
}

/// Applies an RFC 6902 JSON Patch operation list to `target`.
///
/// **Atomic**: RFC 6902 §5 requires that a patch either applies in full or not
/// at all, so the list is applied to a copy and only written back on success.
/// A `test` operation is therefore a real precondition — the earlier operations
/// in the list are undone when it fails.
///
/// `add` and `replace` are **not** interchangeable, and the difference is where
/// a naive implementation loses data: on an array, `add` inserts and shifts
/// every later element along, while `replace` overwrites in place. `replace`
/// also requires the target to exist (§4.3), where `add` creates it.
///
/// ```
/// use rutmf::server::apply_json_patch;
/// use serde_json::json;
///
/// let mut target = json!({"a": 1, "b": 2});
/// let failing = json!([
///     {"op": "replace", "path": "/a", "value": 9},
///     {"op": "test", "path": "/b", "value": 3},
/// ]);
///
/// assert!(apply_json_patch(&mut target, &failing).is_err());
/// assert_eq!(target, json!({"a": 1, "b": 2}), "nothing was applied");
/// ```
///
/// # Errors
///
/// Returns `Err` when the patch is not an array, an operation lacks `op` or
/// `path`, an operation names an unknown `op`, or a `test` operation fails.
pub fn apply_json_patch(target: &mut Value, patch: &Value) -> Result<(), String> {
    let Some(operations) = patch.as_array() else {
        return Err("JSON Patch body must be an array of operations".to_owned());
    };

    let mut draft = target.clone();
    for operation in operations {
        apply_one(&mut draft, operation)?;
    }
    *target = draft;
    Ok(())
}

fn apply_one(target: &mut Value, operation: &Value) -> Result<(), String> {
    let op = operation
        .get("op")
        .and_then(Value::as_str)
        .ok_or_else(|| "operation is missing 'op'".to_owned())?;
    let pointer = operation
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| "operation is missing 'path'".to_owned())?;

    match op {
        "add" => {
            let value = operation
                .get("value")
                .ok_or_else(|| "'add' is missing 'value'".to_owned())?;
            add_path(target, pointer, value.clone())
        }
        "replace" => {
            let value = operation
                .get("value")
                .ok_or_else(|| "'replace' is missing 'value'".to_owned())?;
            replace_path(target, pointer, value.clone())
        }
        "remove" => remove_path(target, pointer),
        "test" => {
            let expected = operation
                .get("value")
                .ok_or_else(|| "'test' is missing 'value'".to_owned())?;
            let actual = resolve_pointer(target, pointer)
                .ok_or_else(|| format!("path '{pointer}' does not exist"))?;
            if actual == expected {
                Ok(())
            } else {
                Err(format!("test failed at '{pointer}'"))
            }
        }
        "move" | "copy" => {
            let from = operation
                .get("from")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("'{op}' is missing 'from'"))?;
            // RFC 6902 §4.4: a location cannot be moved into its own child,
            // which would otherwise detach the subtree being moved.
            if op == "move" && is_proper_prefix(from, pointer) {
                return Err(format!(
                    "cannot move '{from}' into its own child '{pointer}'"
                ));
            }
            let value = resolve_pointer(target, from)
                .ok_or_else(|| format!("path '{from}' does not exist"))?
                .clone();
            if op == "move" {
                remove_path(target, from)?;
            }
            add_path(target, pointer, value)
        }
        other => Err(format!("unsupported operation '{other}'")),
    }
}

/// Resolves a dotted attribute path used by TMF filters, descending into
/// arrays.
///
/// Most of the members worth filtering on in TM Forum are collections:
/// `relatedParty`, `characteristic`, `productPrice`, `note`. A path that
/// crosses one selects **every** element's member rather than failing, so
/// `relatedParty.id=42` matches a resource that lists party 42 among several —
/// which is what a client asking that question means. An explicitly numeric
/// segment still selects one element, so `relatedParty.0.id` addresses the
/// first.
///
/// Returns every value the path reaches; empty means the member is absent.
fn resolve_path_multi<'a>(resource: &'a Value, path: &str) -> Vec<&'a Value> {
    let mut current = vec![resource];
    for segment in path.split('.') {
        let mut next = Vec::new();
        for value in current {
            match value {
                Value::Object(map) => next.extend(map.get(segment)),
                Value::Array(items) => match segment.parse::<usize>() {
                    Ok(index) => next.extend(items.get(index)),
                    // Not an index: distribute the member over the collection.
                    Err(_) => next.extend(
                        items
                            .iter()
                            .filter_map(|item| item.as_object()?.get(segment)),
                    ),
                },
                _ => {}
            }
        }
        if next.is_empty() {
            return Vec::new();
        }
        current = next;
    }
    current
}

/// Resolves a dotted attribute path to a single value, for ordering.
///
/// Sorting needs one value per resource, so where [`resolve_path_multi`] finds
/// several this takes the first — a stable, if arbitrary, choice that keeps a
/// sort total.
fn resolve_path<'a>(resource: &'a Value, path: &str) -> Option<&'a Value> {
    resolve_path_multi(resource, path).into_iter().next()
}

/// Resolves an RFC 6901 JSON Pointer.
fn resolve_pointer<'a>(root: &'a Value, pointer: &str) -> Option<&'a Value> {
    if pointer.is_empty() {
        return Some(root);
    }
    root.pointer(pointer)
}

/// Splits a pointer into its parent pointer and final token.
fn split_pointer(pointer: &str) -> Result<(&str, String), String> {
    let (parent, token) = pointer
        .rsplit_once('/')
        .ok_or_else(|| format!("'{pointer}' is not a JSON Pointer"))?;
    // RFC 6901 escapes: ~1 is '/', ~0 is '~'; order matters.
    Ok((parent, token.replace("~1", "/").replace("~0", "~")))
}

/// Whether `prefix` addresses an ancestor of `pointer` (RFC 6902 §4.4).
///
/// Compared token-wise rather than as text, so `/a/b` is a prefix of `/a/b/c`
/// but not of `/a/bc`.
fn is_proper_prefix(prefix: &str, pointer: &str) -> bool {
    pointer.len() > prefix.len()
        && pointer.starts_with(prefix)
        && pointer.as_bytes().get(prefix.len()) == Some(&b'/')
}

/// RFC 6902 §4.1 `add`: inserts into an array, or sets a member on an object.
///
/// An index equal to the array's length appends, as does `-`. Adding a member
/// that already exists on an object replaces its value.
fn add_path(root: &mut Value, pointer: &str, value: Value) -> Result<(), String> {
    if pointer.is_empty() {
        *root = value;
        return Ok(());
    }
    let (parent_pointer, token) = split_pointer(pointer)?;
    let parent = root
        .pointer_mut(parent_pointer)
        .ok_or_else(|| format!("path '{parent_pointer}' does not exist"))?;

    match parent {
        Value::Object(map) => {
            map.insert(token, value);
            Ok(())
        }
        Value::Array(items) => {
            if token == "-" {
                items.push(value);
                return Ok(());
            }
            let index = array_index(&token)?;
            if index > items.len() {
                return Err(format!("index {index} is out of bounds"));
            }
            items.insert(index, value);
            Ok(())
        }
        _ => Err(format!("path '{parent_pointer}' is not a container")),
    }
}

/// RFC 6902 §4.3 `replace`: overwrites a value that must already exist.
///
/// The distinction from [`add_path`] is not cosmetic. On an array, `add`
/// *inserts* and shifts every later element along, while `replace` overwrites
/// in place — so treating the two alike silently corrupts the array a client
/// meant to edit. And `replace` on a location that does not exist is an error,
/// where `add` creates it.
fn replace_path(root: &mut Value, pointer: &str, value: Value) -> Result<(), String> {
    if pointer.is_empty() {
        *root = value;
        return Ok(());
    }
    let (parent_pointer, token) = split_pointer(pointer)?;
    let parent = root
        .pointer_mut(parent_pointer)
        .ok_or_else(|| format!("path '{parent_pointer}' does not exist"))?;

    match parent {
        Value::Object(map) => match map.get_mut(&token) {
            Some(slot) => {
                *slot = value;
                Ok(())
            }
            None => Err(format!("path '{pointer}' does not exist")),
        },
        Value::Array(items) => {
            // `-` addresses the position *after* the last element, which by
            // definition holds no value to replace.
            let index = array_index(&token)?;
            match items.get_mut(index) {
                Some(slot) => {
                    *slot = value;
                    Ok(())
                }
                None => Err(format!("index {index} is out of bounds")),
            }
        }
        _ => Err(format!("path '{parent_pointer}' is not a container")),
    }
}

/// Parses an array index, rejecting the forms RFC 6901 §4 does not allow.
///
/// The grammar is `0` or a digit sequence with no leading zero — so `01`, `+1`
/// and ` 1` are errors rather than a silent `1`. Rust's own integer parser
/// accepts the middle one, which is why this does not simply delegate.
fn array_index(token: &str) -> Result<usize, String> {
    let invalid = || format!("'{token}' is not an array index");
    let valid = match token.as_bytes() {
        [] => false,
        [b'0'] => true,
        [first, ..] if !first.is_ascii_digit() || *first == b'0' => false,
        rest => rest.iter().all(u8::is_ascii_digit),
    };
    if !valid {
        return Err(invalid());
    }
    token.parse().map_err(|_| invalid())
}

fn remove_path(root: &mut Value, pointer: &str) -> Result<(), String> {
    let (parent_pointer, token) = split_pointer(pointer)?;
    let parent = root
        .pointer_mut(parent_pointer)
        .ok_or_else(|| format!("path '{parent_pointer}' does not exist"))?;

    match parent {
        Value::Object(map) => map
            .shift_remove(&token)
            .map(|_| ())
            .ok_or_else(|| format!("path '{pointer}' does not exist")),
        Value::Array(items) => {
            let index = array_index(&token)?;
            if index >= items.len() {
                return Err(format!("index {index} is out of bounds"));
            }
            items.remove(index);
            Ok(())
        }
        _ => Err(format!("path '{parent_pointer}' is not a container")),
    }
}

/// Compares a JSON value against the string form a query parameter carries.
fn scalar_eq(actual: &Value, expected: &str) -> bool {
    match actual {
        Value::String(s) => s == expected,
        Value::Number(n) => n.to_string() == expected,
        Value::Bool(b) => b.to_string() == expected,
        Value::Null => expected == "null",
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn query(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn filters_on_nested_and_non_string_attributes() {
        let resource = json!({
            "lifecycleStatus": "Active",
            "isBundle": false,
            "productSpecification": {"id": "9881"},
        });
        assert!(matches_filters(
            &resource,
            &query(&[("lifecycleStatus", "Active")])
        ));
        assert!(matches_filters(&resource, &query(&[("isBundle", "false")])));
        assert!(matches_filters(
            &resource,
            &query(&[("productSpecification.id", "9881")])
        ));
        assert!(!matches_filters(
            &resource,
            &query(&[("lifecycleStatus", "Retired")])
        ));
        assert!(!matches_filters(&resource, &query(&[("missing", "x")])));
    }

    #[test]
    fn a_nested_path_is_not_mistaken_for_an_operator() {
        // `productSpecification.id` ends in a segment that is not an operator
        // name, so both segments stay part of the path.
        let resource = json!({"productSpecification": {"id": "9881"}});
        assert!(matches_filters(
            &resource,
            &query(&[("productSpecification.id", "9881")])
        ));
    }

    #[test]
    fn comparison_operators_bound_a_range() {
        let resource = json!({"orderDate": "2026-06-15T00:00:00Z", "quantity": 5});

        assert!(matches_filters(
            &resource,
            &query(&[("orderDate.gte", "2026-01-01")])
        ));
        assert!(matches_filters(
            &resource,
            &query(&[("orderDate.lt", "2027-01-01")])
        ));
        assert!(!matches_filters(
            &resource,
            &query(&[("orderDate.gt", "2027-01-01")])
        ));

        // Numbers compare numerically, not as text: "5" < "10" as text.
        assert!(matches_filters(&resource, &query(&[("quantity.lt", "10")])));
        assert!(matches_filters(&resource, &query(&[("quantity.gte", "5")])));
        assert!(!matches_filters(&resource, &query(&[("quantity.gt", "5")])));
    }

    #[test]
    fn timestamps_compare_as_instants_not_as_text() {
        // `+02:00` at 01:00 is 23:00 the previous day in UTC — earlier than the
        // bound, though it sorts after it as text. TM Forum's own examples carry
        // non-`Z` offsets, so this is the ordinary case rather than a corner.
        let resource = json!({"orderDate": "2026-01-01T01:00:00+02:00"});

        assert!(matches_filters(
            &resource,
            &query(&[("orderDate.lt", "2026-01-01T00:00:00Z")])
        ));
        assert!(!matches_filters(
            &resource,
            &query(&[("orderDate.gt", "2026-01-01T00:00:00Z")])
        ));

        // The same instant written two ways is equal, so a closed bound
        // includes it either way round.
        let same = json!({"orderDate": "2026-01-01T02:00:00+02:00"});
        assert!(matches_filters(
            &same,
            &query(&[("orderDate.gte", "2026-01-01T00:00:00Z")])
        ));
        assert!(matches_filters(
            &same,
            &query(&[("orderDate.lte", "2026-01-01T00:00:00Z")])
        ));
    }

    #[test]
    fn a_date_only_bound_is_that_days_midnight_utc() {
        let resource = json!({"orderDate": "2026-06-15T10:00:00Z"});
        assert!(matches_filters(
            &resource,
            &query(&[("orderDate.gte", "2026-01-01")])
        ));
        assert!(matches_filters(
            &resource,
            &query(&[("orderDate.lt", "2027-01-01")])
        ));
        assert!(!matches_filters(
            &resource,
            &query(&[("orderDate.lt", "2026-06-15")])
        ));
    }

    #[test]
    fn sorting_by_a_timestamp_orders_by_instant() {
        // As text these are in the order written; as instants they are not.
        let mut resources = vec![
            json!({"id": "a", "orderDate": "2026-01-01T01:00:00+02:00"}),
            json!({"id": "b", "orderDate": "2026-01-01T00:00:00Z"}),
        ];
        sort_resources(&mut resources, "orderDate");
        assert_eq!(resources[0]["id"], "a", "23:00Z the previous day is first");
    }

    #[test]
    fn a_lifecycle_name_is_not_read_as_a_date() {
        // Falling into date parsing for every string would be worse than text
        // comparison, not better.
        let resource = json!({"lifecycleStatus": "Launched"});
        assert!(matches_filters(
            &resource,
            &query(&[("lifecycleStatus.gt", "Active")])
        ));
        assert!(!matches_filters(
            &resource,
            &query(&[("lifecycleStatus.gt", "Retired")])
        ));
    }

    #[test]
    fn a_comma_list_matches_any_of_its_values() {
        let resource = json!({"state": "pending"});
        assert!(matches_filters(
            &resource,
            &query(&[("state", "held,pending")])
        ));
        assert!(!matches_filters(
            &resource,
            &query(&[("state", "held,failed")])
        ));
    }

    #[test]
    fn ne_and_regex_are_supported() {
        let resource = json!({"name": "Basic Firewall", "state": "held"});
        assert!(matches_filters(
            &resource,
            &query(&[("state.ne", "pending")])
        ));
        assert!(!matches_filters(&resource, &query(&[("state.ne", "held")])));
        assert!(matches_filters(
            &resource,
            &query(&[("name.regex", "Basic*")])
        ));
        assert!(!matches_filters(
            &resource,
            &query(&[("name.regex", "Adv*")])
        ));
    }

    #[test]
    fn reserved_parameters_do_not_filter() {
        let resource = json!({"id": "1"});
        assert!(matches_filters(
            &resource,
            &query(&[("limit", "20"), ("fields", "id"), ("sort", "name")])
        ));
    }

    #[test]
    fn sorting_applies_keys_in_order_and_honours_the_minus_prefix() {
        let mut items = vec![
            json!({"name": "b", "version": 2}),
            json!({"name": "a", "version": 1}),
            json!({"name": "a", "version": 3}),
        ];
        sort_resources(&mut items, "name,-version");
        assert_eq!(
            items,
            vec![
                json!({"name": "a", "version": 3}),
                json!({"name": "a", "version": 1}),
                json!({"name": "b", "version": 2}),
            ]
        );
    }

    #[test]
    fn resources_missing_the_sort_key_go_last() {
        let mut items = vec![json!({"id": "1"}), json!({"id": "2", "name": "a"})];
        sort_resources(&mut items, "name");
        assert_eq!(items[0]["id"], "2");
    }

    #[test]
    fn projection_always_keeps_identity_members() {
        let resource = json!({"id": "1", "href": "/x/1", "@type": "ProductOffering", "name": "n", "description": "d"});
        let projected = project_fields(&resource, Some("name"));
        assert_eq!(
            projected,
            json!({"id": "1", "href": "/x/1", "@type": "ProductOffering", "name": "n"})
        );
    }

    #[test]
    fn merge_patch_deletes_with_null_and_merges_nested() {
        let mut target = json!({"name": "old", "drop": 1, "nested": {"keep": 1, "change": 2}});
        apply_merge_patch(
            &mut target,
            &json!({"name": "new", "drop": null, "nested": {"change": 3}}),
        );
        assert_eq!(
            target,
            json!({"name": "new", "nested": {"keep": 1, "change": 3}})
        );
    }

    #[test]
    fn json_patch_supports_the_full_operation_set() {
        let mut target = json!({"a": 1, "list": [1, 2, 3]});

        apply_json_patch(
            &mut target,
            &json!([{"op": "replace", "path": "/a", "value": 9}]),
        )
        .unwrap();
        assert_eq!(target["a"], 9);

        apply_json_patch(&mut target, &json!([{"op": "remove", "path": "/list/1"}])).unwrap();
        assert_eq!(target["list"], json!([1, 3]));

        apply_json_patch(
            &mut target,
            &json!([{"op": "add", "path": "/list/-", "value": 4}]),
        )
        .unwrap();
        assert_eq!(target["list"], json!([1, 3, 4]));

        apply_json_patch(
            &mut target,
            &json!([{"op": "copy", "from": "/a", "path": "/b"}]),
        )
        .unwrap();
        assert_eq!(target["b"], 9);

        apply_json_patch(
            &mut target,
            &json!([{"op": "move", "from": "/b", "path": "/c"}]),
        )
        .unwrap();
        assert!(target.get("b").is_none());
        assert_eq!(target["c"], 9);
    }

    #[test]
    fn json_patch_is_all_or_nothing() {
        // RFC 6902 §5. Without this, a failing `test` leaves the resource in a
        // state neither the client nor the server asked for.
        let mut target = json!({"a": 1, "b": 2});
        let patch = json!([
            {"op": "replace", "path": "/a", "value": 9},
            {"op": "remove", "path": "/nope"},
        ]);
        assert!(apply_json_patch(&mut target, &patch).is_err());
        assert_eq!(
            target,
            json!({"a": 1, "b": 2}),
            "the first op must be undone"
        );
    }

    #[test]
    fn json_patch_reports_failures() {
        let mut target = json!({"a": 1});
        assert!(apply_json_patch(&mut target, &json!({"not": "an array"})).is_err());
        assert!(
            apply_json_patch(
                &mut target,
                &json!([{"op": "test", "path": "/a", "value": 2}])
            )
            .is_err()
        );
        assert!(
            apply_json_patch(&mut target, &json!([{"op": "frobnicate", "path": "/a"}])).is_err()
        );
        assert!(
            apply_json_patch(&mut target, &json!([{"op": "remove", "path": "/nope"}])).is_err()
        );
    }

    #[test]
    fn json_pointer_escapes_are_decoded() {
        let mut target = json!({"a/b": 1, "c~d": 2});
        apply_json_patch(&mut target, &json!([{"op": "remove", "path": "/a~1b"}])).unwrap();
        apply_json_patch(&mut target, &json!([{"op": "remove", "path": "/c~0d"}])).unwrap();
        assert_eq!(target, json!({}));
    }

    #[test]
    fn replace_overwrites_an_array_element_rather_than_inserting_one() {
        // RFC 6902 §4.3 against §4.1. Treating the two alike turns an edit of
        // element 1 into an insertion before it, shifting the rest along and
        // lengthening the array the client meant to keep the same size.
        let mut target = json!({"list": [1, 2, 3]});
        apply_json_patch(
            &mut target,
            &json!([{"op": "replace", "path": "/list/1", "value": 9}]),
        )
        .unwrap();
        assert_eq!(target["list"], json!([1, 9, 3]));

        let mut target = json!({"list": [1, 2, 3]});
        apply_json_patch(
            &mut target,
            &json!([{"op": "add", "path": "/list/1", "value": 9}]),
        )
        .unwrap();
        assert_eq!(target["list"], json!([1, 9, 2, 3]), "add still inserts");
    }

    #[test]
    fn replace_requires_the_target_to_exist() {
        // RFC 6902 §4.3. Silently creating the member would let a typo in a
        // path look like a successful edit.
        let mut target = json!({"a": 1, "list": [1]});
        assert!(
            apply_json_patch(
                &mut target,
                &json!([{"op": "replace", "path": "/nope", "value": 9}])
            )
            .is_err()
        );
        assert!(
            apply_json_patch(
                &mut target,
                &json!([{"op": "replace", "path": "/list/5", "value": 9}])
            )
            .is_err()
        );
        assert!(
            apply_json_patch(
                &mut target,
                &json!([{"op": "replace", "path": "/list/-", "value": 9}])
            )
            .is_err(),
            "'-' addresses the position after the last element, which holds nothing"
        );
        assert_eq!(target, json!({"a": 1, "list": [1]}));
    }

    #[test]
    fn a_location_may_not_be_moved_into_its_own_child() {
        // RFC 6902 §4.4: the subtree being moved would contain its own new home.
        let mut target = json!({"a": {"b": {}}});
        assert!(
            apply_json_patch(
                &mut target,
                &json!([{"op": "move", "from": "/a", "path": "/a/b/c"}])
            )
            .is_err()
        );
        // A sibling whose name merely starts the same is not a child.
        let mut target = json!({"ab": 1});
        apply_json_patch(
            &mut target,
            &json!([{"op": "move", "from": "/ab", "path": "/abc"}]),
        )
        .unwrap();
        assert_eq!(target, json!({"abc": 1}));
    }

    #[test]
    fn array_indices_reject_the_forms_rfc_6901_excludes() {
        let mut target = json!({"list": [1, 2, 3]});
        for path in ["/list/01", "/list/+1", "/list/ 1"] {
            assert!(
                apply_json_patch(&mut target, &json!([{"op": "remove", "path": path}])).is_err(),
                "{path} is not an array index"
            );
        }
    }

    #[test]
    fn a_filter_descends_into_a_collection() {
        // The members worth filtering on in TM Forum are mostly arrays, and a
        // path that stopped at one could never match `relatedParty.id`.
        let resource = json!({
            "relatedParty": [
                {"id": "7", "role": "seller"},
                {"id": "42", "role": "buyer"},
            ],
            "characteristic": [{"name": "speed", "value": 100}],
        });

        assert!(matches_filters(
            &resource,
            &query(&[("relatedParty.id", "42")])
        ));
        assert!(matches_filters(
            &resource,
            &query(&[("relatedParty.id", "7")])
        ));
        assert!(!matches_filters(
            &resource,
            &query(&[("relatedParty.id", "99")])
        ));
        assert!(matches_filters(
            &resource,
            &query(&[("characteristic.name", "speed")])
        ));
        // Both filters must hold, on possibly different elements — TMF630
        // combines filters with AND over the resource.
        assert!(matches_filters(
            &resource,
            &query(&[
                ("relatedParty.role", "buyer"),
                ("characteristic.value.gte", "50")
            ])
        ));
    }

    #[test]
    fn an_explicit_index_still_selects_one_element() {
        let resource = json!({"relatedParty": [{"id": "7"}, {"id": "42"}]});
        assert!(matches_filters(
            &resource,
            &query(&[("relatedParty.0.id", "7")])
        ));
        assert!(!matches_filters(
            &resource,
            &query(&[("relatedParty.0.id", "42")])
        ));
    }

    #[test]
    fn ne_over_a_collection_means_no_element_matches() {
        // Read as "some element differs", this would match — which is the
        // opposite of what a client excluding buyers is asking for.
        let resource = json!({"relatedParty": [{"role": "seller"}, {"role": "buyer"}]});
        assert!(!matches_filters(
            &resource,
            &query(&[("relatedParty.role.ne", "buyer")])
        ));
        assert!(matches_filters(
            &resource,
            &query(&[("relatedParty.role.ne", "auditor")])
        ));
    }

    #[test]
    fn projection_selects_into_a_nested_member() {
        let resource = json!({
            "id": "1",
            "name": "n",
            "productSpecification": {"id": "9", "name": "spec", "version": "1.0"},
            "characteristic": [{"name": "speed", "value": 100}],
        });

        // A dotted name keeps the parent, narrowed — not nothing.
        let projected = project_fields(&resource, Some("productSpecification.id"));
        assert_eq!(
            projected,
            json!({"id": "1", "productSpecification": {"id": "9"}})
        );

        // Arrays project element-wise.
        let projected = project_fields(&resource, Some("characteristic.name"));
        assert_eq!(
            projected,
            json!({"id": "1", "characteristic": [{"name": "speed"}]})
        );

        // Naming the member outright wins over a narrower sibling request.
        let projected = project_fields(
            &resource,
            Some("productSpecification,productSpecification.id"),
        );
        assert_eq!(
            projected["productSpecification"],
            json!({"id": "9", "name": "spec", "version": "1.0"})
        );
    }

    #[test]
    fn glob_matching_covers_prefix_suffix_and_single_characters() {
        assert!(matches_glob("Basic Firewall", "Basic*"));
        assert!(matches_glob("Basic Firewall", "*Firewall"));
        assert!(matches_glob("Basic Firewall", "^Basic*ll$"));
        assert!(matches_glob("abc", "a?c"));
        assert!(!matches_glob("abc", "a?"));
    }
}
