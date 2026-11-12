# Changelog

## 0.0.14 (in progress)

### English

- **Blinc only through fs-components.** The program has no Blinc dependency and no `[patch]` any more; `Cargo.lock` carries one Blinc source (Blinc fs-blinc-3.1, through fs-components v0.0.11). German keyboards can type `@`, `+`, `/` and AltGr signs (e-mail addresses) again.
- **Version handshake.** The window asks the server for name and version (`Control.version`): a server of another version is replaced, none running is started. A server of another program on the socket (a wrong `--socket`) is left alone and named. When no fitting server can be reached the window still opens and says why; the next call connects again, also after a server that went away in the middle of a session.
- **Time limits.** No call to the server waits for ever: 10 s for the first questions, 30 s for every other call.
- New: `--facet people` (the only facet) and `fs-people --version`.
- Built on fs-components v0.0.11 and fs-interfaces v0.0.19.
- For the typing test only: the optional feature `test-access` (debug builds; fs-components refuses it in a release build) lets the test read what is typed into the fields.

### Deutsch

- **Blinc nur über fs-components.** Das Programm hat keine Blinc-Abhängigkeit und kein `[patch]` mehr; `Cargo.lock` trägt eine Blinc-Quelle (Blinc fs-blinc-3.1, über fs-components v0.0.11). Auf deutschen Tastaturen lassen sich `@`, `+`, `/` und AltGr-Zeichen (E-Mail-Adressen) wieder tippen.
- **Versions-Handshake.** Das Fenster fragt den Server nach Name und Version (`Control.version`): ein Server anderer Version wird ersetzt, läuft keiner, wird einer gestartet. Ein Server eines anderen Programms am Socket (falsches `--socket`) bleibt unberührt und wird genannt. Ist kein passender Server erreichbar, öffnet das Fenster trotzdem und sagt warum; der nächste Aufruf verbindet neu, auch nach einem Server, der mitten in der Sitzung verschwunden ist.
- **Zeitlimits.** Kein Aufruf des Servers wartet für immer: 10 s für die ersten Fragen, 30 s für jeden weiteren Aufruf.
- Neu: `--facet people` (die einzige Facette) und `fs-people --version`.
- Gebaut auf fs-components v0.0.11 und fs-interfaces v0.0.19.
- Nur für den Tipptest: das optionale Feature `test-access` (Debug-Builds; fs-components verweigert es in einem Release-Build) lässt den Test lesen, was in die Felder getippt wird.
