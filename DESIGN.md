# fs-contacts – Design

`fs-contacts` is the contacts program of FreeSynergy. This repo holds the **program core** (`crates/fs-people-core`, no GUI).
A contact is a **JSContact `Card`** (RFC 9553) inside an AS2 **`Profile`** envelope, stored through `fs-store`
(ADR 0014, 0019, 0020). Every program is at least a workbench (ADR 0037); the GUI/workbench crates come later and use this core.

## State

Version 0.1.0 (refactored to ADR 0043), local only (no remote, never pushed). Built on fs-store 0.2 (path dependency).

| Area | State |
|---|---|
| Typed `Card` (serde) with lossless unknown/vendor properties (`extra` maps on the card and every sub-object) | done; all 20 demo cards round-trip byte-equal as JSON |
| Validation (RFC 9553: `@type`, `version` "1.0", `uid`, `kind` enum or `<domain>:<value>`, UTC date-times, required members of sub-objects, `members` only for groups) | done |
| Contract `ContactBook` (trait, object-safe) + store-backed `ContactRepo` (create / update / delete / get / get_by_uid / list / search / groups / links / import / export) | done; `ContactRepo` depends on `dyn ObjectRepository` + `dyn ObjectQuery` only (narrow fs-store traits) |
| Envelope derivation: type `Profile`, `name` (ADR 0032: `name.full`, else components, nickname, org, e-mail, uid), `context=contacts`, IRI `https://<instance>/contacts/<id>`, `sensitive` from the card | done |
| Index fields: `uid`, `kind`, `name_sort`, `email` (lowercase), `phone` (E.164-ish), `nickname`, `org` (lowercase), `member` | done; `describes` moved to the envelope (fs-store 0.2) |
| Envelope `describes` (AS2 `Profile.describes`) from the vendor property `freesynergy.net:describes`; `find_by_describes` | done |
| Search by name / nickname / e-mail / phone / organisation (substring, case-insensitive) | done **in the store**: one query with an OR-group of `Op::Contains` (name, `name_sort`, nickname, e-mail, org, phone digits); Unicode case-insensitive, `%` `_` literal |
| Listing / ordering | `list` ordered by the `name_sort` index ("surname, given") case-insensitively, limit/offset in the store (no Rust sort) |
| Formats behind `Importer` / `Exporter` traits (`VCardFormat`, `JsContactFormat`); `ContactRepo::import(&dyn Importer, ..)` / `export(&dyn Exporter, ..)` | done |
| vCard 4.0 import/export via `calcard` (also reads 2.1/3.0); original kept per card (`text/vcard`, ADR 0030); re-import skips known `uid`s | done |
| Groups / folders: `kind: group` + `members` (ADR 0037); add/remove member, resolve members, groups of a card | done |
| Same-person proposals (ADR 0019/0029) as a `MatchStrategy` (default `SharedContactPoint`, swappable with `with_match_strategy`): shared normalised e-mail or phone, same `kind`, never for sensitive cards, groups, or already linked pairs; confirm creates an AS2 `Relationship` (`owl:sameAs`); unlink; deleting a card removes its links | done |
| Sensitive marking: vendor property `freesynergy.net:sensitive` (DPV category list) on the card or any nested object -> `envelope.sensitive` | done; encryption itself is `fs-store`'s job (it refuses sensitive objects in an unencrypted store, tested) |
| Example `cargo run -p fs-people-core --example demo`: import demo contacts, list, search, export vCard | done |
| Tests | 21 (3 unit + 18 integration in `tests/{model,crud,search,vcard,groups,proposals,interfaces}.rs`; the 18 pre-refactor tests are unchanged in behaviour, 3 new ones cover the trait seams): unit (normalisation incl. a property test), integration over a real in-memory `TursoStore` |
| ADR 0043 refactor | done: every file <= 200 lines (`check_rust_rules.py`: 0 errors, 0 warnings), `//! Pattern:` in every module, `#![deny(missing_docs)]`, `#![warn(clippy::pedantic)]` clean |

## Structure and pattern map (ADR 0043)

Public contract: the trait `ContactBook`; `ContactRepo` is its implementation (Repository + Facade) and keeps all
its inherent methods, so callers that do not import the trait keep working. Every file is one concept, <= 200 lines
(tests included); `lib.rs` and every `mod.rs` only declare and re-export.

| Module | Pattern | Why |
|---|---|---|
| `book.rs` | Repository (interface) | the program's public contract; GUI, hub and tests use `dyn ContactBook` |
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

* `ContactRepo::store()` is removed: the repository no longer holds the whole `Store`, only the narrow traits. Keep your
  own `Arc<dyn Store>` for undo / log / count calls (the old tests did `r.store().undo(..)`).
* `vcard::ImportedCard` moved to `format::ImportedCard` (re-exported at the crate root); `original` is now `Option<Blob>`.
* `Card`, `ContactRepo::new/with_instance/create/get/update/delete/list/search/...`, `ListFilter`, `ImportReport`, `MatchKind`,
  `MatchReason`, `SameAsProposal`, `validate::*`, `vcard::*` keep their paths and signatures; fs-hub compiles unchanged.
* New: `ContactBook`, `Importer`, `Exporter`, `MatchStrategy`, `ContactRepo::{with_backend, with_match_strategy, import, export}`.
  If you import `ContactBook` next to `ContactRepo`'s inherent methods nothing is ambiguous (inherent methods win).

## Crates

* `fs-store` (path dependency, not modified).
* `calcard =0.3.14` (Apache-2.0 OR MIT): vCard <-> JSContact following RFC 9554. **Pinned exactly**: Stalwart moved later
  versions to AGPL, which we must not use. Pulls `jmap-tools` (Apache-2.0 OR MIT), `chrono`, `mail-parser` etc., all permissive.
  Its JSON model is separate from ours: we exchange JSON text at the boundary and own the typed `Card`.
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
