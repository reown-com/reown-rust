# WalletConnect Rust SDK

This is the foundation for the WalletConnect Rust SDK. Currently, there's only the core client and the RPC types required to communicate with the Relay.

Examples:
- [HTTP client](examples/http_client.rs)
- [WebSocket client](examples/websocket_client.rs)
- [Webhook dispatch](examples/webhook.rs)

## `relay_client`

The core Relay client. Provides access to all available Relay RPC methods to build on top of.

## `relay_rpc`

Provides all of the Relay domain types (e.g. `ClientId`, `ProjectId` etc.) as well as auth token generation and validation functionality.

# License

[Apache License (Version 2.0)](LICENSE)
