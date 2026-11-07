# fs-people – Design

`fs-people` is the people program of FreeSynergy. This repo holds the **program core** (`crates/fs-people-core`, no GUI) and its **server** (`crates/fs-people-server`).
A person is a **JSContact `Card`** (RFC 9553) inside an AS2 **`Profile`** envelope, stored through `fs-objects`
(ADR 0014, 0019, 0020). Every program is at least a workbench (ADR 0037); the GUI/workbench crates come later and use this core.

## State

Version 0.3.0: provider of the `people` contract (`people.capnp` in `fs-interfaces` v0.22.0, draft, consumers: 0 of 2), served over a Unix socket. Built on fs-objects 0.2 (git tag).

| Area | State |
|---|---|
| Typed `Card` (serde) with lossless unknown/vendor properties (`extra` maps on the card and every sub-object) | done; all 20 demo cards round-trip byte-equal as JSON |
| Validation (RFC 9553: `@type`, `version` "1.0", `uid`, `kind` enum or `<domain>:<value>`, UTC date-times, required members of sub-objects, `members` only for groups) | done |
| Contract `People` (trait, object-safe) + store-backed `PeopleRepo` (create / update / delete / get / get_by_uid / list / search / groups / links / import / export) | done; `PeopleRepo` depends on `dyn ObjectRepository` + `dyn ObjectQuery` only (narrow fs-objects traits) |
| Envelope derivation: type `Profile`, `name` (ADR 0032: `name.full`, else components, nickname, org, e-mail, uid), `context=people`, IRI `https://<instance>/people/<id>`, `sensitive` from the card | done |
| Index fields: `uid`, `kind`, `name_sort`, `email` (lowercase), `phone` (E.164-ish), `nickname`, `org` (lowercase), `member` | done; `describes` moved to the envelope (fs-objects 0.2) |
| Envelope `describes` (AS2 `Profile.describes`) from the vendor property `freesynergy.net:describes`; `find_by_describes` | done |
| Search by name / nickname / e-mail / phone / organisation (substring, case-insensitive) | done **in the store**: one query with an OR-group of `Op::Contains` (name, `name_sort`, nickname, e-mail, org, phone digits); Unicode case-insensitive, `%` `_` literal |
| Listing / ordering | `list` ordered by the `name_sort` index ("surname, given") case-insensitively, limit/offset in the store (no Rust sort) |
| Formats behind `Importer` / `Exporter` traits (`VCardFormat`, `JsContactFormat`); `PeopleRepo::import(&dyn Importer, ..)` / `export(&dyn Exporter, ..)` | done |
| vCard 4.0 import/export via `calcard` (also reads 2.1/3.0); original kept per card (`text/vcard`, ADR 0030); re-import skips known `uid`s | done |
| Groups / folders: `kind: group` + `members` (ADR 0037); add/remove member, resolve members, groups of a card | done |
| Same-person proposals (ADR 0019/0029) as a `MatchStrategy` (default `SharedContactPoint`, swappable with `with_match_strategy`): shared normalised e-mail or phone, same `kind`, never for sensitive cards, groups, or already linked pairs; confirm creates an AS2 `Relationship` (`owl:sameAs`); unlink; deleting a card removes its links | done |
| Sensitive marking: vendor property `freesynergy.net:sensitive` (DPV category list) on the card or any nested object -> `envelope.sensitive` | done; encryption itself is `fs-objects`'s job (it refuses sensitive objects in an unencrypted store, tested) |
| Example `cargo run -p fs-people-core --example demo`: import demo contacts, list, search, export vCard | done |
| Interface `People` + `RoleBook` (provider rule, ADR 0045): every method exists, what a provider does not override answers `Error::Unsupported`; `capabilities()` reports the same | done, draft |
| Roles (ADR 0052): one card, several roles. Definitions are objects: configuration (`roles/known.json`, swapped atomically after checking, `load_role_config`) and your own (`define_role`, in the store). No role list in the code | done |
| Role assignments: AS2 `Relationship` objects of their own (index `person`, `role`, `scope`), never on the card, so no export reveals them (ADR 0029) | done |
| Relations (what someone is to me): JSContact `relatedTo` on the card, `relate` / `unrelate` / `relations` | done |
| AI marking (ADR 0054): `mark_ai` = `kind: application` (RFC 9553 §2.1.4, travels with the card) + role `ai` (plain software is `application` too). Restricts nothing | done |
| Server `fs-people-server`: every method of `People`, `Persons`, `Roles`; `subscribe` (with `ended`), `supportReport`; typed problems (`people/not-found` 404, `people/conflict` 409, `people/invalid` 422, `people/sensitive` 403, `people/encryption-required` 409, `people/stored-data` 500, shared `invalid-request` 422, `too-large` 413, `unsupported` 501); binary with `--socket`, `--data`, `--actor` | done, draft |
| Live feed: `PeopleFeed`, `PersonListener`, `PersonChange`; cards only, never roles or links | done |
| `list`: kind, role and term combine with AND (`ListFilter`) | done |
| Writes through the traits take turns (`writes` in `PeopleRepo`), so a read-modify-write is not lost when the backend yields; the inherent methods do not, and compose each other | done |
| Limits: a card is at most `MAX_CARD_BYTES` as JSON (also after small edits), every stored text passes `check_intake`, every answer without a cursor `check_answer` | done |
| Role configuration survives a restart: `store_role_config` keeps it in the store, `restore_role_config` puts it back at start | done |
| Tests | 52 in the core, 30 for the server: over a real socket against the real store, one of them the real binary across a restart. Two small providers beside the real one, only for what it cannot show: the provider rule (a read-only source) and a feed whose reads fail |
| ADR 0043 refactor | done: every file <= 200 lines (`check_rust_rules.py`: 0 errors, 0 warnings), `//! Pattern:` in every module, `#![deny(missing_docs)]`, `#![warn(clippy::pedantic)]` clean |

## Structure and pattern map (ADR 0043)

Public contract: the trait `People`; `PeopleRepo` is its implementation (Repository + Facade) and keeps all
its inherent methods, so callers that do not import the trait keep working. Every file is one concept, <= 200 lines
(tests included); `lib.rs` and every `mod.rs` only declare and re-export.

| Module | Pattern | Why |
|---|---|---|
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
* **Delete.** The store has no transaction over several objects, so the card goes first: if that fails, nothing changed. If removing links and roles then fails, the error is `Error::PartlyDeleted`, which says the person is gone and what remains; they stay until the same call is made again, which finishes the job. **Known limit:** until then a link or role names a card that is gone, and a frontend whose list no longer shows the person cannot offer that second call.

## Migration for dependents

* `PeopleRepo::store()` is removed: the repository no longer holds the whole `Store`, only the narrow traits. Keep your
  own `Arc<dyn Store>` for undo / log / count calls (the old tests did `r.store().undo(..)`).
* `vcard::ImportedCard` moved to `format::ImportedCard` (re-exported at the crate root); `original` is now `Option<Blob>`.
* `Card`, `PeopleRepo::new/with_instance/create/get/update/delete/list/search/...`, `ListFilter`, `ImportReport`, `MatchKind`,
  `MatchReason`, `SameAsProposal`, `validate::*`, `vcard::*` keep their paths and signatures; fs-hub compiles unchanged.
* New: `People`, `Importer`, `Exporter`, `MatchStrategy`, `PeopleRepo::{with_backend, with_match_strategy, import, export}`.
  If you import `People` next to `PeopleRepo`'s inherent methods nothing is ambiguous (inherent methods win).

## Crates

* `fs-objects` (git tag v0.2.0). `fs-interfaces` (git tag v0.22.0): `Capabilities`, `Unsupported`; the server also takes the contract and `rpc` from it.
* `calcard =0.3.14` (Apache-2.0 OR MIT): vCard <-> JSContact following RFC 9554. **Pinned exactly**: Stalwart moved later
  versions to AGPL, which we must not use. Pulls `jmap-tools` (Apache-2.0 OR MIT), `chrono`, `mail-parser` etc., all permissive.
  Its JSON model is separate from ours: we exchange JSON text at the boundary and own the typed `Card`.
* `serde`, `serde_json`, `chrono` (timestamps), `thiserror`, `async-trait` (MIT OR Apache-2.0, already used by fs-objects); dev: `tokio`, `proptest`.
* All transitive licences are MIT / Apache-2.0 / BSD / ISC / Unicode-3.0 / Unlicense (a few have an LGPL *alternative*, never required).

## Decisions

* **Lossless model.** Known properties are typed; everything else (vendor properties, `cryptoKeys`, `calendars`, `personalInfo`,
  the RFC 9554 `vCard` member, ...) is kept verbatim in `extra`. Property order inside objects is not preserved (JSON objects are unordered).
* **Object id = uid** when the `uid` is a `urn:uuid:` UUIDv7 (the demo data are), so re-imports map onto the same object. Otherwise a fresh UUIDv7.
  vCards without `UID` get a deterministic `urn:uuid:` (version 8) derived from the text, so importing the same file twice does not duplicate.
* **`uid` is immutable**; `update` refuses a changed `uid`. `updated` is set to now on update.
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
* **Details of a role (provisional):** `assign` refuses `details` for a role not owned by `fs-people`: the owner of a role holds its fields (`people.capnp`). A non-empty `scope` must be an IRI, which also keeps it apart from the internal "personal".
* **`peopleWith` with an empty scope (provisional):** means every scope, as `PersonFilter` reads empty as no restriction, although `RoleAssignment.scope` reads empty as personal. The contract should say.
* **Sensitive cards in the binary:** the binary opens an unencrypted store, so `fs-objects` refuses them (`people/encryption-required`). The key provider comes with the key vault (ADR 0024).
* **Role definition format:** SKOS not read yet (strategy session); the JSON form may still get sharper before 1.0.
* **The trait returns `StoredObject` of `fs-objects`:** a provider without a store has to build one. To be decided before 1.0.
* **Gaps of `people.capnp` (for the owner of the contract):**
  * The root has no `capabilities` method (`tasks.capnp` has one), so `Capabilities::write_list` has no place; `supportReport` is the only report.
  * `RoleDefinition` has no place for the lossless `extra` of the core: a role read over the wire loses it. It stays in what the store and a configuration hold.
  * `confirmSame` is the only way to learn the id of a link, `linkedTo` answers card ids: after a restart a consumer cannot name the link it wants to `unlinkSame`.
  * Lists without a cursor (`findBy*`, `listMembers`, `listsOf`, `relations`, `proposals`, `Roles.list`, `rolesOf`, `peopleWith`): an answer that would not fit one message is refused as `too-large`; such a list cannot be read at all.
  * The ids `person.read`, `person.find` and the others are comments in the schema; the report follows them.
* **Whole import limit:** `importText` takes at most `MAX_INTAKE_BYTES` per call, also for many small cards. Split a larger file.
* **`Registry` of the feed** has the same shape as the one in `fs-tasks-core`; it belongs into a shared crate once a third program needs it (ADR 0043 §4).
* **Group cards vs `fs-groups`:** `create_group` / `members` stay as JSContact lists of people; belonging and rights go to `fs-groups` (ADR 0050).

## fs-objects 0.2 – what is used, what is still missing

Used: `order_by_index("name_sort")` (case-insensitive list order and paging), `Cond`/`Op::Contains` OR-group (search), `Cond::Describes`
and envelope `describes`, OR-group of two `Eq` conditions for the `Relationship` lookup (one query instead of two).

Still wanted:

1. **Tokenised full-text search / ranking** (substring only today).
2. The proposal finder still lists all non-group Profiles and groups by e-mail/phone in Rust; a "values shared by more than one object" query (`GROUP BY` on `index_fields`) would move that into the store.
3. Locale collation (CLDR) for ordering; the store orders by code point of the lowercased NFC key.

## Open points

* A failure between the steps of `delete` (links, assignments, card) leaves the earlier steps done; the store has no transaction across calls here.
* Sync token / CardDAV (RFC 6352) is not part of the core.
* Field-level `freesynergy.net:sensitive` is detected, but export/sharing filtering by field (ADR 0029 point 2) is a hub/frontend concern.
* JSContact `PatchObject` updates (RFC 9553) are not implemented; `update` replaces the whole card.
* The vCard `PHOTO`/`KEY` handling is whatever calcard does (not tested here).
* "Dismissed" proposals are not remembered; a person who rejects one will see it again.
