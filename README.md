# OxiiLink

A URL shortener and pastebin clone written in Rust using Axum,
currently hosted at [https://roman.vm.net.ua](https://roman.vm.net.ua)

# Features

- API and Web interface
- Syntax highlighting(pastes)
- Smart embed responses(data needed to generate an embed is only sent when an embed aware client is detected)(i.e an embed on a Discord message)
- automatic content type detection(only responds with HTML to HTML enabled clients, otherwise falls back to plaintext)
