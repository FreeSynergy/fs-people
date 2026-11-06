# fs-people – Design

`fs-people` is the people program of FreeSynergy. This repo holds the **program core** (`crates/fs-people-core`, no GUI) and its frontend (`crates/fs-people-app`).
A person is a **JSContact `Card`** (RFC 9553) inside an AS2 **`Profile`** envelope, stored through `fs-store`
(ADR 0014, 0019, 0020). Every program is at least a workbench (ADR 0037); the work surface is the workbench `people` in `fs-components`, which knows neither this core nor its types; `fs-people-app` connects the two.

## State

Version 0.2.0: provider of the `people` contract (`people.capnp` in `fs-interfaces` v0.2.0, draft, consumers: 0 of 2). Built on fs-store 0.2 (git tag).

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
| Tests | 38 (17 new: roles, relations, AI marking, provider rule). Before them: 21 (3 unit + 18 integration in `tests/{model,crud,search,vcard,groups,proposals,interfaces}.rs`; the 18 pre-refactor tests are unchanged in behaviour, 3 new ones cover the trait seams): unit (normalisation incl. a property test), integration over a real in-memory `TursoStore` |
| ADR 0043 refactor | done: every file <= 200 lines (`check_rust_rules.py`: 0 errors, 0 warnings), `//! Pattern:` in every module, `#![deny(missing_docs)]`, `#![warn(clippy::pedantic)]` clean |

## Frontend `fs-people-app` (first version, provisional)

Imports no `blinc_*` crate (ADR 0009). Arguments: `--store <file>` (default in memory), `--import <file.vcf>`, `--export <file>`, `--snapshot <png>`, `--size WxH`.

**In process, not RPC (provisional, do not forget):** `PeopleAdapter` holds `Arc<dyn People>` and `Arc<dyn RoleBook>` in the same process and drives them with a tokio handle, blocking the interface thread like `fs-wiki-app`. When the Cap'n Proto server of `people.capnp` exists, the adapter must call it instead. Until then the program opens the store itself, and the workbench's calls freeze the window for as long as the store takes.

| Area | State |
|---|---|
| List (a page of 200 with a Show more button), search, read as sections, marks for an AI and for sensitive data (ADR 0029, 0054), roles read only | done |
| Create and edit name, e-mails, phones, organisation; delete with a second press | done |
| Editing keeps everything else: the form goes onto the card as stored now, and only fields that differ from what the form started with are written, each to the entry its key names. Proved in `tests/keeps.rs` by comparing the whole card as JSON | done |
| vCard import at start; a store refusing a sensitive card is shown as a problem in the status line and on stderr | done |
| vCard export of everyone with `--export`, then the program stops (with `--snapshot` it draws as well) | done |
| Windows | `launch` is wired, never opened by tests; `AppWidget::build` and `picture` are drawn offscreen and compared |

Decisions (provisional, easy to reverse):

* **A changed name replaces the name's parts** (`components`, `isOrdered`, `defaultSeparator`): left in place they would keep the old name in sorting, search and vCard `N`. `name.extra` stays, also a `sortAs` in it, which may then be out of date. An unchanged name stays as it is.
* **An emptied field removes that value;** an organisation with `units` only loses its name. A removed e-mail's id may be taken by one added in the same edit.
* **Conflict:** a revision is the store's `updated` plus a fingerprint of the content. Compare and write are not atomic, because `People::update` takes no revision: **wanted from the core: compare-and-set**. After a conflict the second Save writes only the fields the person changed, onto the other's version.
* **`--export` leaves out people marked sensitive and names them** (ADR 0029: they leave the device only by explicit release, and there is no way yet to give it). It never replaces a file, and it stops without a window, so it works in a script.
* **`--import` skips known `uid`s** (the core's rule); a file that cannot be read or holds no vCard ends the program with an error rather than opening without the people.
* The actor is a fixed `https://localhost/actors/owner`: there is no sign-in yet.

Unsupported in this version (nothing in the interface pretends otherwise): groups and their members (a group card is listed and read, `members` named as kept), relations (`relatedTo` kept, not shown), same-person proposals and links, assigning roles or marking an AI, editing anything else (nicknames, titles, addresses, links, notes, keywords), setting the sensitive marking, field-level release of sensitive data, a file dialog for import and export, undo, live updates (`person.subscribe`), a draft surviving a closed window, an encrypted store.

Wanted from the core:

* `People::search` has no paging: the whole result is read for every page of the list.
* `People::list` reports no total, so the list shows Show more instead of a scrollbar over everyone.
* `import` stops at the first store error and loses the report of the people before it (they are stored; the summary cannot count them).
* `export_vcard` keeps `VERSION:3.0` for a card imported from vCard 3.0 (seen in a real export), while its documentation says 4.0.

## Structure and pattern map (ADR 0043)

Public contract: the trait `People`; `PeopleRepo` is its implementation (Repository + Facade) and keeps all
its inherent methods, so callers that do not import the trait keep working. Every file is one concept, <= 200 lines
(tests included); `lib.rs` and every `mod.rs` only declare and re-export.

| Module | Pattern | Why |
|---|---|---|
| `role_book.rs`, `repo/{roles,assign}.rs` | Repository (interface + implementation) | role definitions and assignments, split from `People` so a provider of cards alone still fits |
| `roles/registry.rs` | Registry, swapped atomically | configuration plugin after Zentinel (ADR 0045): checked first, readers see old or new, never a mix |
| `roles/{definition,assignment}.rs` | Value objects | the same form for shipped and own roles |
| `card/relations.rs`, `repo/relations.rs` | Command | small edits of `relatedTo` |
| `people.rs` | Repository (interface) + Null Object | the program's public contract; GUI, hub and tests use `dyn People` |
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

## Migration for dependents

* `PeopleRepo::store()` is removed: the repository no longer holds the whole `Store`, only the narrow traits. Keep your
  own `Arc<dyn Store>` for undo / log / count calls (the old tests did `r.store().undo(..)`).
* `vcard::ImportedCard` moved to `format::ImportedCard` (re-exported at the crate root); `original` is now `Option<Blob>`.
* `Card`, `PeopleRepo::new/with_instance/create/get/update/delete/list/search/...`, `ListFilter`, `ImportReport`, `MatchKind`,
  `MatchReason`, `SameAsProposal`, `validate::*`, `vcard::*` keep their paths and signatures; fs-hub compiles unchanged.
* New: `People`, `Importer`, `Exporter`, `MatchStrategy`, `PeopleRepo::{with_backend, with_match_strategy, import, export}`.
  If you import `People` next to `PeopleRepo`'s inherent methods nothing is ambiguous (inherent methods win).

## Crates

* `fs-store` (git tag v0.2.0). `fs-interfaces` (git tag v0.2.0): `Capabilities`, `Unsupported`.
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
* **Sensitive (ADR 0029).** The card carries categories in `freesynergy.net:sensitive`; the repo copies them to `envelope.sensitive`.
  Sensitive cards are excluded from proposals and `confirm_same_person` refuses them even for hand-made pairs.
* **Link, don't merge.** The `Relationship` object has `subject`/`object` IRIs, `relationship: "owl:sameAs"` and index fields `subject`/`object`
  (object ids) for lookup. Both cards stay. `relatedTo` is not used.
* **Groups** are normal cards of `kind: group`; `members` maps member `uid` -> `true` (RFC 9553). Folder = group = mailing list (ADR 0037).
* **vCard export** strips nested `@type` members (no vCard meaning) and relies on calcard's RFC 9554 parameters (`PROP-ID`, `JSCOMPS`, `JSPROP`)
  for lossless return. Unknown vendor properties are exported as `JSPROP`.

## Open points of the `people` interface

* **Operator of an AI (ADR 0054):** planned, the form is open. Nothing blocks it: `RoleAssignment.details` and the card's `extra` are lossless. Locking AIs out is deliberately not built.
* **`person.subscribe`:** in the contract, not built; the provider reports it as not built yet. Comes with the first consumer that needs a live view.
* **Role privacy across programs:** assignments are `Relationship` objects with `to = [actor]`. A program with read access to the type `Relationship` could see them through the hub, whose rule goes by type. To be settled with `fs-groups` and the sharing interface (ADR 0029).
* **Role definition format:** SKOS not read yet (strategy session); the JSON form may still get sharper before 1.0.
* **Cap'n Proto server:** the wire form exists, the capnp-rpc server is not written yet (same as `fs-tasks`). The trait returns `StoredObject` of `fs-store`; a provider without a store has to build one. To be decided before 1.0.
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
