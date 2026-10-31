# Portable-font regression fixture

`Lato-Regular.ttf` is the original, unmodified Google Fonts static face, downloaded
2026-10-30. Source pinned at google/fonts commit
`f3b885d5590e307e02542f1a724cec55d567fdaa`:
https://github.com/google/fonts/blob/f3b885d5590e307e02542f1a724cec55d567fdaa/ofl/lato/Lato-Regular.ttf

License: SIL Open Font License 1.1, included as `Lato-OFL.txt` from the same commit.
Tests intentionally declare Lato and the existing Noto Sans face under the same
authored family, weight and style to exercise content identity isolation. They
retain both original binaries and internal names. This is test data, never an
automatic project import or an OS fallback.

SHA-256: `d636e4683231f931eda222d588e944d082bfd3bdba02f928bee461c0f185b251`

`Lato-Bold.ttf` and `Lato-Italic.ttf` are unmodified static faces from the same
pinned commit and `ofl/lato/` directory, with the same bundled OFL license. They
exercise genuine 700 normal and 400 italic font outlines, rather than registering
the regular bytes under invented face metadata. SHA-256 hashes:

- Bold: `8a0aace75d33794eece4b28187bfc1df0bbd2888b5d8a56e01788c8d65d16be1`
- Italic: `e399c44efe1387100531d26c7e4800c5d12251b890d6654a3098c7c679cb1786`
