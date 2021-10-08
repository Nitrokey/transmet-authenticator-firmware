# Apdu Dispatch

This layer accepts APDU (application packet data units) from a contact and/or contactless interface and passes them to a selected app.

It handles parsing APDU's, chaining, T=0, and T=1.

Run tests via `cargo test --features std,log-all`
