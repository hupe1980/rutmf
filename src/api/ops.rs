//! The operation surface a client exposes, one macro per HTTP operation.
//!
//! [`resource_ops!`] instantiates the usual five — list, get, create, patch,
//! delete — at once; [`task_ops!`] and [`readonly_ops!`] cover the collections
//! that declare fewer, and the individual macros compose anything else. See the
//! [`api` module documentation](crate::api) for why a client's surface is
//! composed rather than generated wholesale.

/// `GET {path}` — one page of a collection.
#[allow(
    unused_macros,
    reason = "no per-API client is enabled in an `api`-only build"
)]
macro_rules! op_list {
    ($path:expr, $read:ty, $name:ident, $doc:literal) => {
        #[doc = concat!("Lists ", $doc, " matching `query`, one page at a time.")]
        pub async fn $name(&self, query: &Query) -> Result<Page<$read>> {
            self.inner.list($path, query).await
        }
    };
}

/// `GET {path}` repeatedly — every page, as a [`Stream`](futures_core::Stream).
#[allow(
    unused_macros,
    reason = "no per-API client is enabled in an `api`-only build"
)]
macro_rules! op_stream {
    ($path:expr, $read:ty, $name:ident, $doc:literal) => {
        #[doc = concat!("Streams every ", $doc, " matching `query`, fetching pages as needed.")]
        pub fn $name(&self, query: Query) -> PageStream<$read> {
            let client = self.inner.clone();
            paginate(query, move |request| {
                let client = client.clone();
                async move {
                    match request {
                        PageRequest::Query(query) => client.list($path, &query).await,
                        PageRequest::Follow(url) => client.list_absolute(&url).await,
                    }
                }
            })
        }
    };
}

/// `GET {path}/{id}` — one resource.
#[allow(
    unused_macros,
    reason = "no per-API client is enabled in an `api`-only build"
)]
macro_rules! op_get {
    ($path:expr, $read:ty, $name:ident, $doc:literal) => {
        #[doc = concat!("Retrieves one ", $doc, " by id.")]
        pub async fn $name(&self, id: &str, query: &Query) -> Result<$read> {
            self.inner.get($path, id, query).await
        }
    };
}

/// `POST {path}` — create, taking the v5 `_FVO` body.
#[allow(
    unused_macros,
    reason = "no per-API client is enabled in an `api`-only build"
)]
macro_rules! op_create {
    ($path:expr, $read:ty, $create:ty, $name:ident, $doc:literal) => {
        #[doc = concat!("Creates ", $doc, " (`POST`, the v5 `_FVO` body).")]
        pub async fn $name(&self, body: &$create) -> Result<$read> {
            self.inner.create($path, body).await
        }
    };
}

/// `PATCH {path}/{id}` — update, taking the v5 `_MVO` body or an operation list.
#[allow(
    unused_macros,
    reason = "no per-API client is enabled in an `api`-only build"
)]
macro_rules! op_patch {
    ($path:expr, $read:ty, $update:ty, $name:ident, $doc:literal) => {
        #[doc = concat!("Updates ", $doc, " (`PATCH`, the v5 `_MVO` body).")]
        ///
        #[doc = concat!("Pass `&", stringify!($update), "` for a merge patch — the safe")]
        /// default — or `&[JsonPatchOp]` for an RFC 6902 operation list. The
        /// remaining two v5 flavours are the explicit [`Patch`] variants.
        ///
        /// [`Patch`]: crate::api::Patch
        pub async fn $name<'a>(
            &self,
            id: &str,
            body: impl Into<Patch<'a, $update>>,
        ) -> Result<$read> {
            self.inner.patch($path, id, body).await
        }
    };
}

/// `DELETE {path}/{id}`.
#[allow(
    unused_macros,
    reason = "no per-API client is enabled in an `api`-only build"
)]
macro_rules! op_delete {
    ($path:expr, $name:ident, $doc:literal) => {
        #[doc = concat!("Deletes ", $doc, " by id.")]
        pub async fn $name(&self, id: &str) -> Result<()> {
            self.inner.delete($path, id).await
        }
    };
}

/// All five operations, for a resource whose specification declares all five.
///
/// Where it does not, compose the individual macros instead — see the
/// [module documentation](self).
#[allow(
    unused_macros,
    reason = "no per-API client is enabled in an `api`-only build"
)]
macro_rules! resource_ops {
    (
        $path:expr,
        read = $read:ty,
        create = $create:ty,
        update = $update:ty,
        list = $list:ident,
        stream = $stream:ident,
        get = $get:ident,
        new = $new:ident,
        patch = $patch:ident,
        delete = $delete:ident,
        doc = $doc:literal
    ) => {
        crate::api::ops::op_list!($path, $read, $list, $doc);
        crate::api::ops::op_stream!($path, $read, $stream, $doc);
        crate::api::ops::op_get!($path, $read, $get, $doc);
        crate::api::ops::op_create!($path, $read, $create, $new, $doc);
        crate::api::ops::op_patch!($path, $read, $update, $patch, $doc);
        crate::api::ops::op_delete!($path, $delete, $doc);
    };
}

/// List, stream, get and create — the surface of a *task* collection.
///
/// TM Forum models several operations as resources you `POST` to and read
/// back: `cancelProductOrder`, the six alarm tasks, `customerBillOnDemand`.
/// None of them defines `PATCH` or `DELETE`, so none of them gets one.
#[allow(
    unused_macros,
    reason = "no per-API client is enabled in an `api`-only build"
)]
macro_rules! task_ops {
    (
        $path:expr,
        read = $read:ty,
        create = $create:ty,
        list = $list:ident,
        stream = $stream:ident,
        get = $get:ident,
        new = $new:ident,
        doc = $doc:literal
    ) => {
        crate::api::ops::op_list!($path, $read, $list, $doc);
        crate::api::ops::op_stream!($path, $read, $stream, $doc);
        crate::api::ops::op_get!($path, $read, $get, $doc);
        crate::api::ops::op_create!($path, $read, $create, $new, $doc);
    };
}

/// List, stream and get — a collection the API only lets you read.
#[allow(
    unused_macros,
    reason = "no per-API client is enabled in an `api`-only build"
)]
macro_rules! readonly_ops {
    (
        $path:expr,
        read = $read:ty,
        list = $list:ident,
        stream = $stream:ident,
        get = $get:ident,
        doc = $doc:literal
    ) => {
        crate::api::ops::op_list!($path, $read, $list, $doc);
        crate::api::ops::op_stream!($path, $read, $stream, $doc);
        crate::api::ops::op_get!($path, $read, $get, $doc);
    };
}

#[allow(
    unused_imports,
    reason = "no per-API client is enabled in an `api`-only build"
)]
pub(crate) use {
    op_create, op_delete, op_get, op_list, op_patch, op_stream, readonly_ops, resource_ops,
    task_ops,
};
