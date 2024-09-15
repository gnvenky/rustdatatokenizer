# rustdatatokenizer

A Rust program that uses a crypto hash function (SHA256) to store tokens of sensitive data.
Crrently the tokens are persisted through SLED functions on Disk

This can be called in Oracle through a C-wrapper over Rust (or) python wrapper on SQL Server.
Alternatively the Rust program may be a web service which passes back the token and this
Web service may be securely called from Stored-procedures over HTTPS/TLS

