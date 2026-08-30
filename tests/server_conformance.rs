//! The server layer against every vendored specification, over a real socket.
//!
//! # What this is, and what it is not
//!
//! TM Forum publishes a Conformance Test Kit that exercises a live
//! implementation. It is distributed through TM Forum's own channels rather
//! than a public registry, so a public CI job cannot run it — see the project
//! documentation for that gap, which this file does **not** close.
//!
//! What it does close is a narrower one that was entirely open: `server.rs`
//! exercises the handler through *one* API (TMF620) and *one* hand-written
//! store. Everything the handler promises for the other ten was unverified.
//!
//! So this reads each vendored `OpenAPI` document, discovers the collections and
//! the methods it declares, and drives `TmfHandler<MemoryStore>` over HTTP for
//! every one of them — asserting the status codes, headers and error bodies
//! TMF630 requires. It is spec-driven, so vendoring a fifteenth API extends the
//! suite without a line being added here.

#![cfg(all(feature = "server-axum", feature = "transport-reqwest"))]

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use http::{Method, StatusCode, header};
use serde_json::{Value, json};
use serde_norway::Value as Yaml;

use rutmf::api::{ReqwestTransport, TmfClient, TmfRequest};
use rutmf::server::{MemoryStore, TmfHandler};

/// The vendored documents, and the API root each is served under.
/// Collections the vendored specifications declare between them.
///
/// Quoted in README.md, `src/lib.rs` and the site.
const COLLECTION_COUNT: usize = 43;

const SPECS: &[(&str, &str)] = &[
    (
        "TMF620",
        "TMF620-Product_Catalog_Management-v5.0.0.oas.yaml",
    ),
    ("TMF621", "TMF621-Trouble_Ticket-v5.0.1.oas.yaml"),
    ("TMF622", "TMF622-ProductOrdering-v5.0.0.oas.yaml"),
    ("TMF629", "TMF629-Customer_Management-v5.0.1.oas.yaml"),
    ("TMF632", "TMF632-Party_Management-v5.0.0.oas.yaml"),
    (
        "TMF634",
        "TMF634-Resource_Catalog_Management-v5.0.0.oas.yaml",
    ),
    ("TMF637", "TMF637-ProductInventory-v5.0.0.oas.yaml"),
    (
        "TMF638",
        "TMF638-Service_Inventory_Management-v5.0.0.oas.yaml",
    ),
    (
        "TMF639",
        "TMF639-Resource_Inventory_Management-v5.0.0.oas.yaml",
    ),
    ("TMF642", "TMF642_Alarm_v5.0.1.oas.yaml"),
    ("TMF666", "TMF666-Account_Management-v5.0.0.oas.yaml"),
    ("TMF669", "TMF669-Party_Role_Management-v5.0.0.oas.yaml"),
    (
        "TMF679",
        "TMF679-Product_Offering_Qualification-v5.0.0.oas.yaml",
    ),
    ("TMF678", "TMF678-CustomerBill-v5.0.0.oas.yaml"),
];

/// One collection and the HTTP methods its specification declares.
#[derive(Debug, Default)]
struct Declared {
    get_collection: bool,
    get_item: bool,
    post: bool,
    patch: bool,
    delete: bool,
}

/// Reads the collections and methods a specification declares.
fn declared(spec: &Yaml) -> BTreeMap<String, Declared> {
    let mut out: BTreeMap<String, Declared> = BTreeMap::new();
    let Some(paths) = spec["paths"].as_mapping() else {
        return out;
    };
    for (raw, ops) in paths {
        let (Some(raw), Some(ops)) = (raw.as_str(), ops.as_mapping()) else {
            continue;
        };
        let trimmed = raw.trim_start_matches('/');
        let (name, is_item) = match trimmed.split_once("/{id}") {
            Some((name, "")) => (name, true),
            _ => (trimmed, false),
        };
        // `hub` is subscription management, not a resource collection; the
        // handler serves it as one, and `tests/server.rs` covers it.
        if name.contains('/') || name.is_empty() || name == "hub" {
            continue;
        }
        let entry = out.entry(name.to_owned()).or_default();
        for method in ops.keys().filter_map(Yaml::as_str) {
            match (method, is_item) {
                ("get", false) => entry.get_collection = true,
                ("get", true) => entry.get_item = true,
                ("post", false) => entry.post = true,
                ("patch", true) => entry.patch = true,
                ("delete", true) => entry.delete = true,
                _ => {}
            }
        }
    }
    out
}

fn load(file: &str) -> Yaml {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("specs")
        .join(file);
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    serde_norway::from_str(&raw).expect("the vendored spec is valid YAML")
}

/// Serves a `MemoryStore` on an ephemeral port and returns a client for it.
async fn serve(api: &str) -> (TmfClient, String) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a free port");
    let port = listener.local_addr().expect("a bound address").port();
    let root = format!("/tmf-api/{}/v5", api.to_ascii_lowercase());
    let base_url = format!("http://127.0.0.1:{port}{root}");

    let app = axum::Router::new().nest(
        &root,
        rutmf::server::router(TmfHandler::new(&base_url, MemoryStore::new())),
    );
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("the server runs");
    });

    let client = TmfClient::new(
        base_url.clone(),
        ReqwestTransport::new().expect("a transport"),
    )
    .expect("a valid base URL");
    (client, base_url)
}

/// A minimal but valid resource for any TMF collection.
fn seed_body(collection: &str) -> Value {
    json!({ "@type": collection, "name": format!("{collection} under test") })
}

/// Posts a body to a collection, returning the created id and any complaints.
async fn create_one(
    client: &TmfClient,
    base_url: &str,
    collection: &str,
    body: &Value,
) -> (Option<String>, Vec<String>) {
    let mut request = TmfRequest::new(Method::POST, format!("{base_url}/{collection}"));
    request.headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json"),
    );
    request.body = Some(serde_json::to_vec(body).expect("serialisable").into());

    match client.send(request).await {
        Ok(response) => {
            let mut problems = Vec::new();
            if response.status != StatusCode::CREATED {
                problems.push(format!("POST answered {} not 201", response.status));
            }
            // RFC 9110: a 201 says where the new resource lives.
            if !response.headers.contains_key(header::LOCATION) {
                problems.push("POST 201 carried no Location header".to_owned());
            }
            let id = serde_json::from_slice::<Value>(&response.body)
                .ok()
                .and_then(|v| v.get("id").and_then(Value::as_str).map(ToOwned::to_owned));
            (id, problems)
        }
        Err(error) => (None, vec![format!("POST failed: {error}")]),
    }
}

/// `GET` on the collection: a JSON array, with both TMF630 counters.
async fn check_list(client: &TmfClient, base_url: &str, collection: &str) -> Vec<String> {
    let request = TmfRequest::new(Method::GET, format!("{base_url}/{collection}"));
    match client.send(request).await {
        Ok(response) => {
            let mut problems = Vec::new();
            if !response.status.is_success() {
                problems.push(format!("GET collection answered {}", response.status));
            }
            for counter in ["x-total-count", "x-result-count"] {
                if !response.headers.contains_key(counter) {
                    problems.push(format!("GET collection omitted {counter}"));
                }
            }
            if serde_json::from_slice::<Vec<Value>>(&response.body).is_err() {
                problems.push("GET collection did not return an array".to_owned());
            }
            problems
        }
        Err(error) => vec![format!("GET collection failed: {error}")],
    }
}

/// `GET` on the item: `200`, and a validator for conditional writes.
async fn check_get(client: &TmfClient, base_url: &str, collection: &str, id: &str) -> Vec<String> {
    let request = TmfRequest::new(Method::GET, format!("{base_url}/{collection}/{id}"));
    match client.send(request).await {
        Ok(response) => {
            let mut problems = Vec::new();
            if response.status != StatusCode::OK {
                problems.push(format!("GET item answered {} not 200", response.status));
            }
            if !response.headers.contains_key(header::ETAG) {
                problems.push("GET item carried no ETag".to_owned());
            }
            problems
        }
        Err(error) => vec![format!("GET item failed: {error}")],
    }
}

/// A merge patch answers `200`.
async fn check_patch(
    client: &TmfClient,
    base_url: &str,
    collection: &str,
    id: &str,
) -> Vec<String> {
    let mut request = TmfRequest::new(Method::PATCH, format!("{base_url}/{collection}/{id}"));
    request.headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/merge-patch+json"),
    );
    request.body = Some(br#"{"@baseType":"patched"}"#.to_vec().into());
    match client.send(request).await {
        Ok(response) if response.status != StatusCode::OK => {
            vec![format!("PATCH answered {} not 200", response.status)]
        }
        Err(error) => vec![format!("PATCH failed: {error}")],
        Ok(_) => Vec::new(),
    }
}

/// A missing resource is a `404` carrying a TMF630 error body.
async fn check_missing(client: &TmfClient, base_url: &str, collection: &str) -> Vec<String> {
    let request = TmfRequest::new(Method::GET, format!("{base_url}/{collection}/no-such-id"));
    match client.send(request).await {
        Ok(response) => vec![format!("a missing item answered {}", response.status)],
        Err(error) => {
            let mut problems = Vec::new();
            if error.status() != Some(StatusCode::NOT_FOUND) {
                problems.push(format!("a missing item answered {:?}", error.status()));
            }
            if error.tmf_error().is_none() {
                problems.push("a 404 carried no TMF630 error body".to_owned());
            }
            problems
        }
    }
}

/// `DELETE` answers `204`.
async fn check_delete(
    client: &TmfClient,
    base_url: &str,
    collection: &str,
    id: &str,
) -> Vec<String> {
    let request = TmfRequest::new(Method::DELETE, format!("{base_url}/{collection}/{id}"));
    match client.send(request).await {
        Ok(response) if response.status != StatusCode::NO_CONTENT => {
            vec![format!("DELETE answered {} not 204", response.status)]
        }
        Err(error) => vec![format!("DELETE failed: {error}")],
        Ok(_) => Vec::new(),
    }
}

/// Drives every operation one collection declares.
async fn check_collection(
    client: &TmfClient,
    base_url: &str,
    collection: &str,
    ops: &Declared,
) -> Vec<String> {
    let mut problems = Vec::new();

    let id = if ops.post {
        let (id, mut created) =
            create_one(client, base_url, collection, &seed_body(collection)).await;
        problems.append(&mut created);
        id
    } else {
        // Nothing to create through the API, so seed the store the way a
        // billing run or a network would.
        let id = format!("seeded-{collection}");
        let mut body = seed_body(collection);
        body["id"] = json!(id);
        let _ = create_one(client, base_url, collection, &body).await;
        Some(id)
    };

    if ops.get_collection {
        problems.extend(check_list(client, base_url, collection).await);
    }

    let Some(id) = id else {
        return problems;
    };

    if ops.get_item {
        problems.extend(check_get(client, base_url, collection, &id).await);
    }
    if ops.patch {
        problems.extend(check_patch(client, base_url, collection, &id).await);
    }
    problems.extend(check_missing(client, base_url, collection).await);
    if ops.delete {
        problems.extend(check_delete(client, base_url, collection, &id).await);
    }
    problems
}

/// Every collection every specification declares is served, with the TMF630
/// status codes and headers.
///
/// Eleven APIs and 34 collections, driven from the documents themselves.
#[tokio::test]
async fn every_declared_collection_is_served_conformantly() {
    let mut checked = 0;
    let mut failures = Vec::new();

    for (api, file) in SPECS {
        let collections = declared(&load(file));
        let (client, base_url) = serve(api).await;

        for (collection, ops) in &collections {
            for problem in check_collection(&client, &base_url, collection, ops).await {
                failures.push(format!("{api}/{collection}: {problem}"));
            }
            checked += 1;
        }
    }

    assert!(
        checked >= COLLECTION_COUNT,
        "expected every collection across the fourteen APIs, checked {checked}"
    );
    assert!(
        failures.is_empty(),
        "the server layer is not conformant for {} collection(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Paging answers `206` while a page is partial and `200` once it is not, with
/// the counters that let a client tell the difference.
#[tokio::test]
async fn paging_reports_partial_content_across_the_boundary() {
    let (client, base_url) = serve("TMF620").await;

    for index in 0..5 {
        let mut request = TmfRequest::new(Method::POST, format!("{base_url}/productOffering"));
        request.headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        request.body = Some(
            serde_json::to_vec(
                &json!({"@type": "ProductOffering", "name": format!("offer {index}")}),
            )
            .expect("serialisable")
            .into(),
        );
        client.send(request).await.expect("created");
    }

    let mut partial = TmfRequest::new(Method::GET, format!("{base_url}/productOffering"));
    partial.query.insert("limit".into(), "2".into());
    let response = client.send(partial).await.expect("a page");
    assert_eq!(
        response.status,
        StatusCode::PARTIAL_CONTENT,
        "a short page must be 206 so the counters are worth reading"
    );
    assert_eq!(response.total_count(), Some(5));
    assert_eq!(response.result_count(), Some(2));

    let whole = TmfRequest::new(Method::GET, format!("{base_url}/productOffering"));
    let response = client.send(whole).await.expect("a page");
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.total_count(), Some(5));
}

/// No two specifications disagree about a collection name in a way that would
/// make one handler serve both wrongly.
///
/// The handler routes by collection name alone, so two APIs using one name for
/// different resources would collide in a deployment serving both. They do —
/// `importJob` and `exportJob` are in two catalogs — and that is fine precisely
/// because those are the same schema. This records which names are shared, so a
/// future overlap that is *not* the same resource is noticed.
#[test]
fn collections_shared_between_apis_are_the_same_resource() {
    let mut owners: BTreeMap<String, BTreeSet<&str>> = BTreeMap::new();
    for (api, file) in SPECS {
        for collection in declared(&load(file)).into_keys() {
            owners.entry(collection).or_default().insert(api);
        }
    }

    let shared: BTreeMap<_, _> = owners.iter().filter(|(_, apis)| apis.len() > 1).collect();
    let expected: BTreeSet<&str> = ["exportJob", "importJob"].into_iter().collect();
    let found: BTreeSet<&str> = shared.keys().map(|k| k.as_str()).collect();

    assert_eq!(
        found, expected,
        "a collection name is now used by more than one API. If the two are the \
         same schema — as the job collections are — add it here. If they are \
         not, a handler serving both APIs would route one to the other."
    );
}

/// Every vendored specification is driven by this suite.
///
/// The list above is the third place the crate names its specifications, after
/// `conformance.rs` and `coverage.rs`. A fourth API vendored and added to two
/// of the three would be served but never exercised — and nothing else would
/// notice — so the list is checked against the directory rather than trusted.
#[test]
fn every_vendored_specification_is_exercised() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("specs");
    let on_disk: BTreeSet<String> = std::fs::read_dir(&dir)
        .expect("the specs directory is missing")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension()? == "yaml")
                .then(|| path.file_name()?.to_str().map(ToOwned::to_owned))
                .flatten()
        })
        .collect();

    let listed: BTreeSet<String> = SPECS.iter().map(|(_, file)| (*file).to_owned()).collect();

    assert_eq!(
        listed, on_disk,
        "the specifications this suite drives and the ones vendored on disk \
         disagree. A document nobody drives is a server surface nobody checks."
    );
}

/// The number of collections the suite drives, so the figure in the docs is
/// asserted rather than remembered.
#[test]
fn the_suite_drives_every_declared_collection() {
    let mut all: BTreeSet<String> = BTreeSet::new();
    for (_, file) in SPECS {
        all.extend(declared(&load(file)).into_keys());
    }
    assert_eq!(
        all.len(),
        COLLECTION_COUNT,
        "the number of server collections changed; update the figure quoted in \
         README.md, src/lib.rs and site/"
    );
}
