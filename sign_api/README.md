# Rust Sign API SDK

This crate implements Sign API, described in:
https://specs.walletconnect.com/2.0/specs/clients/sign/

There is a simple Sign API client example built on top of the websocket client, which can be run as follows:
- In a browser, open: https://react-app.walletconnect.com/
- Click on "Goerli" and then "connect"
- In a new pop-up, click "New Pairing"
- Copy the Pairing URI
- In the terminal, cd _path/to/WalletConnectRust/sign_api_
- .../sign_api$ cargo run --example session "_copied URI_"
- DApp should now display the session window
- Click disconnect to terminate session and pairing

__Warning: this Rust Sign API SDK is community-maintained and may be lacking features and stability or security fixes that other versions of the Sign API SDK receive. We strongly recommend using the JavaScript or other Sign API SDKs instead.__

## Disclaimer

Please note that this crate is still under development, and thus:
- Is incomplete
- Might lack testing in places
- Being developed from a wallet perspective, and thus some DApp specific SDK details might have been overlooked
