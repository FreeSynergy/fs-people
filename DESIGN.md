# fs-people – Design

`fs-people` is the people program of FreeSynergy. This repo holds the **program core** (`crates/fs-people-core`, no GUI), its **server** (`crates/fs-people-server`), its **client** (`crates/fs-people-client`) and its frontend (`crates/fs-people-app`).
A person is a **JSContact `Card`** (RFC 9553) inside an AS2 **`Profile`** envelope, stored through `fs-store`
(ADR 0014, 0019, 0020). Every program is at least a workbench (ADR 0037); the work surface is the workbench `people` in `fs-components`, which knows neither this core nor its types; `fs-people-app` connects the two.

## State

Version 0.3.0: provider of the `people` contract (`people.capnp` in `fs-interfaces` v0.32.0, draft, consumers: 0 of 2), served over a Unix socket. Built on fs-store 0.2 (git tag).

| Area | State |
|---|---|
| Typed `Card` (serde) with lossless unknown/vendor properties (`extra` maps on the card and every sub-object) | done; all 20 demo cards round-trip byte-equal as JSON |
| Validation (RFC 9553: `@type`, `version` "1.0", `uid`, `kind` enum or `<domain>:<value>`, UTC date-times, required members of sub-objects, `members` only for groups) | done |
| Contract `People` (trait, object-safe) + store-backed `PeopleRepo` (create / update / delete / get / get_by_uid / list / search / groups / links / import / export) | done; `PeopleRepo` depends on `dyn ObjectRepository` + `dyn ObjectQuery` only (narrow fs-store traits) |
| Envelope derivation: type `Profile`, `name` (ADR 0032: `name.full`, else components, nickname, org, e-mail, uid), `context=people`, IRI `https://<instance>/people/<id>`, `sensitive` from the card | done |
| Index fields: `uid`, `kind`, `name_sort`, `email` (lowercase), `phone` (E.164-ish), `nickname`, `org` (lowercase), `member` | done; `describes` moved to the envelope (fs-store 0.2) |
| Envelope `describes` (AS2 `Profile.describes`) from the vendor property `freesynergy.net:describes`; `find_by_describes` | done |
| Search by name / nickname / e-mail / phone / organisation (substring, case-insensitive) | done **in the store**: one query with an OR-group of `Op::Contains` (name, `name_sort`, nickname, e-mail, org, phone digits); Unicode case-insensitive, `%` `_` literal |
| Listing / ordering | `list` ordered by the `name_sort` index ("surname, given") case-insensitively, limit/offset in the store (no Rust sort) |
| Formats behind `Importer` / `Exporter` traits (`VCardFormat`, `JsContactFormat`); `PeopleRepo::import(&dyn Importer, ..)` / `export(&dyn Exporter, ..)` | done |
| vCard 4.0 import/export via `calcard` (also reads 2.1/3.0); original kept per card (`text/vcard`, ADR 0030); re-import skips known `uid`s | done |
| Groups / folders: `kind: group` + `members` (ADR 0037); add/remove member, resolve members, groups of a card | done |
| Same-person proposals (ADR 0019/0029) as a `MatchStrategy` (default `SharedContactPoint`, swappable with `with_match_strategy`): shared normalised e-mail or phone, same `kind`, never for sensitive cards, groups, or already linked pairs; confirm creates an AS2 `Relationship` (`owl:sameAs`); unlink; deleting a card removes its links | done |
| Sensitive marking: vendor property `freesynergy.net:sensitive` (DPV category list) on the card or any nested object -> `envelope.sensitive` | done; encryption itself is `fs-store`'s job (it refuses sensitive objects in an unencrypted store, tested) |
| Example `cargo run -p fs-people-core --example demo`: import demo contacts, list, search, export vCard | done |
| Interface `People` + `RoleBook` (provider rule, ADR 0045): every method exists, what a provider does not override answers `Error::Unsupported`; `capabilities()` reports the same | done, draft |
| Roles (ADR 0052): one card, several roles. Definitions are objects: configuration (`roles/known.json`, swapped atomically after checking, `load_role_config`) and your own (`define_role`, in the store). No role list in the code | done |
| Role assignments: AS2 `Relationship` objects of their own (index `person`, `role`, `scope`), never on the card, so no export reveals them (ADR 0029) | done |
| Relations (what someone is to me): JSContact `relatedTo` on the card, `relate` / `unrelate` / `relations` | done |
| AI marking (ADR 0054): `mark_ai` = `kind: application` (RFC 9553 §2.1.4, travels with the card) + role `ai` (plain software is `application` too). Restricts nothing | done |
| Server `fs-people-server`: every method of `People`, `Persons`, `Roles`; `subscribe` (with `ended`), `supportReport`; typed problems (`people/not-found` 404, `people/conflict` 409, `people/invalid` 422, `people/sensitive` 403 (release missing), `people/sensitive-link` 403 (sensitive card is not linked), `people/encryption-required` 409, `people/stored-data` 500, shared `invalid-request` 422, `too-large` 413, `unsupported` 501); binary with `--socket`, `--data`, `--actor` | done, draft |
| Client `fs-people-client` (`RpcPeople`): `People` and `RoleBook` of the core over the socket of `people.capnp`; the connection lives on a thread of its own; problems of the server become the variants of `fs_people_core::Error` again | done, draft; what it leaves `Unsupported` is under *Client* below |
| Live feed: `PeopleFeed`, `PersonListener`, `PersonChange`; cards only, never roles or links | done |
| `list`: kind, role and term combine with AND (`ListFilter`) | done |
| Writes through the traits take turns (`writes` in `PeopleRepo`), so a read-modify-write is not lost when the backend yields; the inherent methods do not, and compose each other | done |
| Limits: a card is at most `MAX_CARD_BYTES` as JSON (also after small edits), every stored text passes `check_intake`, every answer without a cursor `check_answer`; the report of an import keeps `created` whole and cuts `skipped` and `failed` to fit, with a last entry "N more" | done |
| A write of the server runs to its end, also if the client goes away (`run_to_end`) | done |
| Numbers: what plain `serde_json` cannot keep exactly is refused at intake with a clear 422 (an integer beyond `i64` / `u64`, a float whose value does not come back, `-0`): cards, `details`, a role configuration when stored (not when restored, so a stored one cannot stop a start); in an import, per item in `failed`, also one that no parser reads such as `1e400`. A `-0` an older card already holds blocks no edit. `validate/numbers.rs` reads the text, as the parser changes the number. `serde_json` has `float_roundtrip`, without which the parser can be one step off a double (6 % of random ones). The feature `arbitrary_precision` was tried and removed: features unify across the build, it broke vCard export and undo, and other readers could not read the result | done |
| Role configuration survives a restart: `store_role_config` keeps it in the store, `restore_role_config` puts it back at start | done |
| Tests | 109 in the core, 49 for the server (the client has 28 and the app its own: `crates/fs-people-client/tests`, `crates/fs-people-app/tests`, all against the real server on a real socket over the real store): over a real socket against the real store, one of them the real binary across a restart. Two small providers beside the real one, only for what it cannot show: the provider rule (a read-only source) and a feed whose reads fail |
| ADR 0043 refactor | done: every file <= 200 lines (`check_rust_rules.py`: 0 errors, 0 warnings), `//! Pattern:` in every module, `#![deny(missing_docs)]`, `#![warn(clippy::pedantic)]` clean |

## Frontend `fs-people-app` (first version, provisional)

Imports no `blinc_*` crate (ADR 0009). Arguments: `--socket <path>` (required: the socket of a running `fs-people-server --socket <path>`), `--import <file.vcf>`, `--export <file>`, `--snapshot <png>`, `--size WxH`.

**Over the socket (since the client exists):** `PeopleAdapter` holds `Arc<dyn People>` and `Arc<dyn RoleBook>`, and `main.rs` puts an `RpcPeople` (`fs-people-client`) behind both. The app starts no server and opens no store: without `--socket` it stops and names `fs-people-server --socket`; `--store` is refused with a pointer to the server's `--data`. The actor is the server's `--actor`, no longer fixed in the app. `transfer.rs` (import, export) goes through the same client.

Dependencies of the app: `fs-people-core` only for types and traits (`Card`, `People`, `RoleBook`, `ListFilter`, `Error`, `VCardFormat`, `card_of`), `fs-store` only for the types `ObjectId` and `StoredObject` and the variant of `Error::Store` it reads; neither is used to hold or open data. The edit logic on a `Card` (`source/apply.rs`, `marks.rs`) stays in the app: it works on the JSON of the card the server sent.

Known limits: every call still blocks the interface thread until the server has answered (the adapter calls `block_on`), so a slow server freezes the window as before; there is no `subscribe` in the app, a change by someone else shows after the next reload only.

| Area | State |
|---|---|
| List (a page of 200 with a Show more button), search, read as sections, marks for an AI and for sensitive data (ADR 0029, 0054), roles read only | done |
| Create and edit name, e-mails, phones, organisation; delete with a second press | done |
| Editing keeps everything else: the form goes onto the card as stored now, and only fields that differ from what the form started with are written, each to the entry its key names. Proved in `tests/keeps.rs` by comparing the whole card as JSON | done |
| vCard import at start; a store refusing a sensitive card is shown as a problem in the status line and on stderr. On this core the import stops there and the people before it are in; `people-core-fixes` makes it one failure among the rest, and the same code then reports it under `failed`. `ImportReport::unclear` of that branch is not shown yet | done, follows the core |
| vCard export of everyone with `--export`, written through a short-named temporary file beside the target, synced, then linked into place, never over an existing one (without hard links it is copied, and a failed copy removes what it made), then the program stops (with `--snapshot` it draws as well). `--export-sensitive <uid>`, repeatable, releases one named person marked sensitive; naming nobody writes nothing | done |
| Marks: someone sensitive is marked in the list, in search, in the compact variant and in the form, and is opened only when chosen (ADR 0029 point 4) | done |
| Windows | `launch` is wired, never opened by tests; `AppWidget::build` and `picture` are drawn offscreen and compared |

Decisions (provisional, easy to reverse):

* **A changed name replaces the name's parts** (`components`, `isOrdered`, `defaultSeparator`): left in place they would keep the old name in sorting, search and vCard `N`. `name.extra` stays, also a `sortAs` in it, which may then be out of date. An unchanged name stays as it is. An emptied name loses `sortAs` and the phonetic members with it; a name holding anything else unknown can be replaced but not emptied, and the form says what it holds.
* **An edit never reduces a card's sensitive marking:** what the removed part carried moves to card level, a marking that names no category as it came, so it stays unclear and no category is invented. Beside named categories it stays as a non-string member of the list. Whether someone is marked is decided by the property, never by text: a note that mentions it marks nobody. `source/marks.rs` walks the card itself because main's core only knows categories; with `people-core-fixes` merged it becomes `Card::is_sensitive`.
* **One rule for an empty card:** Save and Create both refuse a card left with no name, nickname, e-mail, phone or organisation with a name (units alone would only show the `uid`).
* **An emptied field removes that value;** an organisation with `units` only loses its name. Additions come before removals, so an id freed by an edit is not given to something added by it, and a title lets go of an organisation that went.
* **Conflict:** a revision is the store's `updated` plus a fingerprint of the content. Compare and write are not atomic, because `People::update` takes no revision: **wanted from the core: compare-and-set**. After a conflict the second Save writes only the fields the person changed, onto the other's version.
* **`--export` leaves out people marked sensitive and names them with their `uid`** (ADR 0029: they leave the device only by explicit release, which is `--export-sensitive <uid>`, one person at a time). A marking in any form counts, also one with no category we can read. It never replaces a file, and it stops without a window, so it works in a script.
* **`--import` skips known `uid`s** (the core's rule); a file that cannot be read or holds no vCard ends the program with an error rather than opening without the people.
* The actor is a fixed `https://localhost/actors/owner`: there is no sign-in yet.

Unsupported in this version (nothing in the interface pretends otherwise): groups and their members (a group card is listed and read, `members` named as kept), relations (`relatedTo` kept, not shown), same-person proposals and links, assigning roles or marking an AI, editing anything else (nicknames, titles, addresses, links, notes, keywords), setting the sensitive marking, field-level release of sensitive data, a file dialog for import and export, undo, live updates (`person.subscribe`), a draft surviving a closed window, an encrypted store.

To do when `people-core-fixes` reaches main: show `ImportReport::unclear` in the import summary (`transfer.rs`); answer `PeopleSource::delete` with `Ok(Some(remark))` for the core's `PartlyDeleted`, so the screen says the person went and what stayed; replace the walk in `source/marks.rs`.

Wanted from the core:

* `People::search` has no paging: the whole result is read for every page of the list.
* `People::list` reports no total, so the list shows Show more instead of a scrollbar over everyone.
* `import` stops at the first store error and loses the report of the people before it (they are stored; the summary cannot count them). Fixed on `origin/server`, not on main yet.
* `export_vcard` keeps `VERSION:3.0` for a card imported from vCard 3.0 (seen in a real export), while its documentation says 4.0.

## Client `fs-people-client`

`RpcPeople::connect(socket)` blocks until the server has answered `supportReport`, then holds the connection on its own thread. Dropping it closes the connection and joins the thread.

Implemented, exactly what the app uses: `People::{capabilities, get, list, create, update, delete, search, import_vcard, export_released}` and `RoleBook::{role, roles_of}`. **Everything else answers the `Unsupported` of the provider rule** (the trait defaults, not overridden): `People::{get_card, get_by_uid, find_by_describes, find_by_email, find_by_phone, import, export, relate, unrelate, relations, create_group, add_member, remove_member, members, groups_of, find_same_person_proposals, confirm_same_person, unlink, linked_to, import_jscontact, export_vcard}` and `RoleBook::{define_role, undefine_role, load_role_config, roles, assign_role, unassign_role, people_with_role, people_with_personal_role, mark_ai, is_ai}`. (The server offers most of them; the client does not write what no frontend calls yet.) `capabilities()` is the server's report cut to what the client implements (`offered.rs`): what the server offers and the client does not call (`person.subscribe`, `relation.*`, `cardgroup.*`, `same.*`, `role.define`, `role.configure`, `role.assign`, `ai.mark`) is reported as not possible, with the reason. A capability the client keeps in part stays possible (`person.read` without `get_by_uid`, `person.find` as `search` only, `person.import` as vCard, `person.export` without the plain `export`, `role.read` without `roles`); the methods listed above still answer `Unsupported`.

What does not arrive over the wire, and what the client does about it:

* **`StoredObject`** is built from the `Person`: the envelope via `read_envelope`, the JSContact text parsed as `content`, `content_type` `application/jscontact+json`. `original` (ADR 0030) and `index` are `None` / empty: the contract has no place for them. `fs-store` has no revision field; the app's revision is `updated` plus a fingerprint of the content, and `updated` comes in **milliseconds** (the wire's resolution), so two writes within a millisecond are told apart by the fingerprint alone.
* **`ListFilter.limit` / `offset`** do not exist in the contract. The client reads pages (cursor) until the cursor is empty or `offset + limit` people are there, then cuts. A deep `offset` therefore reads the pages before it, and the server cuts each page from a whole list.
* **`search(term)`** is `list` with `term`: the contract has no search call.
* **Reads per screen:** `export_all` makes one `list` without limit (the client walks the cursor pages). The window's `rows()` asks for `offset` and `limit` and so reads the pages up to `offset + limit` again on each "Show more", and `search` reads all of its result for every page: a cost of the contract (no offset, no paging of search at the server), not removed here.
* **Deadlines:** `connect` waits 5 s (`CONNECT_TIMEOUT`), each call 30 s (`CALL_TIMEOUT`), both changeable with `connect_with(socket, Timeouts)`. The error names the deadline (`people server did not answer within 30s`). A write that timed out may still have happened: the server finishes a write also if the caller goes away. A job that panics answers `people client: a call ended without an answer`, not "connection is gone".
* **`capabilities()`** is a snapshot taken at `connect`; the server is not asked again.
* **Errors** are read back from type, status and text of the problem, because the wire carries no fields. Exact: `not-found` (the id comes from the call, `UnknownRole` for a role), `conflict`, `unsupported`, `stored-data`, `partly-deleted`, `sensitive`, `sensitive-link`, `sensitive-members`, `encryption-required` (the id is read from the text), `too-large` (the first number of the text; 0 if none). `invalid` is read back by the text of its variant: `Error::Invalid` issues, `InexactNumber`, `Json` (message only, no position), else `Error::Store(Invalid)` without a second wrapper. `people/sensitive` (`NotReleased`) and `people/sensitive-link` (`Sensitive(id)`) are two types of the server and are told apart by the type alone. A transport failure (server gone) is `Error::Store(Backend("people server: ..."))`, as there is no variant for it. A text format change on the server breaks the reading back; the table is tested (`problems_table.rs`) and the common types over the real socket.
* **`RoleDefinition.extra`** has no place on the wire (a gap of the contract, see below).


## Structure and pattern map (ADR 0043)

Public contract: the trait `People`; `PeopleRepo` is its implementation (Repository + Facade) and keeps all
its inherent methods, so callers that do not import the trait keep working. Every file is one concept, <= 200 lines
(tests included); `lib.rs` and every `mod.rs` only declare and re-export.

| Module | Pattern | Why |
|---|---|---|
| `fs-people-client/peer.rs`, `people_impl.rs`, `roles_impl.rs` | Adapter | `People` and `RoleBook` of the core in front of the Cap'n Proto client; the frontend depends on the traits, not on the wire. Interface first: `connect` was committed before the calls and the tests |
| `fs-people-client/worker.rs` | Command | capnp-rpc clients are `!Send`: a call becomes a `Job`, a command built on any thread and executed only by the thread that owns the connection (current-thread runtime and `LocalSet`); the caller waits on a `oneshot`, calls overlap there. `reply.rs` is an RAII guard so a job that dies says so |
| `fs-people-client/calls/*`, `read.rs` | Adapter | one request and one reply read back as values of the core (`StoredObject`, `RoleAssignment`, `RoleDefinition`, `ImportReport`) |
| `fs-people-client/problems.rs` | Adapter | the inverse of `errors.rs` of the server: RFC 9457 type and status to the variant of `Error` |
| `fs-people-app/source/*`, `transfer.rs`, `main.rs` | Adapter / Template method / Facade consumer | workbench `PeopleSource` over `dyn People`; import and export flows; wiring to the socket |
| `fs-people-server/{people_service,roles_service,persons/*}.rs` | Adapter + Factory | wire in, one trait call, wire out; depends on the traits only. `persons/service.rs` is the dispatch, `cards`, `links`, `lists` behind it |
| `fs-people-server/wire/*`, `errors.rs` | Adapter | values on the wire, limits of a message, core errors as RFC 9457 problems |
| `fs-people-server/formats.rs` | Registry | media type to format of the core; an unknown type is `Unsupported` |
| `fs-people-server/feed.rs`, `feed.rs` (core) | Observer | `PeopleFeed` to the shared forwarder; a read error ends the feed with `ended` |
| `repo/role_config.rs` | Memento | the last accepted configuration as one object in the store |
| `repo/role_facade.rs` | Facade | `RoleBook` and `PeopleFeed` over the inherent methods, writes take turns |
| `role_book.rs`, `repo/{roles,assign}.rs` | Repository (interface + implementation) | role definitions and assignments, split from `People` so a provider of cards alone still fits |
| `roles/registry.rs` | Registry, swapped atomically | configuration plugin after Zentinel (ADR 0045): checked first, readers see old or new, never a mix |
| `roles/{definition,assignment}.rs` | Value objects | the same form for shipped and own roles |
| `card/relations.rs`, `repo/relations.rs` | Command | small edits of `relatedTo` |
| `people.rs` | Repository (interface) + Null Object | the program's public contract; GUI, hub, server and tests use `dyn People` |
| `repo/handle.rs`, `repo/mod.rs` | Repository + Facade | one small type hides store, envelope rules, formats and matching |
| `repo/backend.rs` | Adapter | `Arc<dyn Store>` presented as `dyn ObjectRepository` + `dyn ObjectQuery` (narrowest store traits) |
| `repo/{crud,find,groups,links}.rs` | Repository | one concern each: writes, queries, groups, `owl:sameAs` links |
| `repo/envelope.rs` | Adapter | card to AS2 `Profile` envelope + index fields (ADR 0020, 0032) |
| `repo/transfer.rs` | Template method | fixed import flow (parse, skip known uid, validate, store, report); parsing is the `Importer` |
| `repo/proposals.rs`, `matching/*` | Strategy | `MatchStrategy` decides who looks alike (`SharedContactPoint`); the repository keeps the ADR 0029 policy |
| `format/*`, `vcard/*` | Adapter (+ Strategy) | `Importer` / `Exporter` traits; vCard (calcard) and JSContact JSON behind them, more formats plug in |
| `card/*` | Value objects / Template method (macro) | typed JSContact with lossless `extra`; `sub_object!` fixes the shared skeleton |
| `card/display.rs` | Chain of responsibility | display name fallback chain (ADR 0032) |
| `card/vendor.rs` | Visitor | recursive search for the sensitive marking (ADR 0029) |
| `index/*` | Strategy (pure functions) / Adapter | one canonical normalisation for indexing and querying; card to index fields |
| `validate/*` | Specification | RFC 9553 rules reporting into one `Sink`; all issues at once |
| `error.rs`, `clock.rs` | none | plain error enum, pure helper |

## Findings of the review (2026-11-06), on branch `people-core-fixes`

* **Identity of a `uid`.** `index::normalise_uid` gives one text for every spelling of a UUID (`urn:uuid:` or not, any case, hyphens or not); any other `uid` stays as it is. The index field `uid_key` holds it, and `get_by_uid`, `create` and `import` compare on it, so another spelling is a conflict or a skip, never an overwrite or a second person, also for UUIDs that are not v7 (which do not become the object id). Cards stored before `uid_key` existed need no migration: a search over `uid` finds them whatever their case, and the result is checked against the identity. `get_by_uid` goes by the `uid` alone; a card whose object id equals the `uid` asked for but has another `uid` is not returned, and `create` and `import` still refuse to overwrite it.
* **A sensitive marking that names no category fails closed.** `Card::is_sensitive` is true for any marking at all. `create` refuses one that is not a list of names; an import keeps the card as it came, names it in `ImportReport::unclear`, and the envelope carries `freesynergy.net:unclear` (not a DPV category) so the store and every reader treat it as sensitive. A plain store refuses it, which the report gives as failed with its reason; the file is still there, so nothing is lost. `update` does not refuse it, so such a card can still be corrected.
* **Delete.** The store has no transaction over several objects. The order is: off every list, as only the card knows its `uid`; then the card; then links and roles. A write goes to its end also if the client goes away (`run_to_end`). What stays possible, because a store call fails or the process dies between two steps: (1) the person is still there, off some lists: `people/partly-deleted` (500) says so if a list changed, a plain store error if nothing did. (2) The card is gone and links or roles remain: `people/partly-deleted` says so; until the same call again a link or role names a card that is gone, and `create` refuses that id. After a crash nobody is told, and nothing repairs it at the next start.

## Migration for dependents

* `PeopleRepo::store()` is removed: the repository no longer holds the whole `Store`, only the narrow traits. Keep your
  own `Arc<dyn Store>` for undo / log / count calls (the old tests did `r.store().undo(..)`).
* `vcard::ImportedCard` moved to `format::ImportedCard` (re-exported at the crate root); `original` is now `Option<Blob>`.
* `Card`, `PeopleRepo::new/with_instance/create/get/update/delete/list/search/...`, `ListFilter`, `ImportReport`, `MatchKind`,
  `MatchReason`, `SameAsProposal`, `validate::*`, `vcard::*` keep their paths and signatures; fs-hub compiles unchanged.
* New: `People`, `Importer`, `Exporter`, `MatchStrategy`, `PeopleRepo::{with_backend, with_match_strategy, import, export}`.
  If you import `People` next to `PeopleRepo`'s inherent methods nothing is ambiguous (inherent methods win).

## Crates

* `fs-store` (git tag v0.2.0). `fs-interfaces` (git tag v0.32.0): `Capabilities`, `Unsupported`; the server also takes the contract and `rpc` from it.
* `calcard =0.3.14` (Apache-2.0 OR MIT): vCard <-> JSContact following RFC 9554. **Pinned exactly**: Stalwart moved later
  versions to AGPL, which we must not use. Pulls `jmap-tools` (Apache-2.0 OR MIT), `chrono`, `mail-parser` etc., all permissive.
  Its JSON model is separate from ours: we exchange JSON text at the boundary and own the typed `Card`.
* `fs-components-app`, `fs-components-workbenches` (frontend only; git `rev`, provisional until a tag exists).
* `serde`, `serde_json`, `chrono` (timestamps), `thiserror`, `async-trait` (MIT OR Apache-2.0, already used by fs-store); dev: `tokio`, `proptest`.
* All transitive licences are MIT / Apache-2.0 / BSD / ISC / Unicode-3.0 / Unlicense (a few have an LGPL *alternative*, never required).

## Decisions

* **Lossless model.** Known properties are typed; everything else (vendor properties, `cryptoKeys`, `calendars`, `personalInfo`,
  the RFC 9554 `vCard` member, ...) is kept verbatim in `extra`. Property order inside objects is not preserved (JSON objects are unordered).
* **Object id = uid** when the `uid` is a `urn:uuid:` UUIDv7 (the demo data are), so re-imports map onto the same object. Otherwise a fresh UUIDv7.
  vCards without `UID` get a deterministic `urn:uuid:` (version 8) derived from the text, so importing the same file twice does not duplicate.
* **`uid` is immutable**; `update` refuses a changed `uid`. `updated` is set to now on update.
* **A card keeps what its roles ask.** `update` refuses a `kind` a held role does not allow, `markAi` refuses one another role does not allow, a new role configuration refuses to orphan or misfit a holder. An AI is `kind: application` and the role, both.
* **A list holds `uid`s.** Deleting a card takes its `uid` off every list; removal goes by the `uid` in any spelling, also for a member whose card is gone, if its `uid` was its id. `removeFromList` and `listMembers` on a card that is no list are a conflict, like `addToList`.
* **Relations are open.** `unrelate` keeps an entry that still has members of its own; `relations` shows an entry without a relation, with an empty one, and `unrelate` with an empty relation removes it.
* **A marking never shrinks.** `update` puts back what an edit removed, at card level (moved here from `fs-people-app`); an unclear marking stays until the edit names categories, and only then goes. A card that is now sensitive loses its same-person links (ADR 0029), also a link older than the marking.
* **What leaves the program (strategy decision, ADR 0029).** A sensitive card goes only on explicit release per recipient: `export` refuses it (`people/sensitive`, 403), `export_released` and `exportText` with its `uid` in `released` let it go. Adding someone sensitive to a list when asked for stays allowed; only automatic linking is forbidden. `members` carries the mark along: `Person.envelope.sensitive`. A list leaves without its sensitive members unless released, and `withheld` names them; the plain `export`, which has no report, refuses such a list by name (`SensitiveMembers`). Who counts: the envelope, or the content of an older card, or a card that does not read.
* **A delete that stopped halfway leaves nothing for the next card.** `create` refuses a `uid`-derived id with roles or links left; deleting it again clears them. Deleting a group card ends the assignments scoped to its IRI.
* **A list that does not read blocks nobody.** A member leaves a list from what is stored, without reading it as a card; `proposals` leaves out a card that does not read, and `unreadable_cards` names them. The contract has no place for that report yet. `Persons.proposals` is silent about them over the wire.
* **An import takes its turn per card**, validates before it asks the store, and keeps at most 5 MiB per list of its report; what does not fit is counted in a last entry "N more". The items are read one at a time (`Importer::import_each`).
* **A new role configuration is judged by what it changes.** Who fits nothing today, from before a rule or a role that went, does not block it. An AI is marked without a scope, whatever the role says of its reach, and that does not block it either.
* **Object id = uid, guarded.** `create` refuses a `uid` that is the id of any other object (a role assignment, a link, another card); `put` would overwrite it without a word.
* **Removals stay with their kind.** `unlink` and `unassign_role` remove only a same-person link or a role assignment; any other id answers not found.
* **Import reports everything.** A card that cannot be stored is listed in `failed` and the others go on; lines outside a complete vCard are listed as well.
* **Sensitive (ADR 0029).** The card carries categories in `freesynergy.net:sensitive`; the repo copies them to `envelope.sensitive`.
  Sensitive cards are excluded from proposals and `confirm_same_person` refuses them even for hand-made pairs.
* **Link, don't merge.** The `Relationship` object has `subject`/`object` IRIs, `relationship: "owl:sameAs"` and index fields `subject`/`object`
  (object ids) for lookup. Both cards stay. `relatedTo` is not used.
* **Groups** are normal cards of `kind: group`; `members` maps member `uid` -> `true` (RFC 9553). Folder = group = mailing list (ADR 0037).
* **vCard export** strips nested `@type` members (no vCard meaning) and relies on calcard's RFC 9554 parameters (`PROP-ID`, `JSCOMPS`, `JSPROP`)
  for lossless return. Unknown vendor properties are exported as `JSPROP`.

## Open points of the `people` interface

* **Operator of an AI (ADR 0054):** planned, the form is open. Nothing blocks it: `RoleAssignment.details` and the card's `extra` are lossless. Locking AIs out is deliberately not built.
* **Role privacy across programs:** assignments are `Relationship` objects with `to = [actor]`. A program with read access to the type `Relationship` could see them through the hub, whose rule goes by type. To be settled with `fs-groups` and the sharing interface (ADR 0029).
* **Role privacy on the server (provisional, stricter reading):** `Person.roles` holds "only what the caller may see", and the socket tells the user id, not the caller. So it is **always empty**; roles are asked for on `Roles`. Whoever reaches the socket reaches `Roles` as well, so `PersonFilter.role`, `rolesOf` and `peopleWith` answer what the contract names. When callers get an identity, all three need the same gate. A subscriber is told of cards only: an assignment or a link causes no change, and `confirmSame` hands out the id of its `Relationship`, nothing else.
* **Details of a role (provisional):** `assign` refuses `details` for a role not owned by `fs-people`: the owner of a role holds its fields (`people.capnp`). For a role already held, the same or no details answer the assignment, other details are a conflict: nothing is dropped and nothing replaced; end the assignment first.
* **Scope of an assignment (provisional):** a non-empty `scope` must be an IRI, which also keeps it apart from the internal "personal". `RoleDefinition.scope` decides: `group` needs one, `personal` has none, `public` can be either. `markAi` does not ask, an AI is one everywhere.
* **`peopleWith` with an empty scope (provisional):** means personal, as in `RoleAssignment.scope`; a non-IRI, also the literal `personal`, is refused. "Everywhere" has no query there: `PersonFilter.role` answers that, as a filter over people. If a consumer needs the ids across all scopes, the contract needs an explicit form.
* **Sensitive cards in the binary:** the binary opens an unencrypted store, so `fs-store` refuses them (`people/encryption-required`). The key provider comes with the key vault (ADR 0024).
* **Role definition format:** SKOS not read yet (strategy session); the JSON form may still get sharper before 1.0.
* **The trait returns `StoredObject` of `fs-store`:** a provider without a store has to build one. To be decided before 1.0.
* **`supportReport` is the standard** for what a provider offers (strategy decision); a method `capabilities` is not added to `People`.
* **Gaps of `people.capnp` (for the owner of the contract):**
  * `RoleDefinition` has no place for the lossless `extra` of the core: a role read over the wire loses it. It stays in what the store and a configuration hold.
  * `confirmSame` is the only way to learn the id of a link, `linkedTo` answers card ids: after a restart a consumer cannot name the link it wants to `unlinkSame`.
  * `proposals` has no place to say that a card did not read.
  * Lists without a cursor (`findBy*`, `listMembers`, `listsOf`, `relations`, `proposals`, `Roles.list`, `rolesOf`, `peopleWith`): an answer that would not fit one message is refused as `too-large`; such a list cannot be read at all.
  * The ids `person.read`, `person.find` and the others are comments in the schema; the report follows them.
* **Whole import limit:** `importText` takes at most `MAX_INTAKE_BYTES` per call, also for many small cards. Split a larger file.
* **`Registry` of the feed** has the same shape as the one in `fs-tasks-core`; it belongs into a shared crate once a third program needs it (ADR 0043 §4).
* **Group cards vs `fs-groups`:** `create_group` / `members` stay as JSContact lists of people; belonging and rights go to `fs-groups` (ADR 0050).

## fs-store 0.2 – what is used, what is still missing

Used: `order_by_index("name_sort")` (case-insensitive list order and paging), `Cond`/`Op::Contains` OR-group (search), `Cond::Describes`
and envelope `describes`, OR-group of two `Eq` conditions for the `Relationship` lookup (one query instead of two).

Still wanted:

1. **Tokenised full-text search / ranking** (substring only today).
2. The proposal finder still lists all non-group Profiles and groups by e-mail/phone in Rust; a "values shared by more than one object" query (`GROUP BY` on `index_fields`) would move that into the store.
3. Locale collation (CLDR) for ordering; the store orders by code point of the lowercased NFC key.

## Open points

* Sync token / CardDAV (RFC 6352) is not part of the core.
* Field-level `freesynergy.net:sensitive` is detected, but export/sharing filtering by field (ADR 0029 point 2) is a hub/frontend concern.
* JSContact `PatchObject` updates (RFC 9553) are not implemented; `update` replaces the whole card.
* The vCard `PHOTO`/`KEY` handling is whatever calcard does (not tested here).
* "Dismissed" proposals are not remembered; a person who rejects one will see it again.
