# FreeSynergy People — know who someone is

**A person is more than a row in an address book, and your contacts should be yours to keep and carry.**

FreeSynergy People holds the people in your life: friends, family, colleagues, customers, groups of them. Each person is one card. A card can carry several roles at once, so the same human can be a colleague and a neighbour without you copying anything.

What is different here: a card follows an open standard, JSContact (RFC 9553), and anything we do not understand is kept, not thrown away. You can bring your contacts in as vCard and take them out again. Cards you mark as sensitive stay on your device unless you release them one by one. Roles are kept apart from the card, so an export does not reveal them. If a participant is an AI, the card says so; the mark restricts nothing, it only tells you who you are dealing with.

People works alone. When the other FreeSynergy programs run next to it, they can refer to a person by id instead of keeping their own copy.

## What it can do today

Latest tag: `v0.6.0`.

- Keep contacts as JSContact cards; unknown or vendor properties survive unchanged.
- Create, edit, delete, list and search (name, nickname, e-mail, phone, organisation).
- Import and export vCard (also reads versions 2.1 and 3.0). The original text is kept per card.
- Groups of people, and several roles per person (shipped: ai, user, member, customer, supplier, lead, prospect). You can define your own.
- Relations ("what someone is to me") on the card.
- Propose that two cards are the same person (shared e-mail or phone). Nothing is merged until you confirm.
- Mark a card as sensitive. Marked cards are left out of exports unless released by name.
- A small server that offers all of this over a local socket (the `people` contract).
- A first, provisional window: list, search, read, create, edit, delete, vCard import and export.
- Tested against the real store: 142 tests in the core, 60 for the server, more for the window.

## What is not there yet

- The window is a first version. It cannot yet manage groups, relations or roles, set the sensitive mark, undo, or update live. A closed window loses an unsaved draft.
- The window reads the data directly instead of talking to the server (a version over the server is on branch `gui`), so it freezes while the store works.
- No sign-in. The acting person is a fixed placeholder.
- No CardDAV and no sync between devices.
- The interface is a draft and has no second user yet, so it may still change.
- A person who rejects a "same person" proposal will see it again.

## Part of FreeSynergy

People is the program that answers "who is this?" for all the others. FreeSynergy is a family of programs that are free for everyone, forever, and that keep your data yours. The documentation, the manifesto and the overall plan live in [fs-documentation](https://github.com/FreeSynergy/fs-documentation); the reading version is at <https://freesynergy.github.io/fs-documentation/>. Start with the [manifesto](https://freesynergy.github.io/fs-documentation/manifesto/) if you want to know why we do this.


## For developers

- Workspace crates: `fs-people-core` (the program core, no GUI), `fs-people-server` (the contract over a Unix socket), `fs-people-app` (the provisional window, on `fs-components`).
- The contract is `people.capnp` in [fs-interfaces](https://github.com/FreeSynergy/fs-interfaces) (draft). People owns it.
- Build and test: `cargo build --workspace` and `cargo test --workspace`. Try the core: `cargo run -p fs-people-core --example demo`.
- Design, decisions and the full state table: [DESIGN.md](DESIGN.md).
- Licence: MIT OR Apache-2.0.
