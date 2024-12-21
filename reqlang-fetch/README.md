# reqlang-fetch

A crate that exposes the `Fetch` trait.

## Usage

```rust
use reqlang::prelude::*;
use reqlang_fetch::{Fetch, HttpRequestFetcher};

async fn main() {
    let request = HttpRequest {
        method: HttpVerb::get(),
        url: Url::parse("https://example.com").unwrap(),
        headers: vec![],
        body: None,
    };

    let fetcher: HttpRequestFetcher = request.into();

    let response: HttpResponse = fetcher.fetch().await.unwrap();
}
```
