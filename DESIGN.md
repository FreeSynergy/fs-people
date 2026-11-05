# fs-contacts – Design

`fs-contacts` is the contacts program of FreeSynergy. This repo holds the **program core** (`crates/fs-contacts-core`, no GUI).
A contact is a **JSContact `Card`** (RFC 9553) inside an AS2 **`Profile`** envelope, stored through `fs-store`
(ADR 0014, 0019, 0020). Every program is at least a workbench (ADR 0037); the GUI/workbench crates come later and use this core.

## State

Version 0.1.0, local only (no remote, never pushed).

| Area | State |
|---|---|
| Typed `Card` (serde) with lossless unknown/vendor properties (`extra` maps on the card and every sub-object) | done; all 20 demo cards round-trip byte-equal as JSON |
| Validation (RFC 9553: `@type`, `version` "1.0", `uid`, `kind` enum or `<domain>:<value>`, UTC date-times, required members of sub-objects, `members` only for groups) | done |
| Repository `ContactRepo` over `Arc<dyn Store>`: create / update / delete / get / get_by_uid / list / search | done |
| Envelope derivation: type `Profile`, `name` (ADR 0032: `name.full`, else components, nickname, org, e-mail, uid), `context=contacts`, IRI `https://<instance>/contacts/<id>`, `sensitive` from the card | done |
| Index fields: `uid`, `kind`, `name_sort`, `email` (lowercase), `phone` (E.164-ish), `org` (lowercase), `member`, `describes` | done |
| Search by name / nickname / e-mail / phone / organisation (substring, case-insensitive) | done, **scans** the Profile list in Rust (store has no FTS / LIKE yet) |
| vCard 4.0 import/export via `calcard` (also reads 2.1/3.0); original kept per card (`text/vcard`, ADR 0030); re-import skips known `uid`s | done |
| Groups / folders: `kind: group` + `members` (ADR 0037); add/remove member, resolve members, groups of a card | done |
| Same-person proposals (ADR 0019/0029): shared normalised e-mail or phone, same `kind`, never for sensitive cards, groups, or already linked pairs; confirm creates an AS2 `Relationship` (`owl:sameAs`); unlink; deleting a card removes its links | done |
| Sensitive marking: vendor property `freesynergy.net:sensitive` (DPV category list) on the card or any nested object -> `envelope.sensitive` | done; encryption itself is `fs-store`'s job (it refuses sensitive objects in an unencrypted store, tested) |
| Example `cargo run -p fs-contacts-core --example demo`: import demo contacts, list, search, export vCard | done |
| Tests | 15: unit (normalisation incl. a property test), integration over a real in-memory `TursoStore` (CRUD, search, vCard, groups, proposals, sensitive handling) |

## Crates

* `fs-store` (path dependency, not modified).
* `calcard =0.3.14` (Apache-2.0 OR MIT): vCard <-> JSContact following RFC 9554. **Pinned exactly**: Stalwart moved later
  versions to AGPL, which we must not use. Pulls `jmap-tools` (Apache-2.0 OR MIT), `chrono`, `mail-parser` etc., all permissive.
  Its JSON model is separate from ours: we exchange JSON text at the boundary and own the typed `Card`.
* `serde`, `serde_json`, `chrono` (timestamps), `thiserror`; dev: `tokio`, `proptest`.
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

## Needed from fs-store (not changed here)

1. **Case-insensitive ordering** (`Order::NameAsc` is bytewise: "NLnet" sorts before "Nextcloud"). The repo sorts by `name_sort` itself and pages in Rust.
2. **Substring / full-text search** over name and index fields. Today `search` loads all Profiles and filters in Rust; fine for address books of some thousands.
3. **Envelope `describes`** (AS2 `Profile.describes`). Today it is the vendor property `freesynergy.net:describes` in the card plus the index field `describes`.
4. **Index lookups with `OR` / prefix** (e.g. several e-mails in one query) would remove the per-key loops in the proposal finder.
5. Both `Relationship` queries need two index lookups (`subject`, `object`) because index keys are exact-match; a "any of these keys" filter would do it in one.

## Open points

* Sync token / CardDAV (RFC 6352) is not part of the core.
* Field-level `freesynergy.net:sensitive` is detected, but export/sharing filtering by field (ADR 0029 point 2) is a hub/frontend concern.
* JSContact `PatchObject` updates (RFC 9553) are not implemented; `update` replaces the whole card.
* The vCard `PHOTO`/`KEY` handling is whatever calcard does (not tested here).
* "Dismissed" proposals are not remembered; a person who rejects one will see it again.
