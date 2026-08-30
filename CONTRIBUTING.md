# Contributing

Thanks for considering a contribution. This crate has a small number of rules
that are load-bearing — most of the review effort goes into these.

## The spec is the source of truth

Every type mirrors a schema in a published TM Forum v5 OpenAPI document. When
adding or changing one:

1. Read the actual OAS, not the PDF user guide and not another implementation.
   The documents live at [`github.com/tmforum-apis`](https://github.com/tmforum-apis).
2. Vendor the spec into `specs/` and its `components.examples` into
   `tests/fixtures/<api>/`, with a `_manifest.json` recording the source,
   license and version.
3. Map the type in `tests/coverage.rs`. **A type not listed there is not done** —
   see below.

If the spec and reality disagree — and they do — **the wire wins**. TM Forum's
own examples send `"@type": "Duration"` on a value object the schema gives no
`@type`, and omit `@type` on an `AttachmentRefOrValue` the schema marks
required. Model what servers send, and leave a comment saying why.

## Round-tripping is not coverage

This is the mistake the crate already made, so it is worth stating plainly.

Every entity captures unknown members in `extensions`, which means a payload
round-trips perfectly **whether or not the model understands a single member of
it**. A fixture test proves the bytes survive. It proves nothing about whether
the field exists.

That is how a v4 member name, four members the v5 schemas do not define, and
some 250 missing fields sat in this crate with a green test suite.

So there are two suites, and both are required:

| Suite | Checks | Run with |
|---|---|---|
| `tests/conformance.rs` | every vendored example parses and round-trips | `cargo test --test conformance` |
| `tests/coverage.rs` | the model against the schemas: 215 Rust types over 462 v5 schemas | `cargo test --test coverage --features schemars` |

`coverage.rs` reads the vendored OpenAPI documents directly and compares them
against the `schemars`-generated schema of each Rust type — so it checks the
actual serde encoding, not a transcription of it. It asserts:

1. every specified member has a typed field;
2. every typed field is specified — this is what catches a v4 name;
3. every member has the **shape** the spec gives it — comparing names alone
   would not catch a `String` where the spec says `array`;
4. every enumeration admits exactly the specified values;
5. every enumeration in the model is paired with a specified one;
6. requiredness matches, under the rule below;
7. the discriminator is always on the wire, and every type captures unknown
   members;
8. every `@type` value is the one the schema's own `discriminator.mapping`
   names, and every `…Ref` class the model claims is one a specification
   defines;
9. **the mapping covers every type the model declares**, and the fourteen
   specifications agree about the types this crate models once;
10. **every schema the documents declare is modelled or excused** by name in
    `NOT_MODELLED` — the reverse direction, and the one that makes "is the model
    complete?" answerable;
11. every addressable resource — anything with a `tmf_entity!` — has a
    `resolvable!` entry, so `Ref<T>::resolve` compiles for it.

Points 9 and 10 are what keep the rest honest: a type nobody mapped is a type
nobody checked, and a schema nobody modelled is a gap nobody sees.

### Figures are asserted, never remembered

Every quantity the documentation quotes — the number of vendored examples, of
mapped types, of schemas, of listener endpoints, which APIs expose
`GET /hub/{id}` — is a constant in a test. `TOTAL_FIXTURES` and the per-API
counts live in `tests/conformance.rs`; `MAPPED_TYPE_COUNT` and
`MAPPED_SCHEMA_COUNT` in `tests/coverage.rs`.

Growing them is expected. The constants only insist the change be deliberate,
and that the prose move with it — `README.md`, `src/lib.rs` and `site/` all
quote these numbers. An unchecked number in a crate selling provable fidelity
decays at exactly the speed of the project.

Types are keyed by `module::Struct`, not by bare name. Three modules declare a
`RelatedPlace` and two a `Feature`: when the key was the bare name, the gate
collapsed each group into one entry and checked the wrong type against the wrong
schema without saying so. If you add a type whose name already exists elsewhere,
this is why nothing breaks.

Point 9's second half is also how you find out that two APIs disagree. When it
fires on a shared name, the answer is usually **two types, not one** — TMF638
and TMF639 both declare `Feature`, and they are different schemas. Do not merge
them; a shared type would let a caller set a member the server drops.

### Polymorphic families

Where v5 has an abstract base with `@type`-discriminated subclasses — five for
`ContactMedium`, thirteen for `Characteristic` — this crate models the family as
one struct carrying the union of their members. Map such a type to the **list**
of schemas it unions, base first:

```rust
[
    "ContactMedium",
    "EmailContactMedium",
    "PhoneContactMedium",
    // …
] => party::ContactMedium,
```

The checks then run over the union, so a member a patch release adds to
`PhoneContactMedium` is required rather than merely tolerated. Do not add an
exception for a subclass member; add the subclass.

## Rules that are not negotiable

**Round-trip fidelity.** Decoding then re-encoding any payload must preserve
every member by value, including ones with no typed field. Practically:

- every entity carries `#[serde(flatten)] extensions: Extensions`;
- list members are `Option<Vec<T>>`, never `Vec<T>` — an explicitly empty array
  must stay distinct from an absent one;
- numeric members that may be integral use `crate::core::decimal_opt`, so `50`
  does not come back as `50.0`;
- a parse must not fail on an unknown `@type`; add a catch-all instead.

**The layering.** `core` and the domain modules are pure data — no I/O, no
async, no HTTP. `core` must keep building for `wasm32-unknown-unknown`. Domain
modules must not depend on `api`.

**Create / update / read stay separate.** Mirror the v5 `_FVO` and `_MVO`
schemas: members the spec requires on create are non-`Option` on the create
type; server-owned members are absent from the update type. Do not collapse
them into one struct for convenience.

**Requiredness binds where the client authors the payload, and relaxes where the
client parses one.** This is the single rule most likely to be got wrong, and it
is now checked by `coverage.rs` rather than by review.

So the `@required` section appears on `…Create` and `…Update` types only. A
nested type inside a `GET` response is something a server hands you, and
refusing to parse an entire catalogue because one relationship omitted its `id`
serves nobody — even though the base schema marks it required.

This has broken the build more than once in the other direction too:
`Attachment.attachmentType` was made mandatory because the *create* schema
requires it, and every read fixture stopped parsing.

**Nothing is invented on the way out.** A payload that omits the spec-mandatory
`@type` must come back without one; read the class through `type_name()`
instead. Middleware must not add members to what it relays.

**Credentials do not follow a server-chosen origin.** The transport attaches
auth to whatever URL it is handed, so anything that follows a URL *from a
response* must say what it is doing. Pagination links are origin-checked;
`get_absolute` deliberately is not, because cross-API `href` resolution is its
whole purpose, and it says so.

## Declaring a type

Types are declared with `tmf_struct!` (see `src/core/macros.rs`), which
generates the serde and builder attributes and the `@type` / `@baseType` /
`@schemaLocation` / `extensions` tail. One line per field:

```rust
tmf_struct! {
    @name = "ProductOffering", @ref = "ProductOfferingRef";
    /// A product offering.
    pub struct ProductOffering {
        /// Server-assigned identifier.
        id: String,
        /// Prices at which the offering is sold.
        product_offering_price: Vec<ProductOfferingPrice>,
    }
}
```

Fields are `Option<T>`, camelCased, omitted when `None`, and take `impl Into<_>`
in the builder. The `@required`, `@decimal` and `@renamed` sections change that;
the macro's own documentation has the details.

The field *lists* are still written out once per variant. That is deliberate: a
`_MVO`'s documentation genuinely differs from its `_FVO`'s, and a macro clever
enough to merge them would be a token-tree muncher nobody could debug.
Correctness of the lists is `coverage.rs`'s job.

## Editor setup

Most of this crate lives behind non-default features — the client layer, the
mock server, the schema derives. An editor analysing with default features only
will report `rutmf::api` and `rutmf::mock` as unresolved in the examples and
integration tests, which is confusing and not a real error.

`.vscode/settings.json` is committed and sets
`rust-analyzer.cargo.features = "all"` to avoid that. For other editors, point
rust-analyzer at all features the same way.

## Before opening a PR

```console
cargo fmt --all
cargo clippy --all-features --all-targets     # must be silent
cargo test --all-features                     # includes conformance and coverage
cargo test                                    # a default checkout must pass too
cargo test --no-default-features --features product   # the model alone must build
```

Every feature slice in `.github/workflows/ci.yml` is built with
`RUSTFLAGS=-D warnings`. A build with `api` or `server` but no per-API client
leaves a declaration macro unused, which is a warning and therefore a failure —
if you add a macro, give it a scoped `#[allow(unused_macros, reason = "…")]`
rather than widening the lint.

If you touched `site/`:

```console
cd site && zola check && zola build
```

Examples declare `required-features` in `Cargo.toml`, so a default build skips
them rather than failing. Add that declaration whenever you add an example that
needs the client layer — forgetting it breaks plain `cargo test`.

Public items need doc comments (`missing_docs` is `deny`), and a doc example
wherever the usage is not obvious. Examples are compiled as tests, so they stay
correct.

## Adding a new API client

1. Add the domain types under a domain module, with the three-variant treatment
   for top-level resources. Reference targets defined by *other* APIs go in
   `core::refs`, not in the domain module.
2. Add `src/api/tmfXXX.rs` using the `resource_ops!` macro for the CRUD surface,
   and `impl HubOps` so event subscriptions work uniformly.
3. Add an `api-tmfXXX` feature gating both, and list it in `.github/workflows/ci.yml`
   so the feature builds alone.
4. Add the resources to `api::resolve`'s `resolvable!` list so `Ref<T>::resolve`
   works for them.
5. Vendor the spec into `specs/` and its examples into `tests/fixtures/tmfXXX/`,
   add the API to `APIS` in both `tests/conformance.rs` and `tests/coverage.rs`
   — with its exact example count, and updating `TOTAL_FIXTURES` — and add the
   drift check to CI. If TM Forum's fixture names do not fit the classification
   rules in `conformance.rs`, extend the rules — do not add an exception,
   because an exception is how a fixture stops being tested.
6. Add every type to the mapping in `tests/coverage.rs` — the suite fails until
   you do, and its output *is* the to-do list. For a polymorphic family, map the
   whole family.
7. Add end-to-end tests against `MockTmfServer`. The mock routes by the API
   version segment, so it needs no per-collection registration.
8. Update the coverage table in `README.md` and in
   `site/content/docs/coverage.md`, and the API list on the landing page
   (`site/content/_index.md`).

Nothing in this list is about the server layer: `TmfHandler` is generic over
collections, so a new API is servable the moment its types exist. If you find
yourself adding a per-API branch to `src/server/`, that is a sign the change
belongs in TMF630's semantics instead — or that the API genuinely is not shaped
like the others, which is worth saying out loud in the PR.

## Commit and PR style

Explain *why* in the commit body, especially when the reason is a spec detail —
those are the comments that stop the change being undone six months later.

## Releasing

`## [Unreleased]` is where entries live during development; it becomes the
version heading at the moment you release, which is one command:

```console
scripts/prepare-release.sh 0.1.0
```

That closes the Unreleased section under `## [0.1.0] - <today>`, opens a fresh
empty one, updates the changelog link references, and bumps `Cargo.toml` and
`Cargo.lock`. Commit it, merge, then tag the merge commit `v0.1.0` and push the
tag.

`.github/workflows/release.yml` takes it from there: it checks the tag against
the manifest and the changelog, re-runs fmt, clippy and the suite on the tagged
commit, publishes to crates.io, then cuts a GitHub release whose notes are the
changelog section.

The publish step runs in the `crates-io` environment, which holds the
`CARGO_REGISTRY_TOKEN` secret and is where a required reviewer belongs — a
version on crates.io is permanent, and a yank does not free the number. Run the
workflow manually with a tag to rehearse everything up to the publish.

A tag with a pre-release identifier (`v0.2.0-rc.1`) is marked as one, so it does
not become the latest release.

## Conduct

Be decent to each other. Disagreements about design are welcome; making it
personal is not.

## License

Contributions are dual-licensed under MIT and Apache-2.0, matching the crate.
