# OxiiLink

A URL shortener and pastebin clone written in Rust using Axum,
currently hosted at [https://oxlink.dev](https://oxlink.dev)

# Features

- API and Web interface
- embed image generation with syntax highlighting 
- Syntax highlighting(pastes)
- Smart embed responses(data needed to generate an embed is only sent when an embed aware client is detected)(i.e an embed on a Discord message)
- automatic content type detection(only responds with HTML to HTML enabled clients, otherwise falls back to plaintext)
