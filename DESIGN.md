# fs-people – Design

`fs-people` is the people program of FreeSynergy. This repo holds the **program core** (`crates/fs-people-core`, no GUI), its **server** (`crates/fs-people-server`) and its frontend (`crates/fs-people-app`).
A person is a **JSContact `Card`** (RFC 9553) inside an AS2 **`Profile`** envelope, stored through `fs-objects`
(ADR 0014, 0019, 0020). Every program is at least a workbench (ADR 0037); the work surface is the workbench `people` in `fs-components`, which knows neither this core nor its types; `fs-people-app` connects the two.

## State

Version 0.5.0 (typed problem extensions, `SupportReport.instance`): provider of the `people` contract (`people.capnp` in `fs-interfaces` v0.43.0, draft, consumers: 0 of 2), served over a Unix socket. Built on fs-objects 0.4 (git tag). The report names the store (`StoreId` from `People::identity`, never in the log); problems carry typed extensions (section "Problem extensions").

`SupportReport.instance` is the identity of the provider's store (`fs-objects` `StoreId`). It is not a secret, but it makes the server recognisable: whoever can reach the socket sees it, and it does not go to the hub or into contents unless somebody decides so.

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
| Roles (ADR 0052): one card, several roles. A role file is a SKOS concept scheme in JSON-LD (`@context` with `@base`, `type: skos:ConceptScheme`, `names`, `roles[]`); the full id of a role is base + short id (ours: `https://freesynergy.net/ns/roles/`). Files lie side by side in `RoleRegistry` (one scheme per base, `load` replaces only the same base, checked first, swapped atomically, `store_role_config` keeps one object per base); your own roles are `define_role` objects. Found by full id, a short id means one of ours. `owner` is an id. Shipped: `ai`, `user`, `member`, `customer`, `supplier`, `lead`, `prospect` (en+de); `customer`/`supplier`/`lead`/`prospect` are `skos:Concept` without `cardKind`, so cards of kind `org` may hold them | done, provisional |
| Role assignments: AS2 `Relationship` objects of their own (index `person`, `role`, `scope`), the role stored as full id (an older short id is read as ours and answered as the full id; queries ask both forms), never on the card, so no export reveals them (ADR 0029) | done |
| Relations (what someone is to me): JSContact `relatedTo` on the card, `relate` / `unrelate` / `relations` | done |
| AI marking (ADR 0054): `mark_ai` = `kind: application` (RFC 9553 §2.1.4, travels with the card) + role `ai` (plain software is `application` too). Restricts nothing | done |
| Server `fs-people-server`: every method of `People`, `Persons`, `Roles`; `subscribe` (with `ended`), `supportReport`; typed problems (`people/not-found` 404, `people/conflict` 409, `people/invalid` 422, `people/sensitive` 403 (release missing), `people/sensitive-link` 403 (sensitive card is not linked), `people/encryption-required` 409, `people/stored-data` 500, shared `invalid-request` 422, `too-large` 413, `unsupported` 501); started as `fs-people serve [--socket] [--data] [--actor]` (ADR 0063: the server is a library, `fs_people_server::serve`) | done, draft |
| CLI `fs-people` (`crates/fs-people-cli`): `list [--term]`, `show`, `add`, `delete`, `find`, `export` (vCard), `import` (vCard) over the socket; the second user of `people.capnp` after the MCP tools; tested against a real server and once by hand with the real binaries | done, draft; no roles, relations or lists yet |
| Roles on the wire: ids leave as full IRIs, short ids are still accepted in requests; `RoleDefinition` carries `type` (empty = `skos:Concept`; anything but `org:Role` and `skos:Concept` is refused), `broader`, `closeMatch`, `standardRefs`. The core keeps one address for `broader` and `closeMatch`, so a second one in a request is refused (`invalid-request`), not dropped | done, draft |
| Live feed: `PeopleFeed`, `PersonListener`, `PersonChange`; cards only, never roles or links | done |
| `list`: kind, role and term combine with AND (`ListFilter`) | done |
| Writes through the traits take turns (`writes` in `PeopleRepo`), so a read-modify-write is not lost when the backend yields; the inherent methods do not, and compose each other | done |
| Limits: a card is at most `MAX_CARD_BYTES` as JSON (also after small edits), every stored text passes `check_intake`, every answer without a cursor `check_answer`; the report of an import keeps `created` whole and cuts `skipped` and `failed` to fit, with a last entry "N more" | done |
| A write of the server runs to its end, also if the client goes away (`run_to_end`) | done |
| Numbers: what plain `serde_json` cannot keep exactly is refused at intake with a clear 422 (an integer beyond `i64` / `u64`, a float whose value does not come back, `-0`): cards, `details`, a role configuration when stored (not when restored, so a stored one cannot stop a start); in an import, per item in `failed`, also one that no parser reads such as `1e400`. A `-0` an older card already holds blocks no edit. `validate/numbers.rs` reads the text, as the parser changes the number. `serde_json` has `float_roundtrip`, without which the parser can be one step off a double (6 % of random ones). The feature `arbitrary_precision` was tried and removed: features unify across the build, it broke vCard export and undo, and other readers could not read the result | done |
| Role configuration survives a restart: `store_role_config` keeps it in the store, `restore_role_config` puts it back at start | done |
| Compare-and-set: `People::update_checked(id, Revision, card)` and `Persons.updateIf`; every `Person` carries `revision` (derived from the stored time and card, not stored; compare for equality only). A stale revision is `people/conflict` (409) whose detail starts `conflict:` and names no revision; nothing is written. Atomic inside one process (the write turn) | done, draft |
| Tests | 142 in the core, 60 for the server (the app has its own: `crates/fs-people-app/tests`): over a real socket against the real store, one of them the real binary across a restart. Two small providers beside the real one, only for what it cannot show: the provider rule (a read-only source) and a feed whose reads fail |
| ADR 0043 refactor | done: no file over the hard limit of 300 lines, five test files over the guideline of 200 (`check_rust_rules.py`: 0 errors, 5 warnings: app tests keeps and people, core tests guards and role_forms, server test roles), `//! Pattern:` in every module, `#![deny(missing_docs)]`, `#![warn(clippy::pedantic)]` clean |

## One binary `fs-people` (ADR 0063)

`crates/fs-people` is the only binary. `fs-people` without arguments (or with window options such as `--store`, `--size`, `--snapshot`, `--import`, `--export`) opens the window; `fs-people serve` runs the server; `fs-people list|show|add|delete|find|export|import` are commands over the socket. `fs-people-server`, `fs-people-cli` and `fs-people-app` stay libraries. Feature `gui` (default) builds the window; `--no-default-features` gives a binary for machines without a screen. The binary tests (`crates/fs-people/tests/binary*.rs`) start `fs-people serve`.

## Frontend `fs-people-app` (first version, provisional)

Imports no `blinc_*` crate (ADR 0009). Arguments: `--store <file>` (default in memory), `--import <file.vcf>`, `--export <file>`, `--snapshot <png>`, `--size WxH`.

**In process, not RPC (provisional, do not forget):** `PeopleAdapter` holds `Arc<dyn People>` and `Arc<dyn RoleBook>` in the same process and drives them with a tokio handle, blocking the interface thread like `fs-wiki-app`. When the Cap'n Proto server of `people.capnp` exists, the adapter must call it instead. Until then the program opens the store itself, and the workbench's calls freeze the window for as long as the store takes.

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

## Structure and pattern map (ADR 0043)

Public contract: the trait `People`; `PeopleRepo` is its implementation (Repository + Facade) and keeps all
its inherent methods, so callers that do not import the trait keep working. Every file is one concept, <= 200 lines
(tests included); `lib.rs` and every `mod.rs` only declare and re-export.

| Module | Pattern | Why |
|---|---|---|
| `fs-people-server/{people_service,roles_service,persons/*}.rs` | Adapter + Factory | wire in, one trait call, wire out; depends on the traits only. `persons/service.rs` is the dispatch, `cards`, `links`, `lists` behind it |
| `fs-people-server/facts.rs` | none | a pure function: the facts of a core error as problem extensions, so a client reads `id`, `kind`, `issue` instead of cutting the detail text |
| `repo/backend.rs` | Adapter | `StoreAdapter` also gives `StoreInfo` (identity) to `PeopleRepo::identity` |
| `fs-people-core/subject.rs` | Value object | whom an error is about (`Kind`, id); `Error::subject` reads it off the variant |
| `fs-people-server/wire/*`, `errors.rs` | Adapter | values on the wire, limits of a message, core errors as RFC 9457 problems |
| `fs-people-server/formats.rs` | Registry | media type to format of the core; an unknown type is `Unsupported` |
| `fs-people-server/feed.rs`, `feed.rs` (core) | Observer | `PeopleFeed` to the shared forwarder; a read error ends the feed with `ended` |
| `repo/role_config.rs` | Memento | the last accepted file of each base address as one object in the store |
| `repo/role_facade.rs` | Facade | `RoleBook` and `PeopleFeed` over the inherent methods, writes take turns |
| `role_book.rs`, `repo/{roles,assign}.rs` | Repository (interface + implementation) | role definitions and assignments, split from `People` so a provider of cards alone still fits |
| `roles/directory.rs` | Strategy (trait `RoleDirectory`) | `PeopleRepo` holds an `Arc<dyn RoleDirectory>` (`with_roles`, default `RoleRegistry`); shipped, decorated and test sources are interchangeable, tests run against the trait |
| `roles/registry.rs` | Registry, swapped atomically | one scheme per base; configuration plugin after Zentinel (ADR 0045): checked first, readers see old or new, never a mix |
| `roles/scheme.rs` | Adapter | the JSON-LD role file into plain definitions with full ids; the old array form (short ids only) is read as a file of our base that merges over the shipped roles; `parse_stored` migrates an old sentence owner ("CRM (later)" -> `crm-later`), `parse` stays strict |
| `roles/ident.rs` | none, plain functions | short id, base, full id, `is_ai_role`: the one place that knows both forms (a newtype would wrap a string compared in one file) |
| `repo/holders.rs` | Repository (queries) | who holds a role, asking every stored form |
| `roles/{definition,assignment}.rs` | Value objects | the same form for shipped and own roles |
| `card/relations.rs`, `repo/relations.rs` | Command | small edits of `relatedTo` |
| `revision.rs`, `People::update_checked` | Newtype + Repository (optimistic lock) | `Revision` has `==` and nothing else, so no code can read it as a counter or an order (the contract says: compare for equality only); the trait method is the compare-and-set, the check and the write one step under the write turn. Derived from the stored time and card, not stored; the same for a sensitive card, and a refusal does not name the revision found (ADR 0029) |
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

## Problem extensions (v0.5.0)

The detail text is unchanged. Next to it the server states the facts as extensions (`fs-interfaces` v0.43.0, keys `id`, `kind`,
`issue`; the others belong to the type). A client checks `id` against what it asked for. `kind` is `person`, `list` or `role`.

| Problem type (`.../problems/people/...`) | Core error | Extensions |
|---|---|---|
| `not-found` | no card, no list, no role, or a stored object without a card | `id`, `kind` (`person`, `list`, `role`); a missing assignment or link: `id` only |
| `conflict` | `Changed` (updateIf, a card changed since it was read) | `id`, `kind` (`person`). Other conflicts (duplicate `uid`, role in use, ...) name no object: none |
| `invalid` | card against RFC 9553 | `issue` once per reason, as `path: reason` (`/version: ...`) |
| `invalid` | number that cannot be kept exactly (`the number ...`) | `value` (the number as written) |
| `invalid` | JSON the core cannot read, store refuses a value | `issue` (the reason without the `json: ` prefix) |
| `sensitive` | sensitive card, not released | `id`, `kind`, `who` (`uid: name`) |
| `sensitive-link` | sensitive card in a same-person link | `id`, `kind` (`person`) |
| `sensitive-members` | list export that would leave sensitive members behind | `id`, `kind` (`list`), `member` once per member held back (at most 63: `with_ext` keeps 64 extensions and 64 KiB; beyond that the rest is left out without a note, and a second export with a release reports the others) |
| `partly-deleted` | delete stopped halfway | `id`, `kind` (`person`), `issue` (where it stopped, without `; delete again to finish it`) |
| `stored-data` | something stored cannot be read | `issue` (the reason without `stored data: `) |
| `encryption-required`, `unsupported`, `too-large`, `invalid-request` | | none of ours |

So a client no longer needs `split_once(": ")`, `strip_prefix("stored data: ")`, `strip_suffix("; delete again to finish it")`, the `is sensitive and needs an explicit release` suffix, `card violates RFC 9553: `, `the number ` or `json: `.

## Migration for dependents

* `PeopleRepo::store()` is removed: the repository no longer holds the whole `Store`, only the narrow traits. Keep your
  own `Arc<dyn Store>` for undo / log / count calls (the old tests did `r.store().undo(..)`).
* `vcard::ImportedCard` moved to `format::ImportedCard` (re-exported at the crate root); `original` is now `Option<Blob>`.
* `Card`, `PeopleRepo::new/with_instance/create/get/update/delete/list/search/...`, `ListFilter`, `ImportReport`, `MatchKind`,
  `MatchReason`, `SameAsProposal`, `validate::*`, `vcard::*` keep their paths and signatures; fs-hub compiles unchanged.
* New: `People`, `Importer`, `Exporter`, `MatchStrategy`, `PeopleRepo::{with_backend, with_match_strategy, import, export}`.
  If you import `People` next to `PeopleRepo`'s inherent methods nothing is ambiguous (inherent methods win).

## Crates

* `fs-objects` (git tag v0.4.0). `fs-interfaces` (git tag v0.43.0): `Capabilities`, `Unsupported`; the server also takes the contract and `rpc` from it.
* `calcard =0.3.14` (Apache-2.0 OR MIT): vCard <-> JSContact following RFC 9554. **Pinned exactly**: Stalwart moved later
  versions to AGPL, which we must not use. Pulls `jmap-tools` (Apache-2.0 OR MIT), `chrono`, `mail-parser` etc., all permissive.
  Its JSON model is separate from ours: we exchange JSON text at the boundary and own the typed `Card`.
* `fs-components-app`, `fs-components-workbenches` (frontend only; git `rev`, provisional until a tag exists).
* `serde`, `serde_json`, `chrono` (timestamps), `thiserror`, `async-trait` (MIT OR Apache-2.0, already used by fs-objects); dev: `tokio`, `proptest`.
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

Unsupported in the role files (nothing pretends otherwise): export of an `org:Role` assignment as `org:Membership`; the context file `roles.jsonld` (the string in `@context` is not fetched, only `@base` is read); SHACL and value types of `fields`; `closeMatch` and `broader` are stored and returned, never followed or checked; hierarchy queries; friends, partners (relations) and clubs (research open); a lead's move to prospect or customer (end one assignment, start the next).

## Open points of the `people` interface

* **Operator of an AI (ADR 0054):** planned, the form is open. Nothing blocks it: `RoleAssignment.details` and the card's `extra` are lossless. Locking AIs out is deliberately not built.
* **Role privacy across programs:** assignments are `Relationship` objects with `to = [actor]`. A program with read access to the type `Relationship` could see them through the hub, whose rule goes by type. To be settled with `fs-groups` and the sharing interface (ADR 0029).
* **Role privacy on the server (provisional, stricter reading):** `Person.roles` holds "only what the caller may see", and the socket tells the user id, not the caller. So it is **always empty**; roles are asked for on `Roles`. Whoever reaches the socket reaches `Roles` as well, so `PersonFilter.role`, `rolesOf` and `peopleWith` answer what the contract names. When callers get an identity, all three need the same gate. A subscriber is told of cards only: an assignment or a link causes no change, and `confirmSame` hands out the id of its `Relationship`, nothing else.
* **Details of a role (provisional):** `assign` refuses `details` for a role not owned by `fs-people`: the owner of a role holds its fields (`people.capnp`). For a role already held, the same or no details answer the assignment, other details are a conflict: nothing is dropped and nothing replaced; end the assignment first.
* **Scope of an assignment (provisional):** a non-empty `scope` must be an IRI, which also keeps it apart from the internal "personal". `RoleDefinition.scope` decides: `group` needs one, `personal` has none, `public` can be either. `markAi` does not ask, an AI is one everywhere.
* **`peopleWith` with an empty scope (provisional):** means personal, as in `RoleAssignment.scope`; a non-IRI, also the literal `personal`, is refused. "Everywhere" has no query there: `PersonFilter.role` answers that, as a filter over people. If a consumer needs the ids across all scopes, the contract needs an explicit form.
* **Sensitive cards in the binary:** the binary opens an unencrypted store, so `fs-objects` refuses them (`people/encryption-required`). The key provider comes with the key vault (ADR 0024).
* **Role definition format (provisional, proposal of `docs/en/interfaces/roles.md`, not yet decided by the owner):** SKOS in JSON-LD as in the State table. Assumptions: the base `https://freesynergy.net/ns/roles/` (who controls the domain is not checked); `broader` and `closeMatch` are one address each; `ai` is `skos:Concept`, `user` and `member` `org:Role`; owners `fs-identity` (user), `fs-crm` (customer, lead, prospect), `fs-erp` (supplier) are ids taken from the documentation's "later CRM/ERP"; a short id given to `define_role` or in the old array means one of ours. Types of fields and SHACL are not read.
* **Registry rules:** a short id has no `:`, `/`, `#` and is not empty; a full id may exist once across all bases (`Conflict`); loads read, change and write under one write lock. `RoleDefinition.id` and `RoleAssignment.role` are always full ids.
* **AI recognition across forms:** `is_ai` and `mark_ai` accept the role stored as `ai` or as the full id. Consumers compare with `is_ai_role`, never with the literal `"ai"`.
* **`store_role_config` called directly and in parallel loses an update.** It is an inherent method of `PeopleRepo` and does not take the write turn (`writes`) that the `RoleBook` facade takes. Two concurrent direct calls can read the same stored state and the later write replaces the earlier one. Through the trait (`RoleBook::load_role_config`, so through the server) the turn serialises them, so it cannot happen there. Found in the review of the role work; not changed, as nothing outside the facade calls it.
* **Stored role files are never fatal, and not silent:** `restore_role_config` skips a stored file it cannot read and answers `RoleRestore` (`#[must_use]`, `skipped()` lists each file and why); `store_role_config` ignores such objects too. The core has no `tracing` dependency, so it logs nothing itself. The server prints `skipped()` at start on stderr, one line `fs-people: skipped stored role file: <file and why>` for each (`tests/binary_restore.rs`). Strict checks only for new input.
* **An array is resolved when saved:** the old array (short ids) adds to the roles in force; what is saved and loaded is the resulting file of our base, not the array, and holders are checked against exactly that set. So running and after a restart the same roles hold. A stored array from before is still merged over the shipped roles on restore.
* **Owner migration:** a sentence becomes an id ("CRM (later)" -> `crm-later`); if nothing readable is left ("?") it becomes `unknown`. Dropping the file would lose a whole configuration for one bad field, and `unknown` invents no program; it is not `fs-people`, so no details are kept for that role. Only reporting was the alternative; it would leave the roles out of force.
* **Friends and partners are relations (vCard `RELATED`), not roles; clubs need research:** not built.
* **The trait returns `StoredObject` of `fs-objects`:** a provider without a store has to build one. To be decided before 1.0.
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

## fs-objects 0.2 – what is used, what is still missing

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

## After the sprint (strategy list, 2026-11-07)

* A taken `uid` is `Error::Exists(id)` since 0.6.1 (in the pre-release line): it names the card that is there (`Error::subject`, and `id` and `kind` in the problem). `Error::is_conflict` is true for it and for `Conflict`. Other conflicts (a role in use, a card linked with itself) name no object and stay `Conflict(String)`. The id of an existing card is not its content; a caller who can create a card can already ask for it by uid.
* `broader`/`closeMatch` in role definitions are not normalised to full IRIs; a stored role file freezes the roles shipped with the version that wrote it.

## Merge lock for consumers of the AI role (strategy, 2026-11-07)

`roles-skos` is **already on `main`** (since v0.4.0): `RoleAssignment.role` and `RoleDefinition.id` are full IRIs (`https://freesynergy.net/ns/roles/ai`), short ids are still accepted in queries. Every consumer that decides "is this member an AI" (ADR 0054: consent, agents are never admin) must use `fs_people_core::is_ai_role()` and must never compare the short id `ai`. Read from the code on 2026-11-07, not tested end to end: `fs-groups` takes the decision through the injected trait `AiMarking` (no string compare of its own in `main`), `fs-messenger-guard` lists `ai` only among its test inputs for unknown values. Whether the host that implements `AiMarking` and the guard compare the short id is **not checked**: switch and check them in the next plan.

