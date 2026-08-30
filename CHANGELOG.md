# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-08-30

### Added

- **Core model** (`rutmf::core`) — I/O-free, `wasm32` clean.
  - `Extensions`: round-trip-safe vendor extension capture, order-preserving.
  - `Ref<T>`: typed entity references deriving `@type` / `@referredType` from
    the target type.
  - `Money`, `Quantity`, `Duration`, `TimePeriod` value objects; money on
    `rust_decimal::Decimal`, accepting a JSON number or string.
  - `Characteristic`, `CharacteristicSpecification`,
    `CharacteristicSpecificationRelationship`, `CharacteristicValueSpecification`
    with `ValueKind` recovering all fourteen v5 `@type` subclasses plus a
    catch-all.
  - `Timestamp`: `DateTime<FixedOffset>`, so a payload's UTC offset survives.
  - `Entity::reference()`: build a `Ref<T>` from a resource, `None` when it has
    no server-assigned `id`.
  - `RelatedParty` / `PartyOrPartyRole` with real `@type` discrimination.
  - `TmfError`: the TMF630 error body.
  - `decimal_opt`: the serde adapter, public for reuse.
  - `TmfEvent`, `EventKind` and `JsonPatchOp`: I/O-free wire data, so a webhook
    handler or a server implementation gets them without the client layer. Both
    are re-exported from `rutmf::api` for the client-side reading.
    `EventKind::name_for::<T>()` derives an event class name and
    `TmfEvent::resource::<T>()` derives its payload member, so neither end of a
    notification is stringly-typed. All 157 listener endpoints across the
    fourteen specifications follow the `{Resource}{Kind}Event` rule, which
    `every_declared_listener_is_a_kind_this_crate_names` asserts against the
    vendored documents rather than leaving as a claim.

- **Conditional requests** (`rutmf::api::Conditional`) — the client half of the
  optimistic-concurrency exchange this crate's server layer was already serving.
  `fetch` returns a `Tagged<T>`: the resource (which it derefs to) plus the
  `ETag` the server issued for it. `Tagged::update` and `Tagged::remove` send
  that back as `If-Match`, so a `PATCH` or `DELETE` against a resource somebody
  has edited in between is a `412` — `Error::is_precondition_failed()` — rather
  than a silent overwrite. `fetch_if_changed` is the read direction: `If-None-Match`,
  answering `Ok(None)` for a `304`, so a polling integration pays for a body only
  when there is a new one. The collection comes from the resource type, so none
  of it takes a path string.

  The v5 documents declare no request headers at all, so this is RFC 9110 rather
  than TMF. A server that ignores the preconditions answers as it would without
  them; `fetch` reports whether a tag was issued, and `update` refuses with
  `Error::NoEntityTag` rather than quietly writing unconditionally, so "this
  deployment does not support it" is distinguishable from "nothing changed".

  On the server side the handler now answers `304 Not Modified` to a `GET` whose
  `If-None-Match` still holds, and stamps an `ETag` on a `201` as well as a `200`
  — so a create can be followed by a conditional write without re-reading. The
  `order_lifecycle` example runs the whole exchange, and `tests/server.rs` asserts
  it over a socket: every step of the loop is a place the two halves can disagree
  in a way that looks like success.

- **`TmfHandler::with_max_page_size`** — bounds how many resources one `GET` on a
  collection may return, lowering a larger `limit` and supplying one where the
  request named none. Without it the size of a response, and the memory the store
  spends producing it, is chosen by whoever sends the request. Off by default,
  because TMF630 permits a maximum without naming one and turning it on silently
  would change what a working deployment returns. `TmfHandler::with_base_url`
  came with it, so a handler can be assembled before the port it will be reached
  on is known.

- **`Selection::capped_at`**, the same bound for a store applying a selection
  itself.

- **`rutmf::prelude`** — the traits whose methods are otherwise invisible:
  `Entity`, `TmfType`, and (behind `api`) `Conditional`, `HubOps`, `ResolveRef`,
  plus `Query`, which every call taking one of those needs. Traits only:
  `reference()`, `resolve()`, `fetch()` and `register_listener()` each read as if
  they should just work and instead fail to compile on an unimported trait, and
  that is the whole problem a prelude should solve here. Concrete types stay in
  their domain module — `rutmf::product::ProductOffering` says which API it
  belongs to, and several of the fourteen declare a `Product` or a `Category` of
  their own.

- **Product domain** (`rutmf::product`) — TMF620 v5 `ProductOffering`,
  `ProductSpecification`, `ProductOfferingPrice`, `ProductCatalog`, `Category`,
  each as a read / `Create` / `Update` triple mirroring the v5
  `_FVO` / `_MVO` schemas, plus `ImportJob` / `ExportJob` with a `JobState`
  enum that tolerates unknown states.

- **Party domain** (`rutmf::party`) — TMF632 v5 `Individual` and
  `Organization` with their create/update variants, `ContactMedium` covering all
  six v5 subclasses via `ContactMediumKind`, and the supporting value objects
  (identifications, credit profiles, skills, language abilities, other names).

- **Order domain** (`rutmf::order`) — TMF622 v5.0.0 `ProductOrder` with its
  create/update variants, nesting `ProductOrderItem`s for bundles, and
  `CancelProductOrder` modelling cancellation as a task rather than a state
  change. Three distinct state enums (`ProductOrderState`,
  `ProductOrderItemState`, `InitialProductOrderState`) keep order states, item
  states, and the two states a *client* may request from being confused for one
  another; each tolerates unknown values and reports them as non-terminal.

- **Product inventory** (`rutmf::product::Product`) — TMF637 v5.0.0, as a
  read / `Create` / `Update` triple. TMF622 and TMF637 declare the `Product`
  schema **identically**, so this is one type serving both: what an order line
  acts on *is* the inventory record, and a product read from the inventory can
  be handed straight back to an order line with no conversion.

- **Service domain** (`rutmf::service`, feature `service`) — TMF638 v5.0.0
  `Service` as a read / `Create` / `Update` triple. Lifecycle and operation are
  separate types (`ServiceState`, `ServiceOperatingStatus`), because a service
  can be lifecycle-`active` and operating-`degraded` at once. `ServiceCreate`
  requires `state` and `serviceSpecification`, and has no `operatingStatus` at
  all — the network reports that, the client does not assert it.

- **Resource domain** (`rutmf::resource`, feature `resource`) — TMF639 v5.0.0
  `Resource`. Status is split across the eight X.731 dimensions the
  specification defines, each its own type so none can be assigned to another:
  `ResourceOperationalState`, `ResourceUsageState`,
  `ResourceAdministrativeState`, `ResourceLifecycleState`,
  `ResourceAlarmStatus` (a *list* — a resource can raise several at once),
  `ResourceProceduralStatus`, `ResourceAvailabilityStatus` and
  `ResourceControlStatus`. `allocationStatus` stays a `String`: TMF639 names its
  values in prose but declares no enumeration.

  TMF639 declares no `Resource_MVO` — its `PATCH` takes the plain `Resource`
  schema — so `ResourceUpdate` is a type alias for `Resource` rather than an
  invented type, and `Resource` implements the sealed `PatchBody` marker.

- **Resource catalog** (`rutmf::resource`, feature `resource`) — TMF634 v5.0.0
  `ResourceCatalog`, `ResourceCategory`, `ResourceCandidate` and
  `ResourceSpecification`, each as a read / `Create` / `Update` triple, plus the
  specification's supporting types. This is the catalog half of the resource
  domain: what it publishes, TMF639 instantiates.

  `ResourceSpecification` is **one type for four schemas** — the base plus
  `Logical…`, `Physical…` and `ResourceFunction…`, whose members it carries as
  the union the discriminator implies, with `ResourceSpecificationKind`
  recovering which subclass a server sent.

  Two spec quirks are modelled rather than smoothed over. TMF634 declares no
  `_FVO`/`_MVO` for those subclasses, so the create and update bodies cannot
  carry `vendor`, `sku` or the connectivity members — a gap in the
  specification, not in this crate. And `ResourceCandidate` has no `name` in any
  of its three schemas while the create body *requires* one and every example
  sends one; the member is typed, and the exception is recorded in `WIRE_ONLY`.

  Adding it turned `core::refs::ResourceSpecification` and `ResourceCandidate`
  from placeholder markers into real, resolvable types — so a
  `Resource.resourceSpecification` now points at something a client can fetch.

- **Assurance domain** (`rutmf::ticket`, feature `ticket`) — TMF621 v5.0.1
  `TroubleTicket` and `TroubleTicketSpecification`, each as a read / `Create` /
  `Update` triple, with `TroubleTicketStatus` tolerating an unknown status and
  reporting it as **non-terminal** so a poller does not stop early.

  TMF621 diverges from its siblings in three ways the model follows rather than
  smooths over: responses have their own `_RES` schema (identical members, but
  `id`/`href` required); the patch body **keeps** `id` and `href` where every
  other `_MVO` drops them; and it raises two event kinds nothing else does —
  `…ResolvedEvent` and `…StatusChangeEvent`, the latter spelled *Status* where
  every other API spells it *State*. Both are now in `EventKind`.

- **Per-API `SPEC_VERSION`** — each client module exposes the exact
  specification version it was modelled from, because the covered APIs are not
  all on one patch release (TMF621 and TMF629 are at 5.0.1, the rest at 5.0.0).
  `TMF_VERSION` remains the major version in the URL path. The constants are
  asserted against the vendored manifests, so they cannot drift.

- **Fault management** (`rutmf::alarm`, feature `alarm`) — TMF642 v5.0.1
  `Alarm` with the X.733 `AlarmType` and `PerceivedSeverity` enumerations, and
  the six operations TMF642 models as their own collections: `ackAlarm`,
  `unAckAlarm`, `clearAlarm`, `commentAlarm`, `groupAlarm`, `unGroupAlarm`.

  Each task is a `POST`-and-read resource with a `…Create` and deliberately no
  `…Update`, because the specification defines `POST` and `GET` on those paths
  and nothing else. Four of them carry an `alarmPattern` so one request acts on
  every matching alarm; the two grouping tasks do not, because a correlation
  group is defined by naming its members rather than matching them.

  `PerceivedSeverity::is_active` and `TroubleTicketStatus::is_terminal` both
  treat an unrecognised value as "still needs attention": a dashboard that hides
  what it does not understand hides exactly the fault worth looking at.

- **Billing** (`rutmf::bill`, feature `bill`) — TMF678 v5.0.0 `CustomerBill`,
  `CustomerBillOnDemand`, `AppliedCustomerBillingRate` and `BillCycle`.

  TMF678 is the first API here where **no resource has the full CRUD surface**,
  and the types say so. There is no `CustomerBillCreate` because there is no
  `POST /customerBill`; `CustomerBillUpdate` carries only `state` and
  `billCycle`, because that is all the `_MVO` declares — an issued invoice is
  evidence, and a client cannot rewrite what it says the customer owes.

- **Composable client operations** (`api::ops`) — `resource_ops!` is now built
  from six single-operation macros, plus `task_ops!` (POST-and-read) and
  `readonly_ops!`. A client composes exactly the operations its specification
  declares, and `every_client_operation_is_declared_by_its_specification`
  checks the composition against the vendored paths. This also removed the
  hand-written method blocks for the jobs, the cancellation task and the six
  alarm tasks.

- **Accounts** (`rutmf::account`, feature `account`) — TMF666 v5.0.0. One
  `Account` type carries the whole family: TMF666 declares an abstract base
  with four `@type`-discriminated subclasses — `BillingAccount`,
  `FinancialAccount`, `PartyAccount`, `SettlementAccount` — and then exposes
  each as its own collection, so the client has four sets of methods over one
  shape, with `AccountKind` recovering which subclass a server sent. Plus
  `BillFormat`, `BillPresentationMedia` and `BillingCycleSpecification`.

  This closes the reference TMF678 opened: a `CustomerBill` names a billing
  account, and that account is now a real, readable resource.

- **Customer domain** (`rutmf::customer`) — TMF629 v5.0.1 `Customer` plus
  `CreditProfile`. Note the spec quirk the types encode: v5.0.1 marks `name` and
  `engagedParty` required on the **patch** schema as well as the create one.

- **Client layer** (`rutmf::api`, feature `api`).
  - `Transport` trait; `reqwest` implementation behind `transport-reqwest`,
    with bearer, basic and OAuth2 client-credentials auth (cached, refreshed
    ahead of expiry).
  - `Query`: `fields`, `sort`, `offset`, `limit`, and TMF630 attribute filtering
    with comparison operators (`FilterOp`) and comma-separated value lists.
  - `Patch` and `core::PatchBody`: the four v5 `PATCH` content types, each
    carrying its own body, so a merge body cannot be labelled as an operation
    list — and only an `…Update` type can be one.
  - `Page` / `paginate`: `X-Total-Count`, `X-Result-Count` and — where a gateway
    adds one — the RFC 8288 `Link: rel="next"` header, parsed per §B.2; plus a
    `Stream` over an entire collection.
  - `Error`: the parsed TMF630 body, with `is_retryable`, `is_not_found` and
    `is_accepted` for the asynchronous `202` every v5 write may answer with.
  - `ResolveRef`: follow a typed `Ref<T>` to the resource it points at,
    preferring the server-supplied `href` so cross-API references work.
  - `RetryTransport` / `RetryPolicy` / `Sleeper`: exponential backoff with full
    jitter, `Retry-After` in both its seconds and HTTP-date forms, applied only
    to idempotent methods, waiting through a pluggable timer.
  - `HubOps`, `Hub`, `HubCreate`, `TmfEvent`: hub/listener subscriptions,
    uniform across every client.
  - Clients, one per API: `tmf634::ResourceCatalogClient`, `tmf620::ProductCatalogClient` (all five resources
    plus import and export jobs), `tmf622::ProductOrderClient` (orders and
    cancellation requests), `tmf629::CustomerClient`, `tmf632::PartyClient`,
    `tmf637::ProductInventoryClient`, `tmf638::ServiceInventoryClient` and
    `tmf639::ResourceInventoryClient`.
  - Secrets are redacted in `Debug` on `Auth`, `ClientCredentials` and
    `ReqwestTransport`, so a transport in a log line or a panic report does not
    publish a live credential.

- **Mock server** (`rutmf::mock`, feature `mock`) — in-process TMF630
  semantics: attribute filtering with the comparison operators, sorting,
  `fields=` projection, paging with count headers and `206` for a partial page,
  merge patch and atomic RFC 6902 JSON Patch. Routing keys off the API version
  segment, so collections this crate has no client for still work. Notifications
  are recorded against each subscription's filter for tests to assert on. The
  patch, filter and sort primitives are public for anyone building a TMF server.

- **Conformance suite** — all **234** `components.examples` values vendored from
  the seven TM Forum spec repositories, asserted to parse and round-trip;
  `proptest` generators for arbitrary extensions, decimals and references. The
  per-API counts are exact constants, so a corpus that silently stops loading
  fails the suite rather than passing a smaller one.

- **Schema-coverage suite** (`tests/coverage.rs`, feature `schemars`) — reads
  the vendored OpenAPI documents and checks, over **120 Rust types mapped to 204
  v5 schemas**: member presence in both directions, member **types**, enumeration values,
  requiredness, discriminator **values** against each schema's own
  `discriminator.mapping`, that every `…Ref` class the model claims is
  specified, extension capture, that the mapping covers every type the model
  declares, that the hub surface is the one the specifications declare, and that the
  fourteen specifications do not disagree about a shared type. Polymorphic
  families map to the list of schemas they union, so a subclass member is
  required rather than merely tolerated.
  Round-tripping cannot catch any of this, because an unmodelled member survives
  in `extensions` regardless.

- **Supply-chain policy** — `deny.toml` with a `cargo-deny` CI job covering
  advisories, licences, banned crates and source registries; `openssl-sys` is
  banned so `rustls` stays the only TLS backend. A `cargo-semver-checks` job
  runs informationally until 1.0.

- **Server layer** (`rutmf::server`, feature `server`) — implement a TM Forum
  API rather than only call one.
  - `ResourceStore`: five async methods — `list`, `get`, `create`, `replace`,
    `delete` — plus an optional `has_collection`. None of them is about HTTP.
  - `TmfHandler<S>`: supplies everything else. URL routing, attribute filtering
    with the comparison operators, `sort=`, `fields=` projection,
    `offset`/`limit` paging with `X-Total-Count`/`X-Result-Count` and a `206`
    for a partial page, all four `PATCH` content types, `Location` on a `201`,
    TMF630 error bodies, and the status code for every outcome.
  - `Selection` / `Matched`: what a `GET` on a collection selects, and what
    matched. `Selection::apply` satisfies the whole thing for an in-memory
    store; a database-backed one translates it into a query, which is why it is
    passed down rather than the handler fetching everything.
  - `StoreError`: `Invalid`, `Unprocessable`, `Conflict`, `Forbidden`,
    `Internal` — and `Accepted`, so a store can answer the `202` every v5 write
    declares. The client models it the same way, as `Error::Accepted`.
  - `MemoryStore`: the in-memory implementation the mock is built on.
  - The TMF630 semantics (`matches_filters`, `sort_resources`, `project_fields`,
    `apply_merge_patch`, `apply_json_patch`) moved here from `mock`. They are
    the rules, and a real server should not have to enable a feature called
    "mock" to get them.

- **`server-axum` feature** — `rutmf::server::router(handler)` returns an
  `axum::Router`. One route, not one per operation: the handler already routes a
  TMF URL. This mirrors `transport-reqwest` on the client side — the layer is
  framework-agnostic, with one ready-made binding.

- **`schemars` feature** — `JsonSchema` on every type, with hand-written impls
  for `Ref<T>`, `Extensions` and `PartyOrPartyRole`.

- **Conditional requests on the server** — `TmfHandler` issues an `ETag` derived
  from the stored resource on `GET` and `PATCH`, and honours `If-Match` on
  `PATCH` and `DELETE` with a `412` (RFC 9110 §13.1.1, strong comparison). A
  `PATCH` is read-modify-write, so without a precondition two clients editing
  different members of one resource each silently discard the other's change. A
  store needs no version column and no extra trait method to take part.

- **`IdGenerator` / `RandomId`** — identifier policy is now a seam on
  `TmfHandler::with_id_generator`, because it is a deployment's decision: real
  systems want a UUIDv7, a ULID or a database sequence. The default gives 128
  unpredictable bits.

- **`RetryPolicy::max_retry_after`** — bounds how long a server may ask the
  client to wait before the retries are abandoned instead.

- **Product offering qualification** (`rutmf::product`, feature `api-tmf679`) —
  TMF679 v5.0.0 `CheckProductOfferingQualification` and
  `QueryProductOfferingQualification`, each as a read / `Create` / `Update`
  triple with the full five-operation surface.

  It is the step between the catalog and the order: a catalog says what a
  provider sells, this says what *this* customer may actually buy. The two
  resources ask eligibility in opposite directions — one names offerings and
  gets a per-item answer, the other gives search criteria and gets the eligible
  set back.

  Both `_FVO`s **drop every member that holds the answer**: no `state`, no
  `qualificationResult`, no `effectiveQualificationDate`, no `id`. A client asks
  the question; only the provider may write down the reply, and a request that
  tried to pre-fill it does not compile.

- **Cursor pagination** — `Query::after` and `Query::before`, and the server
  side that honours them. TMF621 and TMF639 declare `after`/`before` on three
  collections (`troubleTicket`, `troubleTicketSpecification`, `resource`) and
  nothing else declares them at all. Cursors bound the window after sorting and
  before `offset`/`limit`, and an unknown cursor selects nothing — falling back
  to the start would hand a client with a stale cursor page one again, and a
  loop that pages until nothing new arrives would never end.

- **`Query::json_path`** — the `filter` parameter those same three collections
  declare, which carries a JSONPath expression rather than the attribute-name
  filtering the other forty use. The two are different mechanisms, not synonyms,
  so they are different methods.

- **Party role management** (`rutmf::party`, feature `api-tmf669`) — TMF669
  v5.0.0 `PartyRole` and `PartyRoleSpecification`, each as a read / `Create` /
  `Update` triple, both with the full five-operation surface.

  The specification calls itself "a generalization of TMF629 Customer Management
  where Party Roles may be any — not only a Customer", and that is the
  relationship here too: a `Customer` is one party role with its own API, and
  this is the general case.

  Its four subclasses — `Supplier`, `Producer`, `Consumer`, `BusinessPartner` —
  **add no members at all**, differing only in `@type`, so this is one type with
  `PartyRoleKind` recovering the subclass rather than four identical structs.

  **Why this API and not another.** `core::PartyRole` was a zero-sized marker,
  and it is the target of `PartyOrPartyRole::Role`, which sits inside
  `RelatedParty` — carried by **49 types across 15 modules**. It was the most
  load-bearing placeholder in the crate: nearly every resource could name a
  party role that no client could fetch. `PartyRoleSpecification` went the same
  way, and `Customer`'s reference to it now resolves.

  TMF633 Service Catalog was the obvious symmetric gap — the crate has a
  resource catalog and a service inventory but no service catalog — and it is
  **not addable**: the upstream repository publishes v3, v4 and R17.5 and no v5,
  the same reason TMF641 and TMF688 are absent.

### Changed

- **`ValueKind` serves both characteristic families, and `Characteristic` can
  name its own subclass.** TMF v5 gives a characteristic's value shape a class in
  two families that differ only by suffix — `StringCharacteristic` carries a
  value, `StringCharacteristicValueSpecification` describes a permitted one — and
  the enumeration knew only the second. So a `Characteristic` had no way to
  report its subclass at all, and building one emitted `@type: "Characteristic"`:
  the bare base, which **no** characteristic in the vendored corpus uses.

  `ValueKind::from_type_name` now reads either family and
  `characteristic_type` / `value_specification_type` write either back;
  `Characteristic::new` and `CharacteristicValueSpecification::new` derive the
  subclass from the value, because that is what determines it. `ValueKind::of_value`
  is the separate question of what a value *is*, kept apart from what the sender
  said it is — the two disagreeing is a payload defect worth seeing.

- **A merge patch can remove a member.** RFC 7386 has two halves — naming a
  member sets it, naming it with `null` removes it — and the crate could express
  only the first: a field set to `None` means the patch does not *mention* the
  member, which is what leaves it unchanged. So half of what a `PATCH` is for was
  unreachable except by reaching into `extensions`, which happened to work and
  was written down nowhere. Every `…Update` type now has `deleting` and
  `deletes`.

  The round-trip guarantee was also overstated, and a `proptest` generator is
  what found it: an explicit `null` on a member the crate *models* is read as
  absence and is not re-emitted, where a `null` on an unmodelled member survives
  in `extensions`. `Option<T>` has two states where that needs three, and a
  three-state type on every field would cost every caller for a distinction the
  v5 schemas make almost nowhere — so the behaviour stands and the guarantee now
  says so, with a property test pinning which members it applies to.

- **`Page` reads the `206` TMF630 uses to mark a partial collection.** The
  crate paginated on `X-Total-Count`, then a `rel="next"` link, then "the page
  came back full" — and ignored the one signal the specification actually
  defines for this. A server is allowed to omit the counters, and one that also
  caps the page size then returns a *short* page with more to come, which the
  fallback heuristic reads as the end of the collection: the stream truncates
  and nothing reports it. `Page::partial` carries the status, `has_more`
  consults it after the exact count and before the link, and a `200` is still
  read as no information — inferring the end from one would truncate against
  every deployment that answers `200` to everything.

- **`Selection::from_query` is fallible.** It read a malformed `offset` or `limit`
  as absent, so `?limit=abc` answered a request for one page with the whole
  collection — and answered it `200`, so the client could not tell. It now
  returns the reason, and the handler turns that into a `400` naming the
  parameter.

- **`partial` counts as a terminal order state.** TMF622 lists the states without
  saying which are final; `partial` means fulfilment finished with some items
  done and others not, so the order has been worked and will not be worked again.
  Excluding it made `while !state.is_terminal() { poll().await }` — the loop
  everybody writes — never end. `is_success` was added alongside, on both the
  order and item states, so "it stopped" and "it worked" stay separable.

- **`same_origin` reads the port rather than comparing the authority as text.**
  `https://host` and `https://host:443` are one origin (RFC 6454 §4), and a TMF
  deployment writes both — the base URL comes from configuration and the `href`
  from whatever the server was told its own address is. Treating them as
  different origins refused every `href` such a server wrote and pushed callers
  to `resolve_cross_origin`, turning a spelling difference into a reason to
  switch the guard off. Userinfo is still compared, deliberately: dropping it
  would make `https://catalog.example@attacker.example/` match
  `https://catalog.example/`, and the token would go to the host after the `@`.

- **A repeated query parameter widens a filter instead of replacing it**
  (`server-axum`). `?state=held&state=pending` is what most HTTP client libraries
  produce from a list, and it means what `?state=held,pending` means. Keeping the
  last occurrence answered it with `pending` alone — a narrower result than
  either value asked for, with nothing to say half the query had been dropped.
  The reserved parameters keep last-wins, because `limit=20,50` parses as no
  limit at all.

- **A `201` reports the `href` the resource actually has.** The `Location` header
  was the URL the handler composed, not the one on the stored resource — so a
  client that sent its own `href`, or a store that normalised one, got a
  `Location` naming something other than what the body described. It now also
  carries an `ETag`, so a create can be followed by a conditional write without
  re-reading.

- **`Resolvable` grew an `Output` type**, which closed a gap on its own.
  Resolving a `Ref<T>` used to hand back a `T`, so a reference class with no
  collection of its own could never resolve. `BillingAccountRef` and
  `FinancialAccountRef` name subclasses of `Account`; **17 references in the
  crate pointed at them** and none could be followed, even though TMF666 is
  fully modelled. They now fetch from `/billingAccount` and `/financialAccount`
  and come back as `Account`. The same mechanism lets the `core::PartyRole`
  marker — which `core` needs, because `core` cannot depend on a domain
  feature — resolve into the real `party::PartyRole`.

- **Four duplicate types collapsed, and a fifth moved.** `TaxDefinition` and
  `TaxExemptionCertificate` were defined twice each — and **eight
  specifications declare each of them byte for byte**. `RelatedPlace` was
  defined four times, two of which are the same `RelatedPlaceRefOrValue`
  schema. `TaxItem` was defined twice. `CreditProfile` lived in `customer`
  while six specifications declare it and `party` needs it. All five now live
  in `core`.

  **`TaxItem` was not just duplicated, it was inconsistent.** `bill::TaxItem`
  typed `taxRate` as `f64` and `product::TaxItem` as `Decimal`, for one
  identical schema. Both specifications spell it `number/float`; it is a
  `Decimal` now, for the same reason `Money.value` is one — a rate multiplied
  into a monetary amount inherits every rounding error the binary float
  brought with it. A hand-written scan missed this, because the two Rust types
  genuinely differed; only comparing the *specifications* showed that they
  should not have.

  The rule applied is the one the crate already followed for `Note`: **merge
  what TM Forum names identically, keep what it names differently.** So
  `resource::RelatedPlace` (`RelatedPlaceRef`) and `alarm::RelatedPlace`
  (`RelatedPlace`) stay separate from `core::RelatedPlace`
  (`RelatedPlaceRefOrValue`), and `ticket::RelatedEntity` stays separate from
  `service::RelatedEntity` (`RelatedEntityRefOrValue`) — identical shapes, but
  merging them would assert an equivalence the specifications have not.

- **`one_schema_declared_identically_is_one_rust_type`** is the gate that keeps
  this from recurring: where several specifications declare a schema byte for
  byte and the crate answers with a type per module, a caller holding one cannot
  pass it where the other is wanted, for no reason the wire supports. It
  compares within a single schema name and only when the specifications agree,
  so `service::FeatureRelationship` and `resource::FeatureRelationship` stay two
  types — TMF638 and TMF639 declare two different schemas under one name.

- **The server collection count is asserted rather than remembered.**
  `the_suite_drives_every_declared_collection` pins the figure the README,
  `src/lib.rs` and the site all quote, joining the fixture, type and
  schema counts already reconciled in CI.

### Fixed

#### Twentieth pass — the events a server raised that nobody could be listening for

- **Two of the event kinds the specifications declare had no `EventKind`
  variant.** TMF638 declares `serviceOperatingStatusChangeEvent` — its `Service`
  is the one resource of the fourteen carrying both an administrative `state` and
  an operational `operatingStatus`, with a listener for each — and TMF637
  declares `ProductBatchEvent`, whose payload is an array of products rather than
  one. Neither could be named by `EventKind::name_for`, raised by `TmfHandler`,
  or subscribed to with `HubCreate::for_resource`.

  The failure was silent at both ends, which is what makes it worth a pass on its
  own: a hub registers happily against an `eventType` no server emits, and a
  server emitting a name nothing registered for reports nothing either. Added as
  `EventKind::OperatingStatusChange` and `EventKind::Batch`.

- **A lifecycle move was raised under the wrong name for six collections.** TMF621
  and TMF634 declare `…StatusChangeEvent` where the other twelve APIs declare
  `…StateChangeEvent`, and the handler raised `StateChange` for all of them. So a
  subscriber registered for `eventType=ResourceCatalogStatusChangeEvent`, which is
  the name TMF634 actually declares, received nothing — ever, with no error at
  either end.

  It could not be worked out from the resource, either: TMF634 spells the member
  `lifecycleStatus`, exactly as TMF620 does. The spelling is a property of the
  collection, so `server::state_change_kind` records it, transcribed from the
  vendored `/listener/…` paths.

- **The claim that all 157 listener endpoints follow the naming rule was not
  checked, and was not quite true.** Two new coverage tests read the paths back
  out of the fourteen documents: one requires each to decompose into a collection
  that API serves plus a kind `EventKind` names, the other requires
  `state_change_kind` to agree with which collections declare which spelling. The
  first splits on the collection as well as the suffix, because splitting at the
  wrong place still reassembles into the same string —
  `serviceOperatingStatusChangeEvent` read as `serviceOperating` plus a plain
  status change round-trips perfectly, and was exactly how the missing kind hid.

  That leaves one genuine upstream irregularity, now recorded rather than
  implied: TMF637 exposes `ProductBatchEvent` at
  `/listener/productProductBatchEvent`, the resource name having been prefixed
  onto a class that already carried it. The doubling is in the path only.

- **A `Retry-After` date before 1970 panicked the client.** The IMF-fixdate
  conversion was done in `u64`, and the civil-to-epoch arithmetic goes negative
  for such a date: an underflow panic in a debug build, and in release a wrap to
  a wait of roughly 5.8 × 10¹¹ years, which exceeds every policy's limit and
  silently abandons the retry. A server whose clock is unset sends one of these.
  The conversion is signed now, clamped at the epoch — a wait already elapsed is
  no wait.

- **A `Link: <>; rel="next"` header started a page fetch of the empty URL**,
  which failed as a cross-origin refusal — reporting a security decision where
  the server had simply named nowhere. A link with no target is no link.

- **A stream could be polled forever by a server that paged with fresh links and
  served nothing.** `Page::has_more` documents that an empty page ends the
  sequence; `PageStream` applied that rule everywhere except the path where a
  next-link was present, so a server naming a *new* page each time and returning
  no items streamed indefinitely, growing the visited-link set as it went. The
  stream now agrees with its own documentation.

- **Timestamp filters and sorts compared RFC 3339 as text, which gets the sign
  wrong.** `2026-01-01T01:00:00+02:00` sorts *after* `2026-01-01T00:00:00Z` as
  text and is an hour *before* it as an instant, so a range filter over a
  collection with mixed offsets selected the wrong resources and `sort=` ordered
  them wrongly — silently, since a filter has no way to report that it
  misunderstood. TM Forum's own TMF620 examples carry `-04:00`, and this crate
  deliberately keeps whatever offset arrived rather than normalising it, so mixed
  offsets are the ordinary case rather than a corner. `matches_filters` and
  `sort_resources` now compare timestamps as instants, with a bare `YYYY-MM-DD`
  operand read as that day's midnight UTC. A value that is not a date is still
  compared as text, so lifecycle names order as before.

- **`PartyRoleKind` could be read but not written.** Every other subclass
  enumeration in the crate offers `from_type_name` and `type_name`; this one had
  neither, with its mapping inlined in `PartyRole::kind()`. So creating a
  supplier meant `.at_type("Supplier")` — the bare string the type system exists
  to avoid — and there was no way to enumerate the subclasses at all. It now has
  the same three methods as its siblings, and so does every family:
  `every_subclass_enumeration_is_the_declared_mapping` checks all five against
  their schemas' own `discriminator.mapping`, in both directions.

- **`core::TaskState` had no `is_finished`.** Every other task-shaped enumeration
  in the crate — a catalog job, an alarm task, an on-demand bill — answers that
  question, and this is the one TMF622 and TMF679 *share*, so polling a
  `CancelProductOrder` or a qualification meant matching six variants by hand.

- **The documentation site's landing page listed twelve of the fourteen APIs**
  while the sentence beneath the list said fourteen. TMF669 and TMF679 were
  implemented, tested, and documented everywhere except the page a reader sees
  first. `the_documentation_lists_every_covered_api` now reads the API ids back
  out of `README.md` and the landing page and compares them to the vendored
  corpus, because a marketing table is the least likely thing to be remembered
  when an API is added and the most visible thing to be wrong.

- **The `Query` documentation claimed `sort` is understood by every TM Forum list
  endpoint.** Only `fields`, `offset` and `limit` are declared by all fourteen;
  `sort`, `filter`, `after` and `before` are declared by TMF621 and TMF639 alone.
  TMF630 defines sorting generally and most deployments implement it, so `sort`
  is still offered everywhere — but a server may ignore a parameter it never
  declared, and a caller reading back an order that is not the one asked for
  deserves to know that is possible. `every_declared_query_parameter_can_be_expressed`
  now asserts which parameters are universal, so the documentation cannot drift
  from the documents again.

#### Nineteenth pass — the half of the event story a server owes

- **A server built on this crate served `/hub` and then delivered nothing to
  it.** The client half was complete — `HubOps` registers a subscription,
  `EventKind::name_for::<T>()` names an event class so it cannot be misspelled,
  `TmfEvent::resource()` reads a delivered one — and the server half stopped at
  storing the subscription. A deployment that wanted to actually notify had to
  re-derive the `{Resource}{Kind}Event` naming, the payload member, the hub
  filter matching and the `/listener/{eventName}` URL, which is the whole of what
  TMF630 specifies here.

  Worse, that logic *existed* — in `MockTmfServer::emit`, and nowhere else. The
  server module documents itself as running "the same semantics code" as the
  mock; for notifications it was the mock that had semantics the server did not.
  The same class of asymmetry as `Resource` against `ResourceSpecification` in
  the pass before.

  The semantics moved into `server`, where both halves reach them:
  `matching_listeners`, `event_type_for`, `change_event`, `Listener` (with
  `delivery_url`) and `HUB_COLLECTION`. `MockTmfServer` now delegates to them, so
  the claim is true rather than aspirational.

- **Writes now raise their own events.** A `POST`, `PATCH` or `DELETE` through
  the API builds the envelope, matches it against every subscription's `query`
  and hands each match to a `Notifier` — the new seam, alongside `IdGenerator`,
  for the one part only a deployment can decide: whether delivery is a blocking
  `POST`, a queue publish or a retry loop. Without a notifier nothing is sent and
  no store read is made, so the cost is opt-in.

  `TmfHandler::notify` is public, because a change made outside a request — a
  fulfilment worker moving an order to `completed` — owes the same notification
  and should not have to rebuild the envelope.

- **A `PATCH` now reports which kind of change it was.** TMF630 raises
  `…StateChangeEvent` for a lifecycle move and `…AttributeValueChangeEvent` for
  anything else, and a client subscribes to the two separately — so answering
  "edit" for both would deliver every lifecycle move to the wrong subscription
  and none to the right one. The handler can tell, because a `PATCH` is
  read-modify-write and it holds the resource on both sides; the state member is
  spelled `lifecycleStatus`, `state` or `status` across the fourteen APIs, so all
  three are compared.

- **`POST /hub` does not raise a `HubCreateEvent`.** Registering a subscription
  is not a domain event, and treating it as one would deliver it to the
  subscription that had just been created — and to every other one.

- **`EventKind`'s documentation counted 141 listener endpoints across twelve
  specifications**, where there are 157 across fourteen. The same rot the
  seventeenth pass found elsewhere, in the one place that had escaped it.

#### Eighteenth pass — asking whether the domain model is actually complete

- **Nothing checked that the model covers the specifications.**
  `the_mapping_covers_every_type_the_model_declares` asks "does every Rust type
  have a schema?" — the easy direction, and one a crate modelling three schemas
  out of three thousand passes perfectly. The reverse was never asked, so
  "do we have the full domain model?" had no answer beyond assertion.

  `every_declared_schema_is_modelled_or_excused` now asks it. Every schema the
  fourteen documents declare must be mapped, absorbed into a mapped schema
  through `allOf`, handled generically (an event, a `…Ref`, a write variant),
  paired in `ENUMS`, or listed in the new `NOT_MODELLED` table **with the reason
  it is not modelled**. The table is checked in both directions, so an excuse for
  a schema TM Forum has since removed fails as loudly as a schema with no excuse.

- **TMF639's `Resource` was missing its four subclasses.** `Resource` is
  `@type`-discriminated over `LogicalResource`, `PhysicalResource`,
  `ResourceFunction` and `SoftwareResource`, and **nineteen members between them
  had no typed field**: `value`, `serialNumber`, `batchNumber`, `versionNumber`,
  `manufactureDate`, `standbyStatus`, `powerState`, `powerConsumingState`,
  `powerConsumingLevel`, `connectionPoint`, `connectivity`, `priority`, `role`,
  `functionType`, `autoModification`, `schedule`, `lastUpdate`,
  `isDistributedCurrent`, `targetPlatform`.

  The asymmetry is what makes this a defect rather than a scope decision: the
  *catalog* half of the same domain already models `ResourceSpecification` as
  the union of its subclasses with `ResourceSpecificationKind` to recover which
  one a server sent. The *inventory* half did not, and no test could tell —
  round-tripping hid it, because the members survived in `extensions` either way.

  `Resource` now carries the union, and `ResourceKind` recovers the subclass.
  It has an `is_logical()` because TMF639's hierarchy is two levels deep:
  `ResourceFunction` and `SoftwareResource` are `LogicalResource`s, so "is this
  logical" is not the same question as "is `@type` exactly `LogicalResource`".

- **The resource topology was absent entirely.** `ResourceGraph`, `Connection`
  and `EndpointRef` are how TMF639 says what is wired to what, and
  `ResourceFunction.connectivity` is typed as a list of them — so the subclass
  gap above could not be closed without them. Added as
  `resource::{ResourceGraph, Connection, Endpoint, ResourceGraphRelationship}`,
  with `core::{ConnectionPoint, Schedule}` markers for the two reference targets
  no vendored document defines.

  `ResourceGraph` is one of the handful of v5 schemas declared as a plain object
  rather than an `Entity`/`Extensible`, so it carries no `@type` and no `href`
  despite other schemas holding a `ResourceGraphRef` to it. The gate caught that
  too: modelling it as an ordinary entity invented three members the
  specification does not declare.

- **`JsonPatchOp.op` was a `String` over a closed vocabulary.** RFC 6902 defines
  exactly six verbs and every vendored specification repeats the list, so
  `"replaces"` compiled and the server explained the mistake — the defect class
  this crate spends its type system on, in the one type that had escaped the
  check because `JsonPatchOp` was never in the schema mapping. It is now
  `PatchOperation`, with an `Other(String)` arm so an unknown verb still
  round-trips. **Breaking**: `op` is no longer a `String`.

- **`JsonPatchOp` had no `JsonSchema` derive**, against a documented promise of
  "`JsonSchema` on every type". It was the only public serde type missing one.

- **Three schemas that turned out to be modelled all along** were simply absent
  from the mapping, so nothing checked them: `JsonPatch`,
  `RelatedPartyOrPartyRole` and `OrganizationIdentification`. Mapping them
  brought the gate to **215 Rust types against 462 v5 schemas**.

#### Seventeenth pass — credentials, concurrency, and prose that never rendered

- **A server could redirect the client's credentials to any host it named.**
  `Ref::resolve` followed the `href` of a reference wherever it pointed, and a
  transport attaches its bearer token to whatever URL it is handed. An `href` is
  *payload data* — written by the server, and in a telco integration usually
  relayed through several systems — so every `…Ref` in every response was a place
  to put an attacker's host and collect a live token.

  The crate already treated the `Link: rel="next"` header as untrusted for
  exactly this reason and refused to follow it off-origin. References were the
  larger surface and had no such check, which made the threat model
  inconsistent rather than deliberate.

  Both are now checked the same way. `resolve` and `TmfClient::get_absolute`
  refuse a URL that leaves the client's origin with the new
  `Error::CrossOrigin`, which carries the URL and the base it was compared
  against. Within a deployment the TM Forum APIs share a host and differ only by
  path, so nothing that ordinarily happens is refused. Deliberate federation
  across hosts is available as `ResolveRef::resolve_cross_origin` and
  `TmfClient::get_cross_origin` — the same calls with the guard lifted and the
  decision named. The check itself is public as `api::same_origin`, because a
  hand-written transport answering the same question differently is how a guard
  gets bypassed.

- **`If-Match` did not prevent the lost update it exists to prevent.** A `PATCH`
  is read-modify-write: the handler read the resource, checked the tag, applied
  the patch and wrote the result back. Between the check and the write, another
  request could land — and be silently discarded, with `200` to both clients.
  That is precisely the race RFC 9110 §13.1.1 sends the header to close, so a
  check followed by an unconditional write left the hole open. `DELETE` had the
  same shape.

  `ResourceStore` gains `replace_if_unchanged` and `delete_if_unchanged`,
  returning the new `Replaced` enum — `Updated` / `Missing` / `Stale`, the three
  answers HTTP needs. Both are **defaulted**, so five methods still gets a
  working server; the default narrows the window, and an override closes it.
  `MemoryStore` compares and writes under one lock. `entity_tag` is now public,
  because a store implementing the atomic version has to compare against the
  same value the handler issued.

  The handler only takes the conditional path when the client sent `If-Match`: a
  bare `PATCH` is allowed to clobber, and turning ordinary concurrent edits into
  `412`s nobody asked for would be the wrong kind of strict.

- **A short-lived `OAuth2` token disabled its own cache.** The refresh margin was
  a flat 30 seconds subtracted from the token's lifetime, so an authorization
  server granting 10-second tokens produced a cache entry that was stale the
  moment it was written — every API call fetched a token first, turning the
  cache into a second request per request. The margin is now the smaller of 30
  seconds and half the lifetime.

- **262 lines of the crate's documentation never reached docs.rs.** The modules
  under `api/`, `core/` and `server/` are private with their items re-exported,
  and rustdoc does not render the `//!` docs of a private module — so the
  explanations of why `Patch` is one type rather than two arguments, how to
  compose a `Transport`, which requests `RetryTransport` retries, how a `Page`
  detects there is more, and what a reference-target marker is for existed only
  in the source view. Each is now attached to the item it explains, where a
  reader actually looks.

- **`TmfRequest.url` documented a contract the crate itself breaks.** It
  promised no query string, but `list_absolute` passes a server-composed
  next-page URL that may carry an opaque cursor in one. A third-party transport
  written against the documented contract would have dropped it. The
  documentation now states the exception and requires a transport to *append*
  `query` rather than replace what the URL already has.

- **Stale counts throughout the documentation.** The corpus is 591 fixtures
  across fourteen specifications; the server module claimed 234, `Page` claimed
  seven specifications, and the changelog claimed 61 listener endpoints across
  seven where there are 157 across fourteen. Five shared-schema doc comments
  under-counted the specifications declaring them — `TaxDefinition` said "seven
  of the twelve" where eight of fourteen declare it, `CreditProfile` six where
  seven do, `Note` named four specifications where five declare it. The site
  listed four APIs offering `GET /hub/{id}` where the specifications declare
  five, having missed TMF679.

  Those numbers are load-bearing — "eight of the fourteen declare this
  identically" is the argument for the type living in `core` at all — so they
  are now asserted rather than remembered:
  `the_doc_comments_count_declaring_specifications_correctly` reads the count
  back out of the vendored documents and names the doc comment to update when it
  changes. The `api` module's client table listed seven of the fourteen clients
  and is now complete.

- **A stray sentence in `CreditProfile`'s documentation**, left over from the
  merge that moved it out of `customer`, described the type twice with the
  second description interrupting the first. `matches_filters` is public and
  documents TMF630 filter semantics, but implements the `.regex` operator as a
  glob — a caveat that lived only on the private helper and is now on the public
  function.

#### Sixteenth pass — the API roots nobody had checked

- **Two clients pointed at the wrong API root.** `from_host` builds
  `{host}/{API_PATH}`, so a wrong root makes *every* operation `404` — and no
  schema or round-trip check can see it, because the path never appears in a
  payload's body. It appears in the `href`s, which is where the evidence was.

  TMF634 said `resourceCatalogManagement/v5`; its own `servers` block and all
  132 `href`s in its examples say `resourceCatalog`. TMF639 said
  `resourceInventory/v5`; seventy `href`s across its own examples, TMF638's and
  TMF642's say `resourceInventoryManagement`, and none says `resourceInventory`.

  Two others were checked and left alone. TMF632's own examples say `party/v5`,
  but its `servers` block and 223 references from nine other specifications say
  `partyManagement` — its examples are the outlier. TMF642's `servers` block
  says `/v4/` in a v5.0.1 document, which is simply stale.

- **`every_client_uses_the_api_path_the_corpus_uses`** is the gate. It reads the
  path out of each API's own top-level `href`s — nested ones usually point at a
  different API and say nothing about this one — and compares it against
  `API_PATH`. Three APIs are listed as exceptions with their reasons, because
  their examples carry a document title, a stale version, or a path their own
  `servers` block contradicts.

- **`RelatedPlace.place` could parse only one arm in four.** It was typed
  `Ref<Place>`, which requires an `id`, but every specification that declares it
  types it as `PlaceRefOrValue` — a `oneOf` over `GeographicLocation`,
  `GeographicSite`, `GeographicAddress` and `PlaceRef`, of which only the last
  has an `id`. TMF679's corpus is where it surfaced: it sends an inline
  `GeographicAddress` with no `id`, and the fixture failed to parse outright.

  This was a latent defect in shipped code — TMF622, TMF637 and TMF638 declare
  the same schema and simply never exercised the value form. `PlaceRefOrValue`
  now models it, keeping the members the arms share and preserving the rest in
  `extensions`, because TMF673/674/675 are not modelled and their members will
  not be invented here.

#### Fifteenth pass — the parameters nobody was checking

- **Three query parameters the specifications declare could not be built, and
  the server mishandled all three.** `after`, `before` and `filter` were absent
  from `Query` and — worse — absent from the server's reserved-parameter list,
  which treats anything unreserved as an attribute filter. A conformant client
  sending TMF621's own `GET /troubleTicket?after=abc123` was answered by
  filtering for a member named `after`, which matched **nothing at all**.

  Reserving them alone would have swapped one wrong answer for another: a
  request to *narrow* a collection answered with the whole collection. So
  `after`/`before` are implemented, and an unimplemented JSONPath `filter` is
  refused with a `400` carrying a TMF630 body rather than silently ignored.

- **`AppliedBillingTaxRate.taxRate` was still an `f64`** — one struct below the
  `TaxItem` fixed alongside it, in the same file, and missed because the
  duplicate-type gate only compares types that model the *same* schema. It is a
  `Decimal` now, like every other rate and amount in the crate.

- **`every_declared_query_parameter_can_be_expressed`** is the gate that found
  the first of these: it collects every `query` parameter the vendored documents
  declare on a `GET`, resolving `$ref`s into `components.parameters`, and checks
  each against what a fully-populated `Query` actually puts on the wire. A
  parameter a client cannot express is a feature of the API the crate silently
  does not offer, and nothing else notices — the request is well-formed and the
  server just returns a differently-paged or unfiltered result.

- **`no_money_or_rate_is_a_binary_float`** is the gate for the second. Every
  `number` the specifications declare is a monetary quantity, a rate applied to
  one, or a `FloatCharacteristic` value — and the crate holds the first two as
  `Decimal`. An `f64` and a `Decimal` serialise to the same JSON number, so no
  round-trip or schema check could tell them apart; this one reads the field
  declarations out of the source instead.

#### Fourteenth pass — the same blind spot, one type over

The reference gate found five defects that every other check had been passing.
That is a fact about the *kind* of check, not about references: a `Ref` and a
`String` and a `Timestamp` all serialise as JSON, so a suite that checks shape
cannot tell them apart. Two more gates of that kind, and what they found:

- **Ten members with a closed vocabulary were typed as `String`.** TMF642's
  `Alarm.state`, `ackState` and `plannedOutageIndicator`; the six alarm tasks'
  shared `state`; TMF622's `ProductOrderMilestone.status`; TMF634's
  `ConnectionSpecification.associationType`; and the feature-relationship
  `relationshipType` that TMF634, TMF638 and TMF639 each declare.

  A `String` there compiles for `"may_include"` where the wire says
  `"may include"`, for `"pointToPoint"` where it says `"pointtoPoint"`, for
  `"Acknowledged"` where it says `"acknowledged"`. Each is a request a
  conformant server rejects and none was a compile error. They are now
  `AlarmState`, `AckState`, `PlannedOutageIndicator`, `AlarmTaskState`,
  `OrderMilestoneStatus`, `ConnectionAssociationType`,
  `ResourceGraphRelationshipType` and `core::FeatureRelationshipType` — each with
  an `Other(String)` arm, so an unknown value still round-trips.

  `FeatureRelationshipType` lives in `core` because three specifications declare
  it byte for byte. `ResourceGraphRelationshipType` does not, despite sharing the
  member name: TMF634 gives graph relationships `adjacency` / `connectivity`, an
  entirely different vocabulary. Matching on member name alone would have merged
  them.

- **`every_enumerated_member_is_typed_as_an_enumeration`** is the gate. It
  resolves each member's vocabulary *per schema* — `relationshipType` is
  enumerated on `FeatureRelationship` and free text on
  `ProductOfferingRelationship`, and matching by name across all specs reports 52
  members, most of them wrong. `ENUMS` now also accepts a `Schema.member` path,
  because TM Forum writes many vocabularies inline on the one member that uses
  them rather than as named schemas.

  Its first version caught only *required* members: `Option<String>` renders as
  `"type": ["string", "null"]` rather than an `anyOf`, so the scalar-only check
  was blind to the majority of the model. Deliberately breaking a member is what
  surfaced that, and the fixed gate immediately found two more real defects the
  hand-written probe had missed.

- **`every_date_time_member_is_typed_as_a_timestamp`** is the sibling gate, and
  it finds nothing. It is here anyway: a `date-time` typed as `String` accepts
  `"27/08/2026"` and loses the offset `Timestamp` preserves, and the enumeration
  gate is proof that this class of defect survives every other check.

`AlarmTaskState::is_finished` treats an unrecognised state as *not* finished, so
a client polling a task keeps polling rather than giving up on a state it does
not know.

#### Thirteenth pass — accounts, and three references pointing at the wrong class

- **`Ref<T>` was stamping the wrong discriminator in three places, and nothing
  checked it.** `Ref::new` puts `T::REF_TYPE_NAME` on the wire, so the choice of
  target *is* the discriminator. `ProductOrder.billingAccount`,
  `ProductOrderItem.billingAccount` and `Product.billingAccount` were typed
  `Ref<Account>` and emitted `AccountRef` where TMF622 and TMF637 declare
  `BillingAccountRef`; `OrganizationChildRelationship.organization` and its
  parent counterpart emitted `PartyRef` where TMF632 declares `OrganizationRef`.

  Every existing check passed throughout, because a `Ref` serialises identically
  whatever it points at — the shape is the same, the members are the same, only
  the `@type` value differs.
  `every_typed_reference_names_the_class_the_specification_declares` now reads
  the field declarations out of the source and compares each target's reference
  class against the specification's. It found the last two on its first run.

- **A marker distinction worth stating.** `core::refs` markers were described as
  placeholders that disappear when the API is modelled. Two of them will not:
  `BillingAccount` and `FinancialAccount` are subclasses of a type this crate
  *does* model, and TM Forum gives each its own `…Ref` class — so a reference to
  a subclass needs a target of its own even when the entity is modelled.

- **Two names for one idea, left unreconciled.** TMF666 calls its billing-cycle
  template `BillingCycleSpecification`; TMF678 references
  `BillCycleSpecificationRef`. No specification reconciles them, so neither does
  this crate — merging them would assert an equivalence TM Forum has not.

#### Twelfth pass — the server got a conformance suite, and a claim got corrected

- **The server layer was tested through one API out of eleven.**
  `tests/server.rs` exercises `TmfHandler` via TMF620 and a hand-written store;
  everything it promises for the other ten was unverified.
  `tests/server_conformance.rs` now reads each vendored document, discovers the
  collections and methods it declares, and drives the handler over a real socket
  for **all 34** — asserting status codes, the count headers, the `ETag`, and
  that a missing resource is a `404` with a TMF630 body. Spec-driven, so a
  twelfth API extends it with no new test code. Verified non-vacuous by
  breaking the `ETag` and a count header and watching it name every affected
  collection.

- **A claim about CTK alignment was too optimistic, and is corrected.** An
  earlier pass wrote that a CI job running the official kit was "practical
  rather than aspirational" because it ships as Docker images. Researching it
  properly: the images are distributed through TM Forum's own channels, not a
  public registry — nothing pullable under an obvious Docker Hub name, and the
  conformance pages behind the member portal. That gap needs **membership**, not
  engineering, and the documentation now says so. The publicly available
  [static CTK](https://github.com/tmforum-apis/APITestEngineStaticCTK) compares
  OpenAPI *definitions*, which is a narrower check than exercising a server.

- **The site guides only spoke commerce.** A reader arriving for alarms or bills
  found the domain-model and calling-an-api pages illustrated entirely with
  catalogs and orders. Both now cover the two shapes the later APIs introduced:
  operations modelled as their own `POST` collections, and resources that are
  read-only by design.

#### Eleventh pass — the examples caught up with the APIs

- **`assurance_workflow`** — a new example covering the three newest domains
  together: an alarm fires (TMF642), the NOC acknowledges it through a task
  collection, a trouble ticket is raised against it and resolved (TMF621), the
  alarm clears, and the bill is settled (TMF678). It is the clearest place to
  see the two shapes assurance uses that commerce does not — operations modelled
  as their own `POST` collections, and resources that are read-only on purpose.

- **`inventory_chain` now starts in the catalog.** It begins with a TMF634
  `ResourceSpecification` and instantiates it as a TMF639 resource, which is the
  seam adding TMF634 bought: before it, `Resource.resourceSpecification` pointed
  at a placeholder marker that could not be resolved into anything.

#### Tenth pass — billing, and a surface nobody was checking

- **Nothing checked that a client only offers operations the API declares.**
  Eleven collections across five specifications lack at least one of the five
  CRUD operations, and TMF678 has *no* resource with all five. The existing
  clients happened to be right — the jobs, the cancellation task and the alarm
  tasks were all hand-written — but nothing enforced it, and reaching for
  `resource_ops!` on a nearly-CRUD resource would have put
  `delete_customer_bill` on a client against an endpoint no server serves.
  `api::ops` is now composable and the composition is gated.

- **Five more broken official examples.** TMF678's `BillCycle` retrieve example
  carries `"2020-01-00T00:00:00.000Z"` — there is no day zero — and puts its
  period end before its start. Its four `Customer_Bill_Update_*` responses send
  `billDocument[].size` as a bare number where TMF678's own `Attachment` schema
  types it as a `Quantity`, and name the state member `status` where the schema
  says `state`. All five are in `KNOWN_BAD`, still guarded by
  `known_bad_fixtures_are_still_bad`.

- **A member whose schema and example disagree about its name.** TMF678
  declares `BillCycle.BillCycleSpecification` capitalised, against v5's
  `camelCase` everywhere else, while its own example sends
  `billCycleSpecification`. The typed field follows the schema — that is what a
  conformant server is checked against — and the example's spelling still
  round-trips through `extensions` rather than being dropped. Both are
  documented on the field, because an integrator meeting a real deployment
  needs to know the two exist.

#### Ninth pass — fault management, and a gate that had never run

- **The spec-drift job had been checking nothing for TMF622.** It pointed at
  `tmforum-apis/TMF622_ProductOrdering`, which does not exist — the repository
  is `TMF622_ProductOrder`. Every run since the job was written emitted "could
  not fetch" and carried on, because the job is `continue-on-error` and a fetch
  failure was treated as network flakiness. All ten targets are now verified to
  resolve, a missing *local* file is a hard error rather than a silent skip, and
  the warning says that a persistent failure means the repository name is wrong.

- **A vendored filename that did not match upstream.** TMF642's document is
  `TMF642_Alarm_v5.0.1.oas.yaml` upstream, with underscores where every other
  API uses dashes. Vendoring it under the tidier name would have made it
  invisible to the drift job in exactly the way TMF622 was.

- **A macro that imposed uniformity the specification does not have.** The six
  alarm tasks were first written through one `alarm_task!` macro assuming a
  shared shape. The coverage gate rejected it: `GroupAlarm` and `UnGroupAlarm`
  have no `alarmPattern`, and each `_FVO` requires a different set of members.
  The field lists are written per task now — which is the same conclusion the
  crate reached about `_FVO`/`_MVO` variants generally.

#### Eighth pass — assurance, and one duplicate too many

- **`Note` was three identical types.** `order::Note`, `service::Note` and
  `resource::Note` were separate declarations of a schema that TMF621, TMF622,
  TMF638 and TMF639 declare **byte for byte identically** — the same situation
  as `Product`, resolved the opposite way. They are now one `core::Note`, and
  `Note` joins the `SHARED` list so the four specifications are checked for
  agreement rather than assumed to have it.

- **The hub gate caught a claim going stale on the day it was written.**
  `HubOps::get_listener` said only TMF629 and TMF639 offer `GET /hub/{id}`.
  TMF621 offers it too, and the check added in the sixth audit failed the build
  until the note was corrected — which is exactly the job it was added to do.

#### Seventh pass — what adding an eighth API found

Adding TMF634 exercised the gates against a specification they had never seen,
and both of them earned their keep.

- **`JsonPatchOp` dropped unknown members.** Every other type in the crate
  captures what it does not model; the patch operation did not, so a member
  beyond `op`/`path`/`value`/`from` was silently lost. That is not hypothetical:
  TMF634's own `ResourceCatalog` patch example writes the new value in a member
  named after the field rather than under `value`, and relaying that request
  would have discarded the thing being written. It now carries `extensions` like
  everything else.

- **Two mechanisms for specifications that contradict themselves.** TMF634
  requires `ResourceCandidate.name` on a create body that declares no such
  member, and ships a patch example writing `"/path"` where RFC 6902 requires
  `"path"`. Neither can be modelled away, so each is recorded *and* verified:
  `WIRE_ONLY` in `tests/coverage.rs` allows a typed member the schema omits and
  fails unless a vendored fixture carries it, and `KNOWN_BAD` in
  `tests/conformance.rs` excludes an invalid example and fails if it ever starts
  parsing. An exception that cannot show its evidence is not allowed to exist.

- **`tmf_value!` gained a `@renamed` section**, because TMF634's
  `TargetResourceSchema` is a value object consisting of nothing but two
  `@`-prefixed members.

#### Sixth audit — the layers below the model

The first five audits compared the model and the client to the specifications.
The sixth went after the code those rest on: the RFCs the crate implements, the
gate that was supposed to be running, and the claims the prose makes.

- **RFC 6902 `replace` inserted into arrays instead of overwriting.** `add` and
  `replace` shared one code path, so `{"op":"replace","path":"/list/1"}` shifted
  every later element along and left the array one longer than the client meant
  to leave it — silent data corruption, in the operation a client reaches for
  precisely to avoid resending a whole array. `replace` also succeeded on a
  target that did not exist, which RFC 6902 §4.3 forbids and which turns a typo
  in a path into a successful-looking edit. The two are now separate
  operations; `move` into its own child is rejected per §4.4; and array indices
  reject the forms RFC 6901 §4 excludes (`01`, `+1`), which Rust's own integer
  parser accepts.

- **The `Link` header parser split on `,` and `;`.** Both delimiters occur
  inside a link value — in the URI, and in quoted parameters. TMF630 filter
  values *are* comma-separated lists, so a server echoing the client's filter
  into its next-page link produced a header the parser silently dropped: the
  stream ended early, short, with no error anywhere. It is now a
  character-by-character parse per RFC 8288 §B.2, and `rel="next last"` — a
  space-separated relation list, §3.3 — is recognised.

- **Credentials leaked through `Debug`.** `Auth::Bearer`, `Auth::Basic`,
  `ClientCredentials.client_secret`, the cached OAuth2 token and the values of
  default headers were all printed in full by the derived `Debug` on
  `ReqwestTransport` — and by anything wrapping it, such as `RetryTransport`. A
  transport is one `dbg!`, `tracing` span or panic report away from an error
  tracker. All four now have hand-written `Debug` implementations that redact,
  and a test asserts no secret survives any of them.

- **`Retry-After` was clamped by `max_delay`.** A gateway saying "wait 60
  seconds" was re-asked after ten, still rate-limited, spending the whole retry
  budget without ever waiting long enough to succeed — while the documentation
  said the header was honoured. `max_delay` now bounds only the *computed*
  backoff; a `Retry-After` longer than the new `max_retry_after` ends the
  retries and returns the response, so the caller learns it was throttled and
  for how long.

- **Server-assigned ids were not as random as claimed.** `RandomState::new()`
  seeds a thread-local pair once from the OS and then *increments* it, so
  hashing a constant produced a keyed hash over a marching key — a related-key
  construction, 64 bits wide, described in the code as "random". `RandomId` now
  keys one `RandomState` for its lifetime and uses it as a PRF over a counter,
  which is what SipHash is built for, and yields 128 bits.

- **TMF630 filters could not match inside a collection.** `relatedParty.id=42`
  and `characteristic.name=speed` never matched anything, because path
  resolution stopped at an array — and most of what is worth filtering on in TM
  Forum *is* an array. A dotted path now distributes over the elements. `ne` is
  the deliberate exception: over a collection it means no element matches, since
  "some element is not a buyer" would match a resource that plainly has one.

- **`fields=` ignored dotted names.** `fields=productSpecification.id` matched
  no top-level member and so dropped the member entirely — discarding exactly
  what the dotted form exists to request. The selection is now a tree; arrays
  project element-wise, and naming a member outright still wins over a narrower
  sibling request.

- **Seven of CI's twenty-one feature-matrix commands were failing.** `api`,
  `server`, `server-axum`, `mock`, `schemars` and `transport-reqwest` each left
  a declaration macro unused, which `RUSTFLAGS: -D warnings` turns into an
  error. The job that existed to prove the crate slices cleanly had not passed
  in some time. The macros carry a scoped `allow` with the reason, four
  realistic *combinations* were added — the shapes where the unused-macro case
  actually arises — and all twenty-six now pass.

- **Numbers in the documentation had drifted, and one had been false since
  phase 4.** `lib.rs` claimed 176 vendored examples where the README claimed
  234 for the same corpus; "132 mapped types" was 120 types over 204 schemas;
  "47 listener endpoints across the four specifications" was 61 across seven;
  and `HubOps::get_listener` said only TMF629 offers `GET /hub/{id}`, which
  stopped being true when TMF639 arrived. Every one of those quantities is now
  a constant asserted by a test, and a new gate reads the hub surface back out
  of the specifications. The conformance suite's `>= 230` and `>= 170` bounds —
  two different lower bounds for one 234-file corpus — are exact per-API counts.

- **"The vendored v5 documents declare no response headers at all" was wrong.**
  Repeated in three places. They declare `X-Total-Count` and `X-Result-Count`.
  The corrected claim is narrower and is the one that actually carries weight:
  they declare no `Location`, which is what makes setting it an RFC 9110
  decision rather than a TMF one. Relatedly, `Link: rel="next"` is *not* in the
  v5 documents either — the pagination module now says so, and describes the
  support as an accommodation for real gateways rather than a TMF requirement.

- **`EventKind::from_event_name` allocated and sorted a `Vec` on every call**, on
  the path that runs once per delivered event. The order is declared once, and
  a test asserts it is still longest-suffix-first — the property that stops a
  new kind being shadowed by a shorter suffix and never matching.

#### Earlier audits

The following came out of a spec-by-spec audit against the vendored
OpenAPI documents. Every one of them was live in a crate whose stated purpose
is fidelity, and every one was invisible to the round-trip suite.

- **The published crate shipped its tests but not the specifications they
  read.** `exclude` listed `/specs` and not `/tests`, so `cargo test` on the
  packaged crate could not run at all, and 241 fixture files made the download
  four times larger than it needed to be. Both directories are now excluded as a
  pair, and a CI job runs `cargo package --allow-dirty` — which compiles the
  packaged crate — and fails if either reappears. The package went from 333
  files to 80.

- **The mock never set `Location` on a `201`.** Its own client reads `Location`
  to find the monitor URL of a `202`, so the header was understood on one side
  and never sent on the other. The handler now sets it, per RFC 9110. The
  vendored v5 documents declare exactly two response headers — `X-Total-Count`
  and `X-Result-Count`, both on collections — and no `Location`, so this is the
  HTTP rule rather than a TMF claim.

- **The mock assigned sequential ids from a counter.** Harmless in a test
  double; a defect in a server, which is what that code now also is — a counter
  leaks how many resources the server holds and makes another tenant's resource
  guessable. Ids are random, and a test asserts they do not look like a counter.

- **`Resource.alarmStatus` was a scalar; TMF639 types it as an array.** A
  resource can raise several alarms at once, and the model could hold one. The
  shape check caught it as soon as TMF639 was mapped.

- **The coverage gate keyed types by bare struct name.** With four APIs there
  were no collisions. With seven there are: three modules declare a
  `RelatedPlace`, two a `Feature`. The gate collapsed each group into one entry
  and checked the wrong type against the wrong schema, silently. Types are now
  keyed by `module::Struct` on both sides.

- **The conformance suite classified fixtures by file name.** TMF638 ships its
  notification examples as `Create_request.json` and its JSON Patch bodies as
  `Service_partialupdate_example_11_request.json`. Five fixtures would have been
  checked as the wrong shape and the suite would have passed. Content now
  decides the two unambiguous shapes: an operation list is an array of objects
  carrying `op`, a notification carries `eventType`.

- **`core::EventSource` duplicated a marker.** It and the target TMF638 needs
  for `relatedEntity` were the same type — `Entity` / `EntityRef` — declared
  twice. Now one `core::AnyEntity`. **Breaking:** `EventSource` is gone.

- **A v4 member name survived into a v5 type.** `ProductSpecification` declared
  `productSpecificationCharacteristic`; v5 calls it `productSpecCharacteristic`,
  so every characteristic a v5 server sent went into `extensions` unread. Same
  for `Category.parentId` (v5 carries a `parent: CategoryRef`) and
  `ProductOfferingPrice.poprRelationship` (v5: `popRelationship`).
- **Members the v5 schemas do not define** were declared anyway:
  `ProductOfferingPrice.isTaxIncluded` and `.priceDuration`,
  `ProductOfferingPriceRelationship.validFor`, `CharacteristicRelationship.href`.
- **Roughly 250 specified members had no typed field**, most of them on the
  `_MVO` patch bodies: an offering's characteristics, policies, market segments,
  bundled offerings and external identifiers could not be patched at all. All
  four APIs are now member-complete, gated by `tests/coverage.rs`.
- **`Default` produced `{"@type":""}`** on every patch body — a payload no
  conformant server accepts — because `#[derive(Default)]` on a non-`Option`
  `String` yields an empty one.
- **A `PATCH` body could be labelled with the wrong content type.**
  `patch(id, body, PatchKind)` accepted an `_MVO` object with
  `application/json-patch+json`, which every conformant server rejects. `Patch`
  now carries the body inside the variant.
- **`RetryTransport` did not sleep** unless `transport-reqwest` was enabled: it
  re-sent immediately, in a tight loop, at a server that had just rate-limited
  it. Backoff now goes through a `Sleeper`, and `RetryTransport::new` exists
  only where a default one does.
- **RFC 6902 patches were not atomic**, so a failed `test` operation left the
  resource half-patched — the opposite of what a precondition is for.
- **The conformance suite asserted 41 of 176 fixtures.** Event envelopes and
  JSON Patch bodies were vendored and never checked. The mapping is now derived
  from the naming convention, with a test that fails on any unclassifiable file.
- **`@type` was invented when a payload omitted it**, and a timestamp's UTC
  offset was rewritten to `Z`. Both appear in TM Forum's own examples, so
  "nothing is invented" was false in practice. Absence of `@type` is now
  preserved — read the class through `type_name()` — and `Timestamp` keeps its
  offset. The round-trip guarantee no longer has an exception.
- **`Query` silently discarded repeated filters.** Two calls for one attribute
  kept only the last, quietly narrowing the result set.
- **Pagination stopped early against a cursor-paging server.** A short page
  ended the stream even when the server sent `Link: rel="next"`.
- **`Ref::to` panicked** on an entity with no `id`. Replaced by
  `Entity::reference()`, which returns `Option`.
- **The mock server routed by a hard-coded list of collection names**, so any
  other collection was mis-parsed as an item lookup.
- **An arbitrary JSON error body deserialized into an empty `TmfError`**,
  discarding a gateway's own message. A body carrying neither `code` nor
  `reason` is now reported as a raw status.

A second audit turned the coverage gate on itself and found it compared member
*names* and nothing else:

- **Three members were `String` where the spec says `array`** —
  `productOrderItem` on order error messages, milestones and jeopardy alerts,
  which are lists of `ProductOrderItemRef`. No fixture exercises them.
- **Two members could not be parsed at all.** `quoteItem` and
  `productOfferingQualificationItem` were `Ref<T>`, whose `id` is mandatory,
  but `QuoteItemRef` and `ProductOfferingQualificationItemRef` have no `id`
  member. They — and `ProductOrderItemRef` — are now their own types.
- **`Ref<T>` swept `version` into `extensions`**, a member eight `…Ref` schemas
  define and the vendored examples carry 124 times.
- **`Attachment` and `OrderedProduct` had no `@referredType`**, which only the
  reference arm of a `…RefOrValue` union carries.
- **`TmfEvent` was missing six members** of the v5 `Event` schema, including
  `source`, `reportingSystem` and `relatedParty`.
- **Seven types were declared and never exported**, so a public field's type
  could not be named downstream.
- **Streaming looped against a cursor-paging server.** A `Link: rel="next"` was
  read as "there is more" and then an *offset* request was re-derived, which for
  an opaque cursor fetches the first page forever. The stream now follows the
  link, and refuses to revisit one.
- **A pagination link was followed to any origin**, handing the transport's
  credentials to whatever host the server named.

A third audit compared the *client* to the rest of the specification — the
discriminators, paths, methods and status codes:

- **`202 Accepted` produced `invalid type: null, expected struct …`.** Every v5
  `POST` and `PATCH` declares it with an empty body beside the synchronous
  answer, for deployments that fulfil a write asynchronously. It is now
  `Error::Accepted`, carrying the status and the `Location` monitor URL.
- **Four types claimed a reference class no specification defines** —
  `CatalogRef`, `CustomerRef`, `ImportJobRef`, `ExportJobRef`. A `Ref<Customer>`
  stamped a `@type` nothing could route; a customer is referenced as a
  `PartyRoleRef`, which is what every example does.
- **`Attachment` and `OrderedProduct` stamped the `oneOf` wrapper's name** into
  `@type`. The v5 schema states that `@type` belongs to the entity, and its
  `discriminator.mapping` admits only `Attachment`/`AttachmentRef` and
  `Product`/`ProductRef`.
- **`HubOps::get_listener` documented its exception backwards.** Only TMF629
  defines `GET /hub/{id}`; no API lets you list subscriptions at all.
- **`SkillTarget` was exported, never used, and named a schema no spec defines.**

- **The declared MSRV was never achievable.** `Cargo.toml` said 1.85, but the
  crate uses a let-chain and `bon` requires 1.88, so 1.85 could not compile. The
  MSRV is now 1.88 and verified against real toolchains rather than asserted.
- **Value objects dropped members that real payloads carry.** The v5 schemas
  define `Money`, `Quantity`, `Duration` and `TimePeriod` without `@type`, yet
  TM Forum's own TMF622 examples send one. They now carry an `Extensions` map,
  so a payload survives intact whether or not the schema anticipated it.
- **Examples broke a default-feature build.** They use the opt-in client layer
  but did not declare `required-features`, so plain `cargo test` and
  `cargo build --examples` failed. Both are now declared and covered by CI.

- **Eight fields were wrongly non-optional** across seven shared types
  (`ExternalIdentifier.id`, `Characteristic.name`,
  `CharacteristicSpecification.name`/`valueType`, `RelatedParty.role`,
  `ProductOfferingTerm.name`, and two `relationshipType` members). Each took its
  requiredness from a `_FVO` create schema instead of the looser base schema, so
  a conformant read payload failed to parse. Caught by the TMF632 fixtures.
- Ref-target markers moved from `product::common` to `core` (`core::refs`), so
  the party and customer domains can reference `Place`, `Account` and the rest
  without depending on `product`.
- Removed the `service` and `resource` features: they were listed as defaults
  but had no modules behind them.
- Removed the unused `insta` dev-dependency.

### Not shipping, and why

Two features earlier plans promised are dropped rather than deferred.

- **A code generator for the long-tail APIs.** Its justification was that the
  spec-coverage gate would keep generated code honest. That is circular: the
  gate compares the model against the OpenAPI document, so checking a model
  generated *from* that document proves nothing. Separately, the modelling calls
  that matter are not mechanical — eighteen schemas collapse into two
  polymorphic Rust types, TMF622 and TMF637 share one `Product`, and TMF638 and
  TMF639 need two different `Feature`s. A schema-by-schema generator gets all
  three wrong. The crate covers the APIs someone models properly, not all
  ninety-six.

- **An `mcp` feature.** Nothing would verify it — every other surface here is
  checked against a vendored document, and an MCP binding has none. It is also
  a product decision (which tools, which scopes, whose credentials) that a
  library should not make for its users, and the `schemars` feature already
  supplies everything needed to build one outside the crate.

### Notes

- Targets **TMF v5 only**. There is no v4 compatibility mode and no build flag
  that changes what a type means. This is why **TMF641 Service Ordering** and
  **TMF688 Event Management** are absent: neither has a v5 release, and the
  crate does not pretend otherwise. TMF638 references service orders, so
  `service::RelatedServiceOrderItem` exists; a TMF641 client does not.
- `service::Feature` and `resource::Feature` are **different types with the same
  schema name**. TMF638 constrains a feature with a `ConstraintRef`, TMF639 with
  a `PolicyRef`, and their `FeatureRelationship`s differ further still. Merging
  them would let you set a member the server silently drops.
- `utoipa` support is planned but not shipped: it needs manual `ToSchema` impls
  for the hand-coded types, and a half-covered derive is worse than none.
- Types are `#[non_exhaustive]`, so a TMF minor release adding a member is not a
  breaking change. The cost is that struct-update syntax (`..Default::default()`)
  does not compile downstream: use `T::builder()`.
- The `server` layer is what the `mock` runs on: `MockTmfServer` is
  `TmfHandler<MemoryStore>` plus a `Transport` shim. The mock is not a second
  implementation that can drift from the real one.
- Requiredness binds where the client *authors* a payload and relaxes where it
  *parses* one, so `…Create` and `…Update` enforce the schema's required set
  while nested types inside a response do not. `tests/coverage.rs` checks the
  division against the OAS.

[Unreleased]: https://github.com/hupe1980/rutmf/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/hupe1980/rutmf/releases/tag/v0.1.0
